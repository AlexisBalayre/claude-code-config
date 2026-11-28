mod common;

use common::{git, hook, run, write, Output, TempDir};
use std::path::{Path, PathBuf};

const MESSAGE: &str = "src must not call forbidden_call (core.md)";
const HEADING: &str = "\nConvention spot-check on files changed this session:\n";
const ADVICE: &str = "\nFix each real finding; if one is a heuristic misfire, say so and stop again (this check stays silent on the re-run).\n";

struct Fixture {
    _tmp: TempDir,
    repo: PathBuf,
}

impl Fixture {
    /// Build a committed repository whose untracked tsv forbids one call under `src/`.
    fn new(test: &str) -> Fixture {
        let tmp = TempDir::new(test);
        let repo = tmp.path().join("repo");
        write(&repo.join("src/a.ts"), "export const a = 1;\n");
        write(&repo.join("src/gone.ts"), "export const gone = 1;\n");
        git(tmp.path(), &["init", "-q", "-b", "main", "repo"]);
        git(&repo, &["add", "-A"]);
        git(&repo, &["commit", "-qm", "init"]);
        let fixture = Fixture { _tmp: tmp, repo };
        fixture.checks(&format!(
            "# a comment line\n\n^src/\tforbidden_call\\(\t{MESSAGE}\n"
        ));
        fixture
    }

    /// Build the repository with one untracked violation in `src/` and one in `lib/`.
    fn with_violations(test: &str) -> Fixture {
        let fixture = Fixture::new(test);
        write(&fixture.repo.join("src/new.ts"), "forbidden_call(1);\n");
        write(&fixture.repo.join("lib/out.ts"), "forbidden_call(2);\n");
        fixture
    }

    fn tsv(&self) -> PathBuf {
        self.repo.join(".claude/spot-checks.tsv")
    }

    fn checks(&self, text: &str) {
        write(&self.tsv(), text);
    }

    fn spot_check(&self, stdin: &str) -> Output {
        run(
            hook("convention-spot-check").env("CLAUDE_PROJECT_DIR", &self.repo),
            stdin,
        )
    }
}

fn findings(out: &Output) -> Vec<&str> {
    out.stderr
        .lines()
        .filter(|line| line.starts_with("  "))
        .collect()
}

fn assert_silent(out: &Output) {
    assert_eq!(out.code, 0, "stderr: {}", out.stderr);
    assert_eq!(format!("{}{}", out.stdout, out.stderr), "");
}

fn in_dir(dir: &Path, stdin: &str) -> Output {
    run(hook("convention-spot-check").current_dir(dir), stdin)
}

#[test]
fn no_changed_file_is_silent() {
    assert_silent(&Fixture::new("clean").spot_check("{}"));
}

#[test]
fn untracked_file_matching_path_and_content_exits_2_with_the_finding() {
    let out = Fixture::with_violations("finding").spot_check(r#"{"stop_hook_active": false}"#);
    assert_eq!(out.code, 2);
    assert_eq!(
        out.stderr,
        format!("{HEADING}  src/new.ts: {MESSAGE}\n{ADVICE}")
    );
    assert_eq!(out.stdout, "");
}

#[test]
fn non_json_stdin_still_reports() {
    let out = Fixture::with_violations("not-json").spot_check("not json");
    assert_eq!(out.code, 2, "stderr: {}", out.stderr);
}

#[test]
fn stop_hook_active_is_silent() {
    assert_silent(&Fixture::with_violations("active").spot_check(r#"{"stop_hook_active":true}"#));
}

#[test]
fn stop_hook_active_with_spaces_around_the_colon_is_silent() {
    let out =
        Fixture::with_violations("active-spaced").spot_check(r#"{"stop_hook_active" : true}"#);
    assert_eq!(out.code, 0, "stderr: {}", out.stderr);
}

#[test]
fn modified_and_staged_files_are_checked_and_a_deleted_one_is_skipped() {
    let fixture = Fixture::with_violations("modified-staged");
    write(
        &fixture.repo.join("src/a.ts"),
        "export const a = 1;\nforbidden_call(3);\n",
    );
    write(&fixture.repo.join("src/staged.ts"), "forbidden_call(4);\n");
    git(&fixture.repo, &["add", "src/staged.ts"]);
    std::fs::remove_file(fixture.repo.join("src/gone.ts")).expect("delete the tracked file");
    let out = fixture.spot_check("{}");
    assert_eq!(out.code, 2, "stderr: {}", out.stderr);
    let want: Vec<String> = ["a", "new", "staged"]
        .iter()
        .map(|name| format!("  src/{name}.ts: {MESSAGE}"))
        .collect();
    assert_eq!(findings(&out), want);
}

#[test]
fn tsv_of_comments_and_blanks_is_silent() {
    let fixture = Fixture::with_violations("comment-only");
    fixture.checks("# only comments\n\n   \n");
    assert_silent(&fixture.spot_check("{}"));
}

#[test]
fn missing_tsv_is_silent() {
    let fixture = Fixture::with_violations("no-tsv");
    std::fs::remove_file(fixture.tsv()).expect("remove the tsv");
    assert_silent(&fixture.spot_check("{}"));
}

#[test]
fn commented_and_invalid_checks_are_skipped_and_tabs_and_escapes_honoured() {
    let fixture = Fixture::with_violations("tsv-parsing");
    fixture.checks(
        "#^src/\tforbidden_call\tcommented check\n\
         ^src/(\tforbidden_call\tinvalid path regex\n\
         ^lib/\t\t\tforbidden_call\tlib finding\\twith a tab\n",
    );
    let out = fixture.spot_check("{}");
    assert_eq!(out.code, 2, "stderr: {}", out.stderr);
    assert_eq!(findings(&out), ["  lib/out.ts: lib finding\twith a tab"]);
}

#[test]
fn last_line_without_a_newline_is_read() {
    let fixture = Fixture::with_violations("unterminated");
    fixture.checks("^nowhere/\tx\tfirst\n^src/\tforbidden_call\tunterminated");
    let out = fixture.spot_check("{}");
    assert_eq!(out.code, 2, "stderr: {}", out.stderr);
    assert_eq!(findings(&out), ["  src/new.ts: unterminated"]);
}

#[test]
fn without_a_project_dir_it_runs_from_the_git_top_level() {
    let fixture = Fixture::with_violations("no-project-dir");
    let out = in_dir(&fixture.repo.join("src"), "{}");
    assert_eq!(out.code, 2, "stderr: {}", out.stderr);
    assert_eq!(findings(&out), [format!("  src/new.ts: {MESSAGE}")]);
}
