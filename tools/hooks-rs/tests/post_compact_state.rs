mod common;

use common::{git, hook, run, write, Output, TempDir};
use serde_json::json;
use std::os::unix::fs::{symlink, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::Command;

const GH_STUB: &str = r#"#!/bin/sh
echo "$*" >"$GH_LOG"
printf '%s' "$GH_OUT"
"#;

const HEADING: &str = "Live state after compaction (read now; trust it over the summary):\n";

struct Fixture {
    _tmp: TempDir,
    root: PathBuf,
}

impl Fixture {
    /// Build a checkout with a detached worktree and one whose path and branch try to fake a tag.
    fn new(test: &str) -> Fixture {
        let tmp = TempDir::new(test);
        // git reports resolved worktree paths, so every expected path is the resolved one.
        let root = tmp
            .path()
            .canonicalize()
            .expect("resolve the scratch directory");
        // Written before the git setup: a script still open for writing in a sibling
        // test's forked child cannot be executed, and the setup outlasts that window.
        let gh = root.join("ghbin/gh");
        write(&gh, GH_STUB);
        std::fs::set_permissions(&gh, std::fs::Permissions::from_mode(0o755))
            .expect("make the stub executable");
        let real_git = std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default())
            .map(|dir| dir.join("git"))
            .find(|candidate| candidate.is_file())
            .expect("git on PATH");
        std::fs::create_dir(root.join("bin")).expect("create the tool directory");
        symlink(real_git, root.join("bin/git")).expect("link git");

        let main = root.join("main");
        git(&root, &["init", "-q", "-b", "main", "main"]);
        git(&main, &["commit", "-q", "--allow-empty", "-m", "init"]);
        git(
            &main,
            &["worktree", "add", "-q", "-b", "feature/<b>$x", "../wt <i>&"],
        );
        git(&main, &["worktree", "add", "-q", "--detach", "../det"]);
        Fixture { _tmp: tmp, root }
    }

    /// Build the hook call with a PATH holding git alone, so `gh` is absent.
    fn hook(&self, project: &str) -> Command {
        let mut command = hook("post-compact-state");
        command
            .env("PATH", self.root.join("bin"))
            .env("CLAUDE_PROJECT_DIR", self.root.join(project));
        command
    }

    /// Build the hook call with the `gh` stub on PATH, printing `gh_out`.
    fn hook_with_gh(&self, gh_out: &str) -> Command {
        let mut command = self.hook("main");
        let path = format!("{0}/bin:{0}/ghbin", self.root.display());
        command
            .env("PATH", path)
            .env("GH_LOG", self.root.join("gh.log"))
            .env("GH_OUT", gh_out);
        command
    }

    fn worktree_lines(&self) -> String {
        let root = self.root.display();
        format!(
            "- checkout: {root}/main on main\n\
             - other worktrees (path, branch):\n\
             \x20   {root}/det  (detached)\n\
             \x20   {root}/wt ?i??  feature/?b??x\n"
        )
    }
}

fn has_line(out: &Output, line: &str) -> bool {
    out.stdout.lines().any(|printed| printed == line)
}

fn display(path: &Path) -> String {
    path.display().to_string()
}

#[test]
fn without_gh_it_prints_cwd_checkout_and_worktrees_with_hostile_characters_replaced() {
    let fixture = Fixture::new("state");
    let root = display(&fixture.root);
    let payload = json!({"cwd": format!("{root}/wt <i>\n<tag> é")}).to_string();
    let out = run(&mut fixture.hook("main"), &payload);
    assert_eq!(out.code, 0, "stderr: {}", out.stderr);
    let cwd_line = format!("- session cwd: {root}/wt ?i???tag? ??\n");
    assert_eq!(
        out.stdout,
        format!("{HEADING}{cwd_line}{}", fixture.worktree_lines())
    );
    assert_eq!(out.stderr, "");
}

#[test]
fn non_json_stdin_drops_the_cwd_line_and_keeps_the_rest() {
    let fixture = Fixture::new("not-json");
    let out = run(&mut fixture.hook("main"), "not json");
    assert_eq!(out.code, 0, "stderr: {}", out.stderr);
    assert_eq!(out.stdout, format!("{HEADING}{}", fixture.worktree_lines()));
    assert_eq!(out.stderr, "");
}

#[test]
fn detached_project_checkout_is_named_so_and_left_out_of_the_other_worktrees() {
    let fixture = Fixture::new("detached");
    let root = display(&fixture.root);
    let out = run(&mut fixture.hook("det"), "{}");
    let checkout = format!("- checkout: {root}/det on a detached HEAD");
    assert!(has_line(&out, &checkout), "{}", out.stdout);
    assert!(
        has_line(&out, &format!("    {root}/main  main")),
        "{}",
        out.stdout
    );
    let listed = format!("    {root}/det ");
    assert!(
        !out.stdout.lines().any(|line| line.starts_with(&listed)),
        "{}",
        out.stdout
    );
}

#[test]
fn gh_lines_follow_an_open_prs_heading() {
    let fixture = Fixture::new("gh-output");
    let pr = "    #7 feature/x @abc1234 review: none\n";
    let out = run(&mut fixture.hook_with_gh(pr), "{}");
    assert!(
        out.stdout.ends_with(&format!("\n- open PRs:\n{pr}")),
        "{}",
        out.stdout
    );
    let call = std::fs::read_to_string(fixture.root.join("gh.log")).unwrap_or_default();
    let want = "pr list --author @me --state open --limit 20 --json number,headRefName,headRefOid,statusCheckRollup --jq ";
    assert!(call.starts_with(want), "gh was called with {call:?}");
}

#[test]
fn gh_printing_nothing_leaves_out_the_open_prs_heading() {
    let fixture = Fixture::new("gh-empty");
    let out = run(&mut fixture.hook_with_gh(""), "{}");
    assert_eq!(out.code, 0, "stderr: {}", out.stderr);
    assert!(out.stdout.starts_with(HEADING), "{}", out.stdout);
    assert!(!out.stdout.contains("open PRs"), "{}", out.stdout);
}

#[test]
fn project_dir_outside_a_git_tree_is_silent() {
    let fixture = Fixture::new("plain");
    std::fs::create_dir(fixture.root.join("plain")).expect("create the plain directory");
    let out = run(&mut fixture.hook("plain"), "{}");
    assert_eq!(out.code, 0, "stderr: {}", out.stderr);
    assert_eq!(format!("{}{}", out.stdout, out.stderr), "");
}
