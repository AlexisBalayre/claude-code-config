mod common;

use common::{
    executable, git, git_command, git_stdout, hook, path_with, run, write, Output, TempDir,
};
use std::path::{Path, PathBuf};

const OTHER_URL: &str = "git@github.com:other/repo.git";

struct Fixture {
    tmp: TempDir,
    clone: PathBuf,
    /// Origin's URL, which is also its push URL until a test splits them.
    url: String,
    plans: PathBuf,
}

impl Fixture {
    /// Build a checkout with a bare origin, a `gh` stub that lists no merged PR and a scratch HOME.
    fn new(test: &str) -> Fixture {
        let tmp = TempDir::new(test);
        let clone = tmp.path().join("clone");
        let url = tmp.path().join("origin.git").display().to_string();
        git(
            tmp.path(),
            &["init", "-q", "--bare", "-b", "main", "origin.git"],
        );
        git(tmp.path(), &["init", "-q", "-b", "main", "clone"]);
        git(&clone, &["remote", "add", "origin", &url]);
        write(&clone.join("f"), "a\n");
        git(&clone, &["add", "f"]);
        git(&clone, &["commit", "-qm", "a"]);
        git(&clone, &["push", "-q", "origin", "main"]);
        for dir in [clone.join(".worktrees"), tmp.path().join("home")] {
            std::fs::create_dir_all(dir).expect("create a fixture directory");
        }
        executable(&tmp.path().join("bin/gh"), "#!/bin/sh\nexit 0\n");
        let plans = tmp.path().join("plans");
        Fixture {
            tmp,
            clone,
            url,
            plans,
        }
    }

    /// Add the worktree `.worktrees/<name>` on `feature/<name>` and push its branch.
    fn worktree(&self, name: &str) {
        let (branch, dir) = (format!("feature/{name}"), format!(".worktrees/{name}"));
        git(&self.clone, &["worktree", "add", "-q", "-b", &branch, &dir]);
        git(&self.clone, &["push", "-q", "origin", &branch]);
    }

    /// Have `gh` list one merged PR whose head is the trunk's tip, where every fixture branch sits.
    fn merged_pr_ships_every_branch(&self) {
        let tip = git_stdout(&self.clone, &["rev-parse", "HEAD"]);
        executable(
            &self.tmp.path().join("bin/gh"),
            &format!("#!/bin/sh\necho {tip}\n"),
        );
    }

    fn command(&self, args: &[&str]) -> std::process::Command {
        let mut command = hook("worktree-remove");
        command
            .args(args)
            .current_dir(&self.clone)
            .env("PATH", path_with(&[&self.tmp.path().join("bin")]))
            .env("HOME", self.tmp.path().join("home"))
            .env("AGENT_APPROVED_PLANS", &self.plans);
        command
    }

    fn remove(&self, args: &[&str]) -> Output {
        run(&mut self.command(args), "")
    }

    fn has_branch(&self, branch: &str) -> bool {
        let head = format!("refs/heads/{branch}");
        run(
            git_command(&self.clone).args(["rev-parse", "-q", "--verify", &head]),
            "",
        )
        .code
            == 0
    }

    fn plan_lines(&self) -> Vec<String> {
        std::fs::read_to_string(&self.plans)
            .expect("read the approved-plans file")
            .lines()
            .map(str::to_owned)
            .collect()
    }
}

fn find_git() -> PathBuf {
    std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default())
        .map(|dir| dir.join("git"))
        .find(|candidate| candidate.is_file())
        .expect("git on PATH")
}

#[test]
fn deleting_a_pushed_branch_drops_only_its_own_plan_line() {
    let fixture = Fixture::new("pushed");
    let url = &fixture.url;
    fixture.worktree("x");
    write(
        &fixture.plans,
        &format!(
            "feature/x {url} KEY-1 12:00\nfeature/x {OTHER_URL} KEY-9 12:00\nfeature/y {url} KEY-2 12:01\n"
        ),
    );
    // From a subdirectory: the command finds the main checkout itself.
    let sub = fixture.clone.join("sub");
    std::fs::create_dir(&sub).expect("create the subdirectory");
    let out = run(fixture.command(&["x"]).current_dir(&sub), "");
    assert_eq!(out.code, 0, "{}", out.text());
    assert!(!fixture.has_branch("feature/x"), "{}", out.text());
    assert_eq!(
        fixture.plan_lines(),
        [
            format!("feature/x {OTHER_URL} KEY-9 12:00"),
            format!("feature/y {url} KEY-2 12:01"),
        ]
    );
}

#[test]
fn deleting_a_branch_a_merged_pr_shipped_drops_its_plan_line() {
    let fixture = Fixture::new("merged");
    let url = &fixture.url;
    fixture.worktree("z");
    fixture.merged_pr_ships_every_branch();
    write(
        &fixture.plans,
        &format!("feature/z {url} KEY-3 12:02\nfeature/y {url} KEY-2 12:01\n"),
    );
    let out = fixture.remove(&["z"]);
    assert_eq!(out.code, 0, "{}", out.text());
    assert_eq!(
        fixture.plan_lines(),
        [format!("feature/y {url} KEY-2 12:01")]
    );
}

#[test]
fn plan_line_is_matched_on_the_push_url_of_origin() {
    let fixture = Fixture::new("push-url");
    let url = &fixture.url;
    fixture.worktree("v");
    fixture.merged_pr_ships_every_branch();
    let push_url = fixture.tmp.path().join("push.git").display().to_string();
    git(fixture.tmp.path(), &["init", "-q", "--bare", "push.git"]);
    git(
        &fixture.clone,
        &["remote", "set-url", "--push", "origin", &push_url],
    );
    write(
        &fixture.plans,
        &format!("feature/v {push_url} KEY-5 12:03\nfeature/v {url} KEY-6 12:04\n"),
    );
    let out = fixture.remove(&["v"]);
    assert_eq!(
        fixture.plan_lines(),
        [format!("feature/v {url} KEY-6 12:04")],
        "{}",
        out.text()
    );
}

#[test]
fn unreadable_push_url_keeps_the_plan_line_and_says_so() {
    let fixture = Fixture::new("no-push-url");
    let url = &fixture.url;
    fixture.worktree("u");
    fixture.merged_pr_ships_every_branch();
    let wrapper_dir = fixture.tmp.path().join("nourl");
    executable(
        &wrapper_dir.join("git"),
        &format!(
            "#!/bin/sh\n[ \"$1 $2 $3\" = \"remote get-url --push\" ] && exit 2\nexec \"{}\" \"$@\"\n",
            find_git().display()
        ),
    );
    write(&fixture.plans, &format!("feature/u {url} KEY-7 12:05\n"));
    let bin = fixture.tmp.path().join("bin");
    let out = run(
        fixture
            .command(&["u"])
            .env("PATH", path_with(&[&wrapper_dir, &bin])),
        "",
    );
    assert!(!fixture.has_branch("feature/u"), "{}", out.text());
    assert_eq!(
        fixture.plan_lines(),
        [format!("feature/u {url} KEY-7 12:05")]
    );
    assert!(
        out.text().contains("origin has no push URL"),
        "{}",
        out.text()
    );
}

#[test]
fn unset_plans_file_still_deletes_the_branch_and_says_the_line_stayed() {
    let fixture = Fixture::new("no-plans");
    fixture.worktree("w");
    fixture.merged_pr_ships_every_branch();
    let out = run(
        fixture.command(&["w"]).env_remove("AGENT_APPROVED_PLANS"),
        "",
    );
    assert!(
        out.text()
            .contains("Kept feature/w's line in approved-plans"),
        "{}",
        out.text()
    );
}

/// Run `--merged` over three directories git lists as no worktree, as a merged worker's
/// open session leaves them: one empty, one holding a file, one holding an empty file.
fn sweep_leftovers(test: &str) -> (Fixture, Output) {
    let fixture = Fixture::new(test);
    fixture.merged_pr_ships_every_branch();
    let worktrees = fixture.clone.join(".worktrees");
    std::fs::create_dir_all(worktrees.join("gone/.claude/.cc-writes"))
        .expect("create the empty leftover");
    write(&worktrees.join("held/.claude/f"), "x\n");
    write(&worktrees.join("heldempty/.claude/f"), "");
    let out = fixture.remove(&["--merged"]);
    assert_eq!(out.code, 0, "{}", out.text());
    (fixture, out)
}

fn leftover(fixture: &Fixture, rel: &str) -> PathBuf {
    Path::new(&fixture.clone).join(".worktrees").join(rel)
}

#[test]
fn merged_sweep_removes_an_empty_leftover_directory() {
    let (fixture, out) = sweep_leftovers("leftover-empty");
    assert!(!leftover(&fixture, "gone").exists(), "{}", out.text());
    assert!(
        !out.text().contains("Kept .worktrees/gone"),
        "{}",
        out.text()
    );
}

#[test]
fn merged_sweep_keeps_a_leftover_directory_holding_a_file() {
    let (fixture, out) = sweep_leftovers("leftover-file");
    assert!(
        leftover(&fixture, "held/.claude/f").is_file(),
        "{}",
        out.text()
    );
    assert!(
        out.text()
            .contains("Kept .worktrees/held: not a registered worktree"),
        "{}",
        out.text()
    );
}

#[test]
fn merged_sweep_keeps_a_leftover_directory_holding_an_empty_file() {
    let (fixture, out) = sweep_leftovers("leftover-empty-file");
    assert!(
        leftover(&fixture, "heldempty/.claude/f").exists(),
        "{}",
        out.text()
    );
    assert!(
        out.text()
            .contains("Kept .worktrees/heldempty: not a registered worktree"),
        "{}",
        out.text()
    );
}

#[test]
fn worktree_named_in_the_wrong_case_is_removed_not_taken_for_a_leftover() {
    let fixture = Fixture::new("wrong-case");
    fixture.worktree("c");
    // Only a case-insensitive filesystem resolves the wrong case to the worktree.
    if !leftover(&fixture, "C").is_dir() {
        return;
    }
    fixture.merged_pr_ships_every_branch();
    let out = fixture.remove(&["C"]);
    assert!(!leftover(&fixture, "c").exists(), "{}", out.text());
    assert!(
        !out.text().contains("not a registered worktree"),
        "{}",
        out.text()
    );
}
