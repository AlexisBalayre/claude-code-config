mod common;

use common::{git, hook, TempDir};
use std::fs::{self, File};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread::sleep;
use std::time::{Duration, Instant, SystemTime};

/// A background process, killed on drop so a failed assertion leaves none behind.
struct Bg(Child);

impl Bg {
    fn pid(&self) -> String {
        self.0.id().to_string()
    }

    fn alive(&mut self) -> bool {
        matches!(self.0.try_wait(), Ok(None))
    }

    /// The exit code. A call still running after 30 s counts as hung.
    fn finish(&mut self) -> i32 {
        let start = Instant::now();
        loop {
            if let Some(status) = self.0.try_wait().expect("wait for the process") {
                return status.code().unwrap_or(-1);
            }
            assert!(start.elapsed() < Duration::from_secs(30), "the call hung");
            sleep(Duration::from_millis(50));
        }
    }

    fn terminate(&mut self) {
        Command::new("kill")
            .arg(self.pid())
            .status()
            .expect("run kill");
        self.finish();
    }
}

impl Drop for Bg {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn start(command: &mut Command, stderr: Stdio) -> Bg {
    let child = command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(stderr)
        .spawn();
    Bg(child.expect("spawn the process"))
}

fn sleeper() -> Bg {
    start(Command::new("sleep").arg("30"), Stdio::null())
}

fn dead_pid() -> String {
    let mut gone = start(&mut Command::new("true"), Stdio::null());
    gone.finish();
    gone.pid()
}

// Polls for 20 s: a waiter's git calls and 2 s polls take an unknown time on a loaded host.
fn wait_for(mut done: impl FnMut() -> bool) -> bool {
    for _ in 0..200 {
        if done() {
            return true;
        }
        sleep(Duration::from_millis(100));
    }
    false
}

fn backdate(path: &Path, secs: u64) {
    let then = SystemTime::now() - Duration::from_secs(secs);
    File::open(path)
        .and_then(|file| file.set_modified(then))
        .expect("backdate the path");
}

fn age(path: &Path) -> Option<Duration> {
    fs::metadata(path).ok()?.modified().ok()?.elapsed().ok()
}

struct Repo(TempDir);

impl Repo {
    fn new(test: &str) -> Repo {
        let repo = Repo(TempDir::new(test));
        git(repo.0.path(), &["init", "-q", "-b", "feature/x", "w"]);
        git(&repo.dir(), &["commit", "-q", "--allow-empty", "-m", "a"]);
        repo
    }

    fn dir(&self) -> PathBuf {
        self.0.path().join("w")
    }

    fn lock(&self) -> PathBuf {
        self.dir().join(".git/gate-lock")
    }

    fn queue(&self) -> PathBuf {
        self.dir().join(".git/gate-lock.queue")
    }

    fn scratch(&self, name: &str) -> String {
        self.0.path().join(name).display().to_string()
    }

    fn read(&self, name: &str) -> String {
        fs::read_to_string(self.scratch(name)).unwrap_or_default()
    }

    fn gl(&self, args: &[&str]) -> Command {
        let mut command = hook("gate-lock");
        command.current_dir(self.dir()).args(args);
        command
    }

    /// Start a gate-lock call whose stderr lands in the `err` scratch file.
    fn waiter(&self, command: &mut Command) -> Bg {
        let err = File::create(self.scratch("err")).expect("create the err file");
        start(command, err.into())
    }

    /// Start a call that queues under `label` and logs it once it holds the lock.
    fn wait_as(&self, label: &str) -> Bg {
        let script = format!("echo {label} >>'{}'", self.scratch("log"));
        start(
            self.gl(&["sh", "-c", &script])
                .env("GATE_LOCK_LABEL", label),
            Stdio::null(),
        )
    }

    /// Start a holder running `script` under `label` and wait until it holds the lock.
    fn holder(&self, label: &str, script: &str) -> Bg {
        let holder = start(
            self.gl(&["bash", "-c", script])
                .env("GATE_LOCK_LABEL", label),
            Stdio::null(),
        );
        let pid = self.lock().join("pid");
        assert!(wait_for(
            || fs::metadata(&pid).is_ok_and(|meta| meta.len() > 0)
        ));
        holder
    }

    /// A script that ends once the `err` file shows `text`, or after 20 s against a hang.
    fn until_err(&self, text: &str) -> String {
        let err = self.scratch("err");
        format!("for _ in $(seq 200); do grep -qs '{text}' '{err}' && break; sleep 0.1; done")
    }

    fn until_go(&self) -> String {
        format!(
            "for _ in $(seq 200); do [ -e '{}' ] && break; sleep 0.1; done",
            self.scratch("go")
        )
    }

    fn go(&self) {
        fs::write(self.scratch("go"), "").expect("write the go file");
    }

    fn fake_lock(&self, pid: &str) {
        fs::create_dir(self.lock()).expect("create the lock");
        fs::write(self.lock().join("pid"), format!("{pid}\n")).expect("write the pid");
    }

    fn fake_entry(&self, name: &str) -> PathBuf {
        fs::create_dir_all(self.queue()).expect("create the queue");
        let entry = self.queue().join(name);
        fs::write(&entry, "").expect("write the queue entry");
        entry
    }

    fn entries(&self) -> Vec<String> {
        fs::read_dir(self.queue())
            .into_iter()
            .flatten()
            .flatten()
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .collect()
    }

    fn polled(&self, waiter: &Bg) {
        let suffix = format!("-{}", waiter.pid());
        assert!(wait_for(|| self
            .entries()
            .iter()
            .any(|entry| entry.ends_with(&suffix))));
    }

    fn holder_pid(&self) -> String {
        fs::read_to_string(self.lock().join("pid"))
            .unwrap_or_default()
            .trim()
            .to_owned()
    }
}

#[test]
fn serialises_two_holders_and_names_the_holder_to_the_waiter() {
    let repo = Repo::new("serialise");
    let log = repo.scratch("log");
    let first = format!(
        "echo s1 >>'{log}'; {}; echo e1 >>'{log}'",
        repo.until_err("gate-lock: waiting")
    );
    let mut holder = repo.holder("first", &first);
    let second = format!("echo s2 >>'{log}'; echo e2 >>'{log}'");
    assert_eq!(
        repo.waiter(&mut repo.gl(&["bash", "-c", &second])).finish(),
        0
    );
    holder.finish();
    assert_eq!(repo.read("log"), "s1\ne1\ns2\ne2\n");
    let err = repo.read("err");
    assert!(
        err.contains("gate-lock: waiting for first (held 0m"),
        "{err}"
    );
    assert!(err.contains("queue position 1)"), "{err}");
    assert!(!err.contains("rm -rf"), "{err}");
}

// The holder lets go just after A's poll, and B and C poll at other points of the 2 s cycle, so
// they poll first: only the queue keeps the arrival order. Each starts over 1 s after the last
// one's first poll, so in a later second.
#[test]
fn gives_the_lock_in_arrival_order() {
    let repo = Repo::new("order");
    let (mark, a_pid) = (repo.scratch("mark"), repo.scratch("a"));
    let script = format!(
        "{}; touch '{mark}'; for _ in $(seq 200); do \
         [ -n \"$(find '{}' -name \"*-$(cat '{a_pid}')\" -newer '{mark}')\" ] && break; \
         sleep 0.05; done",
        repo.until_go(),
        repo.queue().display()
    );
    let mut holder = repo.holder("holder", &script);
    let mut waiters = Vec::new();
    for label in ["A", "B", "C"] {
        let waiter = repo.wait_as(label);
        if label == "A" {
            fs::write(&a_pid, waiter.pid()).expect("write A's pid");
        }
        repo.polled(&waiter);
        sleep(Duration::from_millis(1100));
        waiters.push(waiter);
    }
    repo.go();
    holder.finish();
    for waiter in &mut waiters {
        assert_eq!(waiter.finish(), 0);
    }
    assert_eq!(repo.read("log"), "A\nB\nC\n");
}

#[test]
fn skips_a_waiter_killed_while_queued_and_drops_its_entry() {
    let repo = Repo::new("killed-waiter");
    let mut holder = repo.holder("holder", &repo.until_go());
    let a = repo.wait_as("A");
    repo.polled(&a);
    sleep(Duration::from_millis(1100));
    let mut b = repo.wait_as("B");
    repo.polled(&b);
    drop(a);
    repo.go();
    holder.finish();
    assert_eq!(b.finish(), 0);
    assert_eq!(repo.read("log"), "B\n");
    assert!(repo.entries().is_empty(), "{:?}", repo.entries());
}

// pid 1 is alive and not this user's: kill fails with EPERM, as on a process in another sandbox.
#[test]
fn keeps_the_place_of_a_fresh_entry_whose_pid_it_may_not_signal() {
    let repo = Repo::new("eperm-entry");
    repo.fake_entry("1000000000-1");
    let mut waiter = repo.waiter(&mut repo.gl(&["true"]));
    sleep(Duration::from_secs(3));
    assert!(waiter.alive());
}

#[test]
fn drops_an_entry_not_touched_for_a_minute() {
    let repo = Repo::new("stale-entry");
    backdate(&repo.fake_entry("1000000000-1"), 3600);
    assert_eq!(repo.waiter(&mut repo.gl(&["true"])).finish(), 0);
}

#[test]
fn touches_its_entry_on_every_poll_and_drops_it_when_terminated() {
    let repo = Repo::new("touch-entry");
    let old = sleeper();
    repo.fake_lock(&old.pid());
    let mut waiter = repo.waiter(&mut repo.gl(&["true"]));
    repo.polled(&waiter);
    let entry = repo.queue().join(&repo.entries()[0]);
    backdate(&entry, 3600);
    assert!(wait_for(
        || age(&entry).is_some_and(|age| age < Duration::from_secs(60))
    ));
    waiter.terminate();
    assert!(repo.entries().is_empty(), "{:?}", repo.entries());
}

#[test]
fn queues_a_later_arrival_with_a_lower_pid_behind() {
    let repo = Repo::new("later-arrival");
    repo.fake_entry("9999999999-1");
    assert_eq!(repo.waiter(&mut repo.gl(&["true"])).finish(), 0);
}

// A 9-digit epoch is older than a 10-digit one: the order is numeric, not by name.
#[test]
fn orders_the_queue_by_number() {
    let repo = Repo::new("numeric-order");
    let old = sleeper();
    repo.fake_lock(&old.pid());
    repo.fake_entry("999999999-1");
    let mut waiter = repo.waiter(&mut repo.gl(&["true"]));
    assert!(
        wait_for(|| repo.read("err").contains("queue position 2)")),
        "{}",
        repo.read("err")
    );
    assert!(waiter.alive());
}

#[test]
fn leaves_a_dead_holder_lock_to_the_head_of_the_queue() {
    let repo = Repo::new("behind-head");
    let dead = dead_pid();
    repo.fake_lock(&dead);
    repo.fake_entry("999999999-1");
    let _waiter = repo.waiter(&mut repo.gl(&["true"]));
    sleep(Duration::from_secs(3));
    assert_eq!(repo.holder_pid(), dead);
}

#[test]
fn exits_with_the_command_status_and_releases_the_lock() {
    let repo = Repo::new("status");
    assert_eq!(repo.waiter(&mut repo.gl(&["false"])).finish(), 1);
    assert!(!repo.lock().exists());
    assert_eq!(
        repo.waiter(&mut repo.gl(&["sh", "-c", "exit 7"])).finish(),
        7
    );
    assert_eq!(
        repo.waiter(&mut repo.gl(&["no-such-command-here"]))
            .finish(),
        127
    );
    assert!(!repo.lock().exists());
}

#[test]
fn takes_over_a_dead_pid_lock() {
    let repo = Repo::new("dead-pid");
    repo.fake_lock(&dead_pid());
    assert_eq!(repo.waiter(&mut repo.gl(&["true"])).finish(), 0);
    assert!(
        repo.read("err")
            .contains("taking over the stale lock of pid"),
        "{}",
        repo.read("err")
    );
    assert!(!repo.lock().exists());
}

#[test]
fn clears_an_empty_lock_older_than_a_minute() {
    let repo = Repo::new("empty-lock");
    fs::create_dir(repo.lock()).expect("create the lock");
    backdate(&repo.lock(), 3600);
    assert_eq!(repo.waiter(&mut repo.gl(&["true"])).finish(), 0);
}

// A holder killed between its label write and its pid write.
#[test]
fn clears_an_old_lock_holding_only_a_label() {
    let repo = Repo::new("label-only");
    fs::create_dir(repo.lock()).expect("create the lock");
    fs::write(repo.lock().join("label"), "first\n").expect("write the label");
    backdate(&repo.lock(), 3600);
    assert_eq!(repo.waiter(&mut repo.gl(&["true"])).finish(), 0);
}

// Two waiters racing on one dead holder can both take its lock: the loser's exit must leave the
// winner's lock alone.
#[test]
fn keeps_a_lock_taken_over_from_the_exiting_holder() {
    let repo = Repo::new("taken-over");
    let mut holder = repo.holder("holder", &repo.until_go());
    let taker = sleeper();
    fs::write(repo.lock().join("pid"), format!("{}\n", taker.pid())).expect("write the pid");
    repo.go();
    holder.finish();
    assert_eq!(repo.holder_pid(), taker.pid());
}

#[test]
fn runs_a_nested_call_at_once() {
    let repo = Repo::new("nested");
    let nested = [env!("CARGO_BIN_EXE_claude-hooks"), "gate-lock", "true"];
    assert_eq!(repo.waiter(&mut repo.gl(&nested)).finish(), 0);
}

#[test]
fn runs_a_nested_call_whose_holder_it_may_not_signal_and_leaves_the_lock() {
    let repo = Repo::new("nested-eperm");
    repo.fake_lock("1");
    assert_eq!(
        repo.waiter(repo.gl(&["true"]).env("GATE_LOCK_HELD", "1"))
            .finish(),
        0
    );
    assert_eq!(repo.holder_pid(), "1");
}

#[test]
fn queues_a_nested_call_whose_unsignalable_holder_is_past_two_hours() {
    let repo = Repo::new("nested-stale");
    repo.fake_lock("1");
    backdate(&repo.lock().join("pid"), 3 * 3600);
    assert_eq!(
        repo.waiter(repo.gl(&["true"]).env("GATE_LOCK_HELD", "1"))
            .finish(),
        0
    );
    let err = repo.read("err");
    assert!(err.contains("taking over the stale lock of pid 1"), "{err}");
}

#[test]
fn queues_under_a_gate_lock_held_that_names_another_process() {
    let repo = Repo::new("stale-held");
    let mut holder = repo.holder("holder", &repo.until_err("gate-lock: waiting"));
    let me = std::process::id().to_string();
    assert_eq!(
        repo.waiter(repo.gl(&["true"]).env("GATE_LOCK_HELD", me))
            .finish(),
        0
    );
    holder.finish();
    assert!(
        repo.read("err").contains("waiting for holder"),
        "{}",
        repo.read("err")
    );
}

#[test]
fn keeps_waiting_for_a_live_holder_it_may_not_signal() {
    let repo = Repo::new("eperm-holder");
    repo.fake_lock("1");
    let mut waiter = repo.waiter(&mut repo.gl(&["true"]));
    sleep(Duration::from_secs(3));
    assert!(waiter.alive());
    assert_eq!(repo.holder_pid(), "1");
}

// The same holder past 2 h: a pid reused by another user's process.
#[test]
fn takes_over_an_unsignalable_holder_past_two_hours() {
    let repo = Repo::new("eperm-stale");
    repo.fake_lock("1");
    backdate(&repo.lock().join("pid"), 3 * 3600);
    assert_eq!(repo.waiter(&mut repo.gl(&["true"])).finish(), 0);
}

#[test]
fn names_the_pid_and_user_of_a_long_held_lock() {
    let repo = Repo::new("long-held");
    let script = repo.until_err("holds the lock");
    let mut old = start(Command::new("bash").args(["-c", &script]), Stdio::null());
    repo.fake_lock(&old.pid());
    backdate(&repo.lock().join("pid"), 3 * 3600);
    let mut waiter = repo.waiter(&mut repo.gl(&["true"]));
    // Reaped here: an exited child not yet waited for still answers a signal.
    old.finish();
    assert_eq!(waiter.finish(), 0);
    let err = repo.read("err");
    let advice = err
        .lines()
        .find(|line| line.contains("holds the lock"))
        .unwrap_or_default();
    assert!(
        advice.starts_with(&format!("gate-lock: pid {} (user ", old.pid())),
        "{err}"
    );
    // git resolves the fixture's symlinked tmp path, so match the lock path's tail.
    assert!(
        advice.contains("rm -rf /") && advice.ends_with("/w/.git/gate-lock"),
        "{err}"
    );
}

// The holder releases between the waiter's pid read and its notice. The label is a FIFO: opening
// it blocks the releaser until the waiter's notice reads the label, as that release would land.
#[test]
fn prints_no_notice_for_a_holder_that_released_before_it() {
    let repo = Repo::new("released");
    let old = sleeper();
    repo.fake_lock(&old.pid());
    let lock = repo.lock();
    let made = Command::new("mkfifo").arg(lock.join("label")).status();
    assert!(made.is_ok_and(|status| status.success()));
    std::thread::spawn(move || {
        let _open = File::options().write(true).open(lock.join("label"));
        let _ = fs::remove_file(lock.join("pid"));
        let _ = fs::remove_file(lock.join("label"));
        let _ = fs::remove_dir(&lock);
    });
    let ran = repo.scratch("ran");
    assert_eq!(repo.waiter(&mut repo.gl(&["touch", &ran])).finish(), 0);
    assert!(
        !repo.read("err").contains("waiting for"),
        "{}",
        repo.read("err")
    );
    assert!(Path::new(&ran).exists());
}

// A lock with no pid yet, as a holder's between its mkdir and its writes: a notice would name no
// holder.
#[test]
fn prints_no_notice_for_a_lock_with_no_pid_yet() {
    let repo = Repo::new("no-pid");
    fs::create_dir(repo.lock()).expect("create the lock");
    let ran = repo.scratch("ran");
    let mut waiter = repo.waiter(&mut repo.gl(&["touch", &ran]));
    repo.polled(&waiter);
    sleep(Duration::from_millis(500));
    fs::remove_dir(repo.lock()).expect("remove the lock");
    assert_eq!(waiter.finish(), 0);
    assert!(
        !repo.read("err").contains("waiting for"),
        "{}",
        repo.read("err")
    );
    assert!(Path::new(&ran).exists());
}

#[test]
fn takes_a_worktree_lock_in_the_git_common_dir() {
    let repo = Repo::new("worktree");
    git(
        &repo.dir(),
        &["worktree", "add", "-q", "../w2", "-b", "feature/w2"],
    );
    let lock = repo.lock().display().to_string();
    let mut command = hook("gate-lock");
    command
        .current_dir(repo.0.path().join("w2"))
        .args(["test", "-d", &lock]);
    assert_eq!(repo.waiter(&mut command).finish(), 0);
}

#[test]
fn fails_at_once_when_the_lock_cannot_be_created() {
    let repo = Repo::new("unwritable");
    let git_dir = repo.dir().join(".git");
    fs::set_permissions(&git_dir, fs::Permissions::from_mode(0o555)).expect("lock .git");
    let code = repo.waiter(&mut repo.gl(&["true"])).finish();
    fs::set_permissions(&git_dir, fs::Permissions::from_mode(0o755)).expect("unlock .git");
    assert_eq!(code, 1);
    assert!(
        repo.read("err").contains("gate-lock: cannot create "),
        "{}",
        repo.read("err")
    );
}
