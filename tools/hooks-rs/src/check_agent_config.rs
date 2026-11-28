use crate::added_lines::git_loud;
use regex_lite::Regex;
use std::fs::{self, File};
use std::io::{Read, Seek, Write};
use std::path::Path;
use std::process::{exit, Child, Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::Instant;

const SETUP: &str = ".claude/skills/setup-agent-env";
const RULES: &str = "tests/scripts/conventions-rules.test.sh";

/// Spawns `cmd` with stdout and stderr merged into one unlinked temp file, as `>file 2>&1` does,
/// so a background process the command leaves behind cannot hold the reader open.
pub fn spawn_captured(cmd: &mut Command) -> std::io::Result<(Child, File)> {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let name = format!(
        "claude-hooks.{}.{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::SeqCst)
    );
    let path = std::env::temp_dir().join(name);
    let file = File::options()
        .read(true)
        .write(true)
        .create_new(true)
        .open(&path)?;
    let _ = fs::remove_file(&path);
    let child = cmd
        .stdin(Stdio::null())
        .stdout(file.try_clone()?)
        .stderr(file.try_clone()?)
        .spawn()?;
    Ok((child, file))
}

pub fn captured_output(mut file: File) -> Vec<u8> {
    let mut out = Vec::new();
    let _ = file.rewind().and_then(|()| file.read_to_end(&mut out));
    out
}

fn is_word(byte: Option<&u8>) -> bool {
    byte.is_some_and(|b| b.is_ascii_alphanumeric() || *b == b'_')
}

/// `grep -wF`: `name` somewhere in `text` with no word character on either side.
fn names_word(text: &[u8], name: &[u8]) -> bool {
    !name.is_empty()
        && text.windows(name.len()).enumerate().any(|(at, window)| {
            window == name
                && !(at > 0 && is_word(text.get(at - 1)))
                && !is_word(text.get(at + name.len()))
        })
}

/// The regular `*.test.sh` files of `dir`, in byte order.
fn harnesses_in(dir: &str) -> Vec<String> {
    let mut found: Vec<String> = fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| name.ends_with(".test.sh") && !name.starts_with('.'))
        .map(|name| format!("{dir}/{name}"))
        .filter(|path| Path::new(path).is_file())
        .collect();
    found.sort();
    found
}

// A harness X.test.sh is affected when it changed, its subject (X.sh, or an extensionless X, in
// scripts/ for tests/scripts/, in .claude/hooks/ for tests/hooks/, else beside it) changed, or it
// or a helper beside it (X.test.*) names a changed file's basename as a word. Two reads go
// through no name: every conventions doc and rule file is conventions-rules.test.sh's input, and
// the lead-tools params reach their harnesses through lib.sh.
fn affects(harness: &str, changed: &[&str], rules: bool) -> bool {
    let stem = harness.strip_suffix(".test.sh").unwrap_or(harness);
    let (dir, base) = stem.rsplit_once('/').unwrap_or((".", stem));
    let subject = match dir {
        "tests/scripts" => format!("scripts/{base}"),
        "tests/hooks" => format!(".claude/hooks/{base}"),
        _ => stem.to_owned(),
    };
    let direct = changed.iter().any(|file| {
        *file == harness
            || *file == subject
            || file.strip_suffix(".sh") == Some(&subject)
            || (rules
                && harness == RULES
                && (file.starts_with("docs/conventions/") || file.starts_with(".claude/rules/")))
            || (file.starts_with(&format!("{SETUP}/params/"))
                && harness.starts_with(&format!("{SETUP}/lead-tools/")))
    });
    direct
        || fs::read_dir(dir)
            .into_iter()
            .flatten()
            .flatten()
            .any(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with(&format!("{base}.test."))
                    && fs::read(entry.path()).is_ok_and(|text| {
                        changed.iter().any(|file| {
                            names_word(&text, file.rsplit('/').next().unwrap_or(file).as_bytes())
                        })
                    })
            })
}

pub fn run(args: &[String]) {
    let affected = args.first().is_some_and(|arg| arg == "--affected");
    let args = &args[affected as usize..];
    let top = git_loud(&["rev-parse", "--show-toplevel"]).unwrap_or_else(|code| exit(code));
    if std::env::set_current_dir(&top).is_err() {
        exit(1);
    }
    let (Some(base), Some(head)) = (
        args.first().filter(|a| !a.is_empty()),
        args.get(1).filter(|a| !a.is_empty()),
    ) else {
        eprintln!("check-agent-config: usage: check-agent-config.sh [--affected] <base> <head>");
        exit(1);
    };
    let diff = git_loud(&["diff", "--name-only", "--no-renames", base, head])
        .unwrap_or_else(|code| exit(code));
    let changed: Vec<&str> = diff.lines().collect();
    let shebang = Regex::new(r"^#!.*[/ ](ba)?sh([[:space:]]|$)").unwrap_or_else(|_| exit(2));
    let (mut failures, mut harnesses, mut lead_tools, mut rules) = (0, false, false, false);

    for file in &changed {
        lead_tools |= file.starts_with(&format!("{SETUP}/lead-tools/"))
            || file.starts_with(&format!("{SETUP}/params/"));
        rules |= file.starts_with("docs/conventions/") || file.starts_with(".claude/rules/");
        harnesses |= [
            "scripts/",
            "tests/scripts/",
            "tests/hooks/",
            ".github/workflows/",
        ]
        .iter()
        .any(|dir| file.starts_with(dir));
        let exists = Path::new(file).is_file();
        if !file.ends_with(".sh") {
            // An extensionless file here (pre-commit, pre-push) is shell when its shebang says so. A
            // deleted one counts by name alone: nothing is left to read, and a harness may still call it.
            if !(file.starts_with("scripts/") || file.starts_with(".claude/hooks/"))
                || file
                    .rsplit('/')
                    .next()
                    .is_some_and(|name| name.contains('.'))
            {
                continue;
            }
            let text = fs::read(file).unwrap_or_default();
            let first =
                String::from_utf8_lossy(text.split(|b| *b == b'\n').next().unwrap_or_default())
                    .into_owned();
            if exists && !shebang.is_match(&first) {
                continue;
            }
        }
        harnesses |= file.starts_with(".claude/hooks/") || file.ends_with(".test.sh");
        if exists
            && !Command::new("bash")
                .args(["-n", file])
                .status()
                .is_ok_and(|status| status.success())
        {
            eprintln!("check-agent-config: bash -n failed on {file}");
            failures += 1;
        }
    }

    let lead_dir = format!("{SETUP}/lead-tools");
    // tests/scripts/*.test.sh already holds the rules harness, so it joins only when the full set is off.
    let mut list = Vec::new();
    if harnesses {
        list.extend(harnesses_in("tests/scripts"));
        list.extend(harnesses_in("tests/hooks"));
    }
    if lead_tools {
        list.extend(harnesses_in(&lead_dir));
    }
    if !harnesses && rules && Path::new(RULES).is_file() {
        list.push(RULES.to_owned());
    }
    if affected {
        list.retain(|harness| affects(harness, &changed, rules));
    }

    // Longest first, by each harness's last time in this clone (an unknown one counts as longest), so
    // the longest never starts last behind three others. Results still print in list order.
    let times = format!(
        "{}/check-agent-config.times",
        git_loud(&["rev-parse", "--git-common-dir"]).unwrap_or_else(|code| exit(code))
    );
    let _ = fs::create_dir_all(&times);
    let time_file = |harness: &str| format!("{times}/{}", harness.replace('/', "_"));
    let mut order: Vec<(u64, String, usize)> = list
        .iter()
        .enumerate()
        .map(|(i, harness)| {
            let last = fs::read_to_string(time_file(harness))
                .ok()
                .and_then(|secs| secs.trim().parse().ok());
            (last.unwrap_or(999_999), i.to_string(), i)
        })
        .collect();
    order.sort();
    order.reverse();

    // Four at a time: run one by one, a lead-tools change takes many minutes. A caller under a held
    // gate lock must not pass its holder pid to the harnesses, whose gate-lock cases build and hold
    // their own. AGENT_REPO and AGENT_GH_REPO would point a lead-tools suite's scratch REPO at the
    // real one (lib.sh).
    let next = AtomicUsize::new(0);
    let results = Mutex::new(vec![None::<(bool, u64, Vec<u8>)>; list.len()]);
    std::thread::scope(|scope| {
        for _ in 0..4 {
            scope.spawn(|| {
                while let Some((_, _, i)) = order.get(next.fetch_add(1, Ordering::SeqCst)) {
                    let start = Instant::now();
                    let mut cmd = Command::new("bash");
                    cmd.arg(&list[*i])
                        .env_remove("GATE_LOCK_HELD")
                        .env_remove("AGENT_REPO")
                        .env_remove("AGENT_GH_REPO");
                    let (passed, output) = match spawn_captured(&mut cmd) {
                        Ok((mut child, file)) => (
                            child.wait().is_ok_and(|status| status.success()),
                            captured_output(file),
                        ),
                        Err(err) => (false, format!("{err}\n").into_bytes()),
                    };
                    if let Ok(mut results) = results.lock() {
                        results[*i] = Some((passed, start.elapsed().as_secs(), output));
                    }
                }
            });
        }
    });

    for (harness, result) in list.iter().zip(results.into_inner().unwrap_or_default()) {
        let Some((passed, secs, output)) = result else {
            continue;
        };
        let _ = fs::write(time_file(harness), format!("{secs}\n"));
        if passed {
            println!("check-agent-config: {harness} passed ({secs}s)");
        } else {
            eprintln!("check-agent-config: {harness} failed ({secs}s):");
            let mut stderr = std::io::stderr().lock();
            for line in output.split_inclusive(|b| *b == b'\n') {
                let _ = stderr
                    .write_all(b"    ")
                    .and_then(|()| stderr.write_all(line));
            }
            if output.last().is_some_and(|b| *b != b'\n') {
                let _ = stderr.write_all(b"\n");
            }
            failures += 1;
        }
    }

    if failures > 0 {
        eprintln!("check-agent-config: {failures} failure(s)");
        exit(1);
    }
}
