mod common;

use common::{git, git_command, git_stdout, hook, run, write, Output, TempDir};
use std::path::PathBuf;

struct Fixture {
    // Held for its Drop, which removes the fixture.
    _tmp: TempDir,
    clone: PathBuf,
    stale_head: String,
    new_tip: String,
}

/// Build a clone whose HEAD is behind origin's trunk, with a local branch named origin/main.
fn fixture(test: &str) -> Fixture {
    let tmp = TempDir::new(test);
    let (seed, clone) = (tmp.path().join("seed"), tmp.path().join("clone"));
    let origin = tmp.path().join("origin.git");
    let origin_url = origin.to_str().expect("a UTF-8 temp path");
    git(
        tmp.path(),
        &["init", "-q", "--bare", "-b", "main", "origin.git"],
    );
    git(tmp.path(), &["init", "-q", "-b", "main", "seed"]);
    git(&seed, &["remote", "add", "origin", origin_url]);
    write(&seed.join("f"), "a\n");
    git(&seed, &["add", "f"]);
    git(&seed, &["commit", "-qm", "a"]);
    git(&seed, &["push", "-q", "origin", "main"]);
    git(tmp.path(), &["clone", "-q", origin_url, "clone"]);
    write(&seed.join("f"), "b\n");
    git(&seed, &["commit", "-qam", "b"]);
    git(&seed, &["push", "-q", "origin", "main"]);

    let stale_head = git_stdout(&clone, &["rev-parse", "HEAD"]);
    let new_tip = git_stdout(&seed, &["rev-parse", "HEAD"]);
    // A sandboxed worker can create this branch, and a bare origin/main resolves to it first.
    git(&clone, &["branch", "origin/main", &stale_head]);
    Fixture {
        _tmp: tmp,
        clone,
        stale_head,
        new_tip,
    }
}

fn attempt(fixture: &Fixture, args: &[&str]) -> Output {
    run(
        hook("worktree-create")
            .args(args)
            .current_dir(&fixture.clone),
        "",
    )
}

fn create(fixture: &Fixture, name: &str) {
    let out = attempt(fixture, &[name]);
    assert_eq!(out.code, 0, "{}", out.text());
}

#[test]
fn branches_from_the_fetched_origin_trunk_not_the_stale_head() {
    let fixture = fixture("origin-tip");
    assert_ne!(
        fixture.stale_head, fixture.new_tip,
        "the clone's HEAD must be behind origin/main"
    );
    create(&fixture, "x");
    assert_eq!(
        git_stdout(&fixture.clone, &["rev-parse", "feature/x"]),
        fixture.new_tip
    );
}

#[test]
fn new_branch_tracks_no_upstream() {
    let fixture = fixture("no-upstream");
    create(&fixture, "x");
    let upstream = run(
        git_command(&fixture.clone).args(["rev-parse", "--abbrev-ref", "feature/x@{upstream}"]),
        "",
    );
    assert_ne!(
        upstream.code, 0,
        "feature/x tracks {}; a bare git push would target it",
        upstream.stdout
    );
}

#[test]
fn refuses_a_name_that_could_leave_the_worktrees_directory() {
    let fixture = fixture("invalid-name");
    for name in ["-f", "../x", "a/b", "a b"] {
        let out = attempt(&fixture, &[name]);
        assert_eq!(out.code, 1, "{name}: {}", out.text());
        assert!(
            out.stderr.contains("invalid worktree name"),
            "{name}: {}",
            out.text()
        );
    }
    assert!(!fixture.clone.join(".worktrees").exists());
}

#[test]
fn refuses_a_missing_name_with_the_usage_line() {
    let fixture = fixture("no-name");
    let out = attempt(&fixture, &[]);
    assert_eq!(out.code, 1, "{}", out.text());
    assert!(out.stderr.contains("Usage:"), "{}", out.text());
}

#[test]
fn refuses_a_worktree_that_already_exists() {
    let fixture = fixture("twice");
    create(&fixture, "x");
    let out = attempt(&fixture, &["x"]);
    assert_eq!(out.code, 1, "{}", out.text());
    assert!(out.stderr.contains("already exists"), "{}", out.text());
}

#[test]
fn names_the_branch_with_the_configured_prefix() {
    let fixture = fixture("prefix");
    write(
        &fixture.clone.join(".claude/project.env"),
        "WORKTREE_BRANCH_PREFIX=task\n",
    );
    create(&fixture, "x");
    assert_eq!(
        git_stdout(&fixture.clone, &["rev-parse", "task/x"]),
        fixture.new_tip
    );
}

#[test]
fn runs_the_install_command_in_the_new_worktree() {
    let fixture = fixture("install");
    write(
        &fixture.clone.join(".claude/project.env"),
        "INSTALL_CMD='touch installed'\n",
    );
    create(&fixture, "x");
    assert!(fixture.clone.join(".worktrees/x/installed").is_file());
}
