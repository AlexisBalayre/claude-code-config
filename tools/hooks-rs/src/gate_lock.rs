// kill, signal and the handler's cleanup have no safe std form.
#![allow(unsafe_code)]
use crate::common::git;
use std::ffi::CString;
use std::fs;
use std::io::ErrorKind;
use std::os::unix::process::{CommandExt, ExitStatusExt};
use std::process::{exit, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

struct Paths {
    entry: CString,
    pid: CString,
    label: CString,
    lock: CString,
    me: String,
}

// The signal handler reads these, so they are built once, before it is installed.
static PATHS: OnceLock<Paths> = OnceLock::new();
static HOLDING: AtomicBool = AtomicBool::new(false);

fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64)
}

fn mtime(path: &str) -> i64 {
    fs::metadata(path)
        .and_then(|meta| meta.modified())
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map_or_else(now, |d| d.as_secs() as i64)
}

fn read(path: &str) -> Option<String> {
    Some(
        fs::read_to_string(path)
            .ok()?
            .trim_end_matches('\n')
            .to_owned(),
    )
}

/// 0 for a pid that can be signalled, else the errno of `kill -0`.
fn kill0(pid: &str) -> i32 {
    match pid.parse::<i32>() {
        Ok(pid) if pid > 0 && unsafe { libc::kill(pid, 0) } != 0 => std::io::Error::last_os_error()
            .raw_os_error()
            .unwrap_or(libc::ESRCH),
        Ok(pid) if pid > 0 => 0,
        _ => libc::ESRCH,
    }
}

// A sandboxed waiter may not signal a holder outside its own sandbox, so kill fails with EPERM
// on a live pid. A dead holder's pid reused by a root or other-user process fails the same way,
// so EPERM counts as alive only for 2 h, well past a verify run on a loaded machine.
fn alive(pid: &str, pid_file: &str) -> bool {
    match kill0(pid) {
        0 => true,
        libc::EPERM => now() - mtime(pid_file) < 7200,
        _ => false,
    }
}

/// Counts the live waiters queued before `me`, removing the dead ones' entries. A waiter touches
/// its entry on every poll, so a stale entry is dead whatever kill says.
fn count_ahead(queue: &str, me: &str) -> usize {
    let number = |s: Option<&str>| s.and_then(|s| s.parse::<u64>().ok()).unwrap_or(0);
    let mut entries: Vec<String> = fs::read_dir(queue)
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    entries.sort_by_cached_key(|name| {
        let mut parts = name.splitn(2, '-');
        (number(parts.next()), number(parts.next()), name.clone())
    });
    let mut ahead = 0;
    for name in entries.iter().take_while(|name| *name != me) {
        let path = format!("{queue}/{name}");
        let pid = name.split_once('-').map_or(name.as_str(), |(_, pid)| pid);
        if now() - mtime(&path) <= 60 && matches!(kill0(pid), 0 | libc::EPERM) {
            ahead += 1;
        } else {
            let _ = fs::remove_file(&path);
        }
    }
    ahead
}

// Runs inside the signal handler too, hence libc calls on prebuilt strings and no allocation.
// Releases only a lock this process still holds: two waiters taking over one dead holder can
// race, and the loser must not delete the winner's lock on its way out.
fn cleanup() {
    let Some(paths) = PATHS.get() else { return };
    unsafe {
        libc::unlink(paths.entry.as_ptr());
        if !HOLDING.load(Ordering::SeqCst) {
            return;
        }
        let mut buf = [0u8; 32];
        let fd = libc::open(paths.pid.as_ptr(), libc::O_RDONLY);
        if fd < 0 {
            return;
        }
        let n = libc::read(fd, buf.as_mut_ptr().cast(), buf.len());
        libc::close(fd);
        let holder = &buf[..n.max(0) as usize];
        if holder.strip_suffix(b"\n").unwrap_or(holder) == paths.me.as_bytes() {
            libc::unlink(paths.pid.as_ptr());
            libc::unlink(paths.label.as_ptr());
            libc::rmdir(paths.lock.as_ptr());
        }
    }
}

extern "C" fn on_signal(sig: libc::c_int) {
    cleanup();
    unsafe {
        libc::signal(sig, libc::SIG_DFL);
        libc::raise(sig);
    }
}

pub fn run(args: &[String]) {
    let common_dir = Command::new("git")
        .args(["rev-parse", "--path-format=absolute", "--git-common-dir"])
        .stderr(Stdio::inherit())
        .output()
        .map(|out| {
            String::from_utf8_lossy(&out.stdout)
                .trim_end_matches('\n')
                .to_owned()
        })
        .unwrap_or_default();
    let lock = format!("{common_dir}/gate-lock");
    let (pid_file, label_file) = (format!("{lock}/pid"), format!("{lock}/label"));
    let c = |s: &str| CString::new(s).unwrap_or_default();
    // Every mkdir failure below reads as contention, so a lock that can never be made (git older
    // than 2.31 prints --path-format back, an unwritable .git) would wait forever.
    if unsafe { libc::access(c(&common_dir).as_ptr(), libc::W_OK) } != 0 {
        eprintln!("gate-lock: cannot create {lock}");
        exit(1);
    }
    // A detached HEAD has no branch: name it by its short sha.
    let label = std::env::var("GATE_LOCK_LABEL")
        .ok()
        .filter(|label| !label.is_empty())
        .or_else(|| git(".", &["branch", "--show-current"]).filter(|branch| !branch.is_empty()))
        .or_else(|| git(".", &["rev-parse", "--short", "HEAD"]))
        .unwrap_or_default();

    // A command run under the lock (verify inside a held lock, any nested gate-lock call) passes
    // straight through. A leaked GATE_LOCK_HELD only counts while its pid is alive and is the
    // lock's holder, so it cannot skip the queue.
    let held = std::env::var("GATE_LOCK_HELD").unwrap_or_default();
    if !held.is_empty()
        && !args.is_empty()
        && read(&pid_file).as_deref() == Some(&held)
        && alive(&held, &pid_file)
    {
        let err = Command::new(&args[0]).args(&args[1..]).exec();
        eprintln!("gate-lock: exec: {}: {err}", args[0]);
        exit(1);
    }

    let queue = format!("{lock}.queue");
    if let Err(err) = fs::create_dir_all(&queue) {
        eprintln!("gate-lock: {queue}: {err}");
        exit(1);
    }
    let me = std::process::id().to_string();
    // Seconds, as the shell version queues: arrivals within one second queue by pid.
    let entry_name = format!("{}-{me}", now());
    let entry = format!("{queue}/{entry_name}");
    let _ = PATHS.set(Paths {
        entry: c(&entry),
        pid: c(&pid_file),
        label: c(&label_file),
        lock: c(&lock),
        me: me.clone(),
    });
    for sig in [libc::SIGTERM, libc::SIGINT, libc::SIGHUP] {
        // A signal the caller ignores stays ignored, as a shell trap leaves it.
        unsafe {
            if libc::signal(
                sig,
                on_signal as extern "C" fn(libc::c_int) as libc::sighandler_t,
            ) == libc::SIG_IGN
            {
                libc::signal(sig, libc::SIG_IGN);
            }
        }
    }

    let mut last_notice = 0;
    loop {
        let _ = fs::File::options()
            .create(true)
            .truncate(false)
            .write(true)
            .open(&entry)
            .and_then(|file| file.set_modified(SystemTime::now()));
        let ahead = count_ahead(&queue, &entry_name);
        // The pid write is part of the acquire: it fails when a waiter cleared the fresh lock as
        // empty. The label goes in first, so a waiter that reads the pid finds the label too.
        if ahead == 0
            && fs::create_dir(&lock).is_ok()
            && fs::write(&label_file, format!("{label}\n")).is_ok()
            && fs::write(&pid_file, format!("{me}\n")).is_ok()
        {
            break;
        }
        let holder = read(&pid_file).unwrap_or_default();
        // Only the head clears a lock, so no waiter behind it can clear the lock the head just
        // took. Re-read before clearing: the holder may have released and another run taken it.
        if ahead == 0
            && !holder.is_empty()
            && !alive(&holder, &pid_file)
            && read(&pid_file).as_deref() == Some(&holder)
        {
            let label = read(&label_file).unwrap_or_default();
            eprintln!("gate-lock: taking over the stale lock of pid {holder} ({label})");
            let _ = fs::remove_file(&pid_file);
            let _ = fs::remove_file(&label_file);
            let _ = fs::remove_dir(&lock);
            continue;
        }
        let now = now();
        // No pid for over a minute: a holder killed between mkdir and the pid write, or between
        // the release's unlink and rmdir.
        if ahead == 0 && holder.is_empty() && now - mtime(&lock) > 60 {
            eprintln!("gate-lock: clearing an empty lock left by a killed holder");
            let _ = fs::remove_file(&label_file);
            let _ = fs::remove_dir(&lock);
            continue;
        }
        if !holder.is_empty() && now - last_notice >= 60 {
            let since = mtime(&pid_file);
            let name = read(&label_file).unwrap_or_else(|| format!("pid {holder}"));
            // A holder that released since its pid was read took its label and pid file with it,
            // which would print its pid and a negative hold: retry the acquire instead.
            if read(&pid_file).as_deref() != Some(&holder) {
                continue;
            }
            let held = now - since;
            eprintln!(
                "gate-lock: waiting for {name} (held {}m{}s, queue position {})",
                held / 60,
                held % 60,
                ahead + 1
            );
            // Past a normal run's length the holder may be hung, or a pid reused by a live process.
            if held > 1800 {
                let user = Command::new("ps")
                    .args(["-o", "user=", "-p", &holder])
                    .output()
                    .ok()
                    .filter(|out| out.status.success())
                    .map_or("unknown".into(), |out| {
                        String::from_utf8_lossy(&out.stdout).trim_end().to_owned()
                    });
                eprintln!("gate-lock: pid {holder} (user {user}) holds the lock; if it runs no gate, rm -rf {lock}");
            }
            last_notice = now;
        }
        std::thread::sleep(Duration::from_secs(2));
    }
    HOLDING.store(true, Ordering::SeqCst);
    let _ = fs::remove_file(&entry);

    let code = match args.split_first() {
        None => 0,
        Some((command, rest)) => match Command::new(command)
            .args(rest)
            .env("GATE_LOCK_HELD", &me)
            .status()
        {
            Ok(status) => status
                .code()
                .unwrap_or_else(|| 128 + status.signal().unwrap_or(0)),
            Err(err) if err.kind() == ErrorKind::NotFound => {
                eprintln!("gate-lock: {command}: command not found");
                127
            }
            Err(err) => {
                eprintln!("gate-lock: {command}: {err}");
                126
            }
        },
    };
    cleanup();
    exit(code);
}
