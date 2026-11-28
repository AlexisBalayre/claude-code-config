use crate::check_agent_config::{captured_output, git_or_exit, spawn_captured};
use crate::common::{git, Config};
use regex_lite::Regex;
use std::io::Write;
use std::os::unix::process::ExitStatusExt;
use std::process::{exit, Command};

// An explicit mode, since status.showUntrackedFiles=no would hide a new file
// the gates read from disk but the commit lacks.
fn clean() -> bool {
    git(".", &["status", "--porcelain", "--untracked-files=normal"])
        .unwrap_or_default()
        .is_empty()
}

pub fn run(_args: &[String]) {
    let top = git_or_exit(&["rev-parse", "--show-toplevel"]);
    let Ok(exe) = std::env::current_exe() else {
        exit(1)
    };
    if std::env::set_current_dir(&top).is_err() {
        exit(1);
    }
    let config = Config::load(".");

    if !clean() {
        eprintln!("verify: commit or drop your changes first; a dirty tree is never stamped.");
        exit(1);
    }
    let head = git_or_exit(&["rev-parse", "HEAD"]);
    // With no merge-base (the trunk was never fetched), diff from the empty tree, so every file counts
    // as changed. By its full name: a bare origin/<trunk> resolves to a branch or tag of that name first.
    let trunk = Some(config.get("GIT_TRUNK"))
        .filter(|trunk| !trunk.is_empty())
        .unwrap_or("main".into());
    let base = git(
        ".",
        &[
            "merge-base",
            &format!("refs/remotes/origin/{trunk}"),
            "HEAD",
        ],
    )
    .unwrap_or_else(|| git_or_exit(&["hash-object", "-t", "tree", "/dev/null"]));

    let extensions: Vec<String> = config
        .get("SOURCE_EXTENSIONS")
        .split(' ')
        .filter(|e| !e.is_empty())
        .map(String::from)
        .collect();
    let sources: Vec<String> = Regex::new(&format!(r"\.({})$", extensions.join("|")))
        .ok()
        .filter(|_| !extensions.is_empty())
        .map(|source| {
            git(
                ".",
                &["diff", "--name-only", "--diff-filter=d", &base, "HEAD"],
            )
            .unwrap_or_default()
            .lines()
            .filter(|file| source.is_match(file))
            .map(String::from)
            .collect()
        })
        .unwrap_or_default();

    // Lint and the shell harnesses run at once, before the lock: they need no machine to themselves,
    // so a red one never takes the lock.
    let lint_cmd = config.get("LINT_CMD");
    let lint = (!lint_cmd.is_empty() && !sources.is_empty()).then(|| {
        spawn_captured(
            Command::new("sh")
                .arg("-c")
                .arg(format!("{lint_cmd} \"$@\""))
                .arg("_")
                .args(&sources),
        )
    });
    let mut red = Vec::new();
    let checked = Command::new(&exe)
        .args(["check-agent-config", "--affected", &base, "HEAD"])
        .status();
    if !checked.is_ok_and(|status| status.success()) {
        red.push("check-agent-config");
    }
    // red is set before the print, so a print that fails (a closed pipe) cannot pass a red job.
    match lint {
        Some(Ok((mut child, file))) => {
            if child.wait().is_ok_and(|status| status.success()) {
                if std::io::stdout().write_all(&captured_output(file)).is_err() {
                    exit(1);
                }
            } else {
                red.push("lint");
                let _ = std::io::stderr().write_all(&captured_output(file));
            }
        }
        Some(Err(err)) => {
            red.push("lint");
            eprintln!("verify: lint: {err}");
        }
        None => {}
    }
    // The last stderr line names what failed, since the output above it can run to pages.
    if !red.is_empty() {
        eprintln!("verify: red: {}", red.join(", "));
        exit(1);
    }

    let steps: Vec<String> = ["TYPECHECK_CMD", "TEST_CMD"]
        .iter()
        .map(|key| config.get(key))
        .filter(|step| !step.is_empty())
        .collect();
    if !steps.is_empty() {
        let script = r#"for step in "$@"; do bash -c "$step" || { echo "verify: red: $step" >&2; exit 1; }; done"#;
        let status = Command::new(&exe)
            .args(["gate-lock", "bash", "-c", script, "_"])
            .args(&steps)
            .status();
        match status {
            Ok(status) if status.success() => {}
            Ok(status) => exit(
                status
                    .code()
                    .unwrap_or_else(|| 128 + status.signal().unwrap_or(0)),
            ),
            Err(_) => exit(1),
        }
    }

    if git(".", &["rev-parse", "HEAD"]).as_deref() != Some(&head) || !clean() {
        eprintln!("verify: HEAD or the tree changed during the run, so the gates tested something else. Run scripts/verify.sh again.");
        exit(1);
    }

    let tree = git_or_exit(&["rev-parse", &format!("{head}^{{tree}}")]);
    let stamp = format!("{}/verify-stamp", git_or_exit(&["rev-parse", "--git-dir"]));
    let date = Command::new("date")
        .args(["-u", "+%Y-%m-%dT%H:%M:%SZ"])
        .output()
        .map(|out| out.stdout)
        .unwrap_or_default();
    let line = format!(
        "{tree} {} {head}\n",
        String::from_utf8_lossy(&date).trim_end()
    );
    let appended = std::fs::File::options()
        .append(true)
        .create(true)
        .open(&stamp)
        .and_then(|mut file| file.write_all(line.as_bytes()));
    if let Err(err) = appended {
        eprintln!("verify: {stamp}: {err}");
        exit(1);
    }
    println!("verify: tree {tree} of {head} is green.");
}
