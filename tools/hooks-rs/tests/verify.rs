mod common;

use common::{git, hook, run, write, Output, TempDir};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::thread::sleep;
use std::time::Duration;

const ENV: &str = "SOURCE_EXTENSIONS=\"ts\"\nLINT_CMD=\"stub lint\"\nTYPECHECK_CMD=\"stub typecheck\"\nTEST_CMD=\"stub test\"\n";
const STUB: &str = "#!/bin/bash
echo \"$1\" >>\"$STUB_CALLS\"
[ \"$1\" = lint ] && echo \"$*\" >>\"$STUB_CALLS.lint\"
[ \"$1\" = \"${STUB_TOUCH:-}\" ] && : >touched
[ \"$1\" = \"${STUB_COMMIT:-}\" ] && git commit -q --allow-empty -m mid-run
[ \"$1\" != \"${STUB_FAIL:-}\" ] && [ \"$1 ${2:-}\" != \"${STUB_FAIL:-}\" ]
";

struct Repo(TempDir);

impl Repo {
    fn new(test: &str) -> Repo {
        Repo::with_env(test, ENV)
    }

    fn with_env(test: &str, env: &str) -> Repo {
        let repo = Repo(TempDir::new(test));
        git(repo.0.path(), &["init", "-q", "-b", "feature/x", "r"]);
        let stub = repo.0.path().join("bin/stub");
        write(&stub, STUB);
        fs::set_permissions(&stub, fs::Permissions::from_mode(0o755)).expect("chmod the stub");
        repo.commit(&[
            (".claude/project.env", env),
            ("a.ts", "let a\n"),
            (
                "tests/scripts/mark.test.sh",
                "echo harness >>\"$STUB_CALLS\"\n",
            ),
        ]);
        repo
    }

    fn dir(&self) -> PathBuf {
        self.0.path().join("r")
    }

    fn commit(&self, files: &[(&str, &str)]) {
        for (path, text) in files {
            write(&self.dir().join(path), text);
        }
        git(&self.dir(), &["add", "-A"]);
        git(&self.dir(), &["commit", "-qm", "next"]);
    }

    /// The environment of a verify run: the stub first on PATH, and an identity for the stub's
    /// own commit.
    fn env<'a>(&self, command: &'a mut Command) -> &'a mut Command {
        let path = std::env::var("PATH").unwrap_or_default();
        command
            .current_dir(self.dir())
            .env(
                "PATH",
                format!("{}:{path}", self.0.path().join("bin").display()),
            )
            .env("STUB_CALLS", self.0.path().join("calls"))
            .env("GIT_AUTHOR_NAME", "t")
            .env("GIT_AUTHOR_EMAIL", "t@t")
            .env("GIT_COMMITTER_NAME", "t")
            .env("GIT_COMMITTER_EMAIL", "t@t")
    }

    fn verify(&self, stub: &[(&str, &str)]) -> Output {
        let mut command = hook("verify");
        run(self.env(&mut command).envs(stub.iter().copied()), "")
    }

    /// What the stub and the fixture harness logged, in order.
    fn calls(&self) -> Vec<String> {
        let calls = fs::read_to_string(self.0.path().join("calls")).unwrap_or_default();
        calls.lines().map(String::from).collect()
    }

    /// The command line of each lint run, in order.
    fn linted(&self) -> Vec<String> {
        let runs = fs::read_to_string(self.0.path().join("calls.lint")).unwrap_or_default();
        runs.lines().map(String::from).collect()
    }

    fn stamp(&self) -> String {
        fs::read_to_string(self.dir().join(".git/verify-stamp")).unwrap_or_default()
    }
}

fn last_line(out: &Output) -> &str {
    out.stderr.lines().last().unwrap_or_default()
}

#[test]
fn stamps_a_clean_green_tree() {
    let repo = Repo::new("green");
    let out = repo.verify(&[]);
    assert_eq!(out.code, 0, "{}", out.stderr);
    let mut calls = repo.calls();
    assert_eq!(calls.split_off(2), ["typecheck", "test"]);
    calls.sort();
    assert_eq!(calls, ["harness", "lint"]);
    let tree = Command::new("git")
        .args(["rev-parse", "HEAD^{tree}"])
        .current_dir(repo.dir())
        .output()
        .expect("run git");
    let tree = String::from_utf8_lossy(&tree.stdout).trim_end().to_owned();
    assert!(
        repo.stamp().starts_with(&format!("{tree} ")),
        "{}",
        repo.stamp()
    );
    assert!(
        out.stdout.contains(&format!("verify: tree {tree} of ")),
        "{}",
        out.stdout
    );
    assert!(out.stdout.ends_with(" is green.\n"), "{}", out.stdout);
}

#[test]
fn refuses_a_dirty_tree_before_any_gate() {
    let repo = Repo::new("dirty");
    write(&repo.dir().join("new.txt"), "new\n");
    let out = repo.verify(&[]);
    assert_eq!(out.code, 1);
    assert!(
        out.stderr
            .starts_with("verify: commit or drop your changes first"),
        "{}",
        out.stderr
    );
    assert!(repo.calls().is_empty(), "{:?}", repo.calls());
    assert_eq!(repo.stamp(), "");

    git(&repo.dir(), &["config", "status.showUntrackedFiles", "no"]);
    assert_eq!(
        repo.verify(&[]).code,
        1,
        "an untracked file hidden by the config"
    );
    assert_eq!(repo.stamp(), "");
}

#[test]
fn names_a_red_suite_and_stops_there() {
    let repo = Repo::new("red-typecheck");
    let out = repo.verify(&[("STUB_FAIL", "typecheck")]);
    assert_eq!(out.code, 1);
    assert_eq!(last_line(&out), "verify: red: stub typecheck");
    assert!(
        !repo.calls().contains(&"test".to_owned()),
        "{:?}",
        repo.calls()
    );
    assert_eq!(repo.stamp(), "");

    let repo = Repo::new("red-test");
    let out = repo.verify(&[("STUB_FAIL", "test")]);
    assert_eq!(out.code, 1);
    assert_eq!(last_line(&out), "verify: red: stub test");
    assert_eq!(repo.stamp(), "");
}

#[test]
fn names_a_red_lint_and_runs_no_suite() {
    let repo = Repo::new("red-lint");
    let out = repo.verify(&[("STUB_FAIL", "lint")]);
    assert_ne!(out.code, 0);
    assert_eq!(last_line(&out), "verify: red: lint");
    assert!(
        !repo.calls().contains(&"typecheck".to_owned()),
        "{:?}",
        repo.calls()
    );
    assert_eq!(repo.stamp(), "");
}

#[test]
fn stays_red_on_a_red_lint_whose_report_cannot_be_printed() {
    let repo = Repo::new("closed-stderr");
    let mut shell = Command::new("sh");
    shell.args([
        "-c",
        "exec \"$@\" 2>&-",
        "_",
        env!("CARGO_BIN_EXE_claude-hooks"),
        "verify",
    ]);
    shell
        .env_clear()
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1");
    let out = run(repo.env(&mut shell).env("STUB_FAIL", "lint"), "");
    assert_ne!(out.code, 0);
    assert_eq!(repo.stamp(), "");
}

#[test]
fn names_a_failing_harness() {
    let repo = Repo::new("red-harness");
    repo.commit(&[("tests/scripts/fail.test.sh", "exit 1\n")]);
    let out = repo.verify(&[]);
    assert_ne!(out.code, 0);
    assert_eq!(last_line(&out), "verify: red: check-agent-config");
    assert!(
        !repo.calls().contains(&"typecheck".to_owned()),
        "{:?}",
        repo.calls()
    );
    assert_eq!(repo.stamp(), "");
}

#[test]
fn refuses_a_tree_that_changed_during_the_run() {
    let repo = Repo::new("touched");
    let out = repo.verify(&[("STUB_TOUCH", "test")]);
    assert_eq!(out.code, 1);
    assert!(
        out.stderr
            .contains("verify: HEAD or the tree changed during the run"),
        "{}",
        out.stderr
    );
    assert_eq!(repo.stamp(), "");
}

#[test]
fn refuses_a_head_that_moved_during_the_run() {
    let repo = Repo::new("moved");
    let out = repo.verify(&[("STUB_COMMIT", "test")]);
    assert_eq!(out.code, 1);
    assert!(
        out.stderr
            .contains("verify: HEAD or the tree changed during the run"),
        "{}",
        out.stderr
    );
    assert_eq!(repo.stamp(), "");
}

// A local branch named origin/main sits on the commit before a.ts: taken as the base, it would
// make a.ts a changed source and run the lint.
#[test]
fn diffs_from_the_remote_trunk_and_lints_only_changed_sources() {
    let repo = Repo::new("base");
    repo.commit(&[("b.ts", "let b\n")]);
    git(
        &repo.dir(),
        &["update-ref", "refs/remotes/origin/main", "HEAD"],
    );
    git(&repo.dir(), &["branch", "origin/main", "HEAD^"]);
    repo.commit(&[("docs/a.md", "text\n")]);
    let out = repo.verify(&[]);
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert_eq!(repo.calls(), ["typecheck", "test"]);
}

#[test]
fn lints_the_changed_sources_that_still_exist() {
    let repo = Repo::new("lint-files");
    repo.commit(&[("gone.ts", "let g\n")]);
    git(
        &repo.dir(),
        &["update-ref", "refs/remotes/origin/main", "HEAD"],
    );
    fs::remove_file(repo.dir().join("gone.ts")).expect("delete the source");
    repo.commit(&[("b.ts", "let b\n"), ("docs/a.md", "text\n")]);
    let out = repo.verify(&[]);
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert_eq!(repo.linted(), ["lint b.ts"]);
}

/// A change of 5002 sources: one exec of every path would outgrow the argument limit. The
/// fixture's own a.ts is the first path of the first run.
fn large_change(test: &str) -> Repo {
    let repo = Repo::new(test);
    let names: Vec<String> = (0..5001).map(|n| format!("many/f{n:04}.ts")).collect();
    let files: Vec<(&str, &str)> = names.iter().map(|name| (name.as_str(), "")).collect();
    repo.commit(&files);
    repo
}

#[test]
fn lints_a_large_change_in_several_runs() {
    let repo = large_change("lint-runs");
    let out = repo.verify(&[]);
    assert_eq!(out.code, 0, "{}", out.stderr);
    let runs = repo.linted();
    assert_eq!(runs.len(), 2);
    assert!(runs[0].ends_with(" many/f4998.ts"));
    assert_eq!(runs[1], "lint many/f4999.ts many/f5000.ts");
}

#[test]
fn fails_when_any_lint_run_fails() {
    let repo = large_change("lint-runs-red");
    let out = repo.verify(&[("STUB_FAIL", "lint a.ts")]);
    assert_eq!(out.code, 1);
    assert_eq!(last_line(&out), "verify: red: lint");
    assert_eq!(repo.linted().len(), 2);
}

#[test]
fn stamps_without_a_suite_when_none_is_configured() {
    let env =
        "SOURCE_EXTENSIONS=\"ts\"\nLINT_CMD=\"stub lint\"\nTYPECHECK_CMD=\"\"\nTEST_CMD=\"\"\n";
    let repo = Repo::with_env("no-suites", env);
    let out = repo.verify(&[]);
    assert_eq!(out.code, 0, "{}", out.stderr);
    let mut calls = repo.calls();
    calls.sort();
    assert_eq!(calls, ["harness", "lint"]);
    assert_ne!(repo.stamp(), "");
}

// Lint and the harnesses need no machine to themselves, so they run while another run holds the
// lock; the suites wait for it.
#[test]
fn runs_lint_and_harnesses_outside_the_lock_and_the_suites_under_it() {
    let repo = Repo::new("held");
    let mut holder = Command::new("sleep")
        .arg("60")
        .spawn()
        .expect("spawn the holder");
    let lock = repo.dir().join(".git/gate-lock");
    write(&lock.join("pid"), &format!("{}\n", holder.id()));
    let mut command = hook("verify");
    let mut verify = repo
        .env(&mut command)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn verify");
    let queued =
        || fs::read_dir(repo.dir().join(".git/gate-lock.queue")).is_ok_and(|d| d.count() > 0);
    for _ in 0..300 {
        if queued() {
            break;
        }
        sleep(Duration::from_millis(100));
    }
    let before = repo.calls();
    holder.kill().expect("kill the holder");
    holder.wait().expect("reap the holder");
    let status = verify.wait().expect("wait for verify");
    let mut early = before.clone();
    early.sort();
    assert_eq!(early, ["harness", "lint"], "while the lock was held");
    assert!(status.success());
    assert_eq!(repo.calls()[2..], ["typecheck", "test"]);
    assert_ne!(repo.stamp(), "");
}
