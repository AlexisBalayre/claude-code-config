mod common;

use common::{executable, git, git_command, hook, path_with, run, write, Output, TempDir};
use std::path::PathBuf;

const EM: char = '\u{2014}';

struct Fixture {
    tmp: TempDir,
    repo: PathBuf,
}

impl Fixture {
    /// Build a repo with a lint profile, a tracked plan and a stub lint command.
    fn new(test: &str) -> Fixture {
        let tmp = TempDir::new(test);
        let repo = tmp.path().join("w");
        git(tmp.path(), &["init", "-q", "-b", "main", "w"]);
        write(
            &repo.join(".claude/project.env"),
            "SOURCE_EXTENSIONS=\"ts py\"\nLINT_CMD=\"stub lint\"\n",
        );
        write(&repo.join("docs/plans/old.md"), "plan\n");
        write(&repo.join("sub/keep"), "keep\n");
        executable(
            &tmp.path().join("bin/stub"),
            "#!/bin/bash\necho \"$*\" >>\"$STUB_CALLS\"\n[ \"${STUB_FAIL:-}\" != 1 ]\n",
        );
        git(&repo, &["add", "."]);
        git(&repo, &["commit", "-qm", "init"]);
        Fixture { tmp, repo }
    }

    fn stage(&self, file: &str, text: &str) {
        write(&self.repo.join(file), text);
        git(&self.repo, &["add", file]);
    }

    /// Run the hook from a subdirectory: it finds the checkout's root itself.
    fn hook(&self, lint_fails: bool) -> Output {
        let mut command = hook("pre-commit");
        command
            .current_dir(self.repo.join("sub"))
            .env("PATH", path_with(&[&self.tmp.path().join("bin")]))
            .env("STUB_CALLS", self.tmp.path().join("calls"));
        if lint_fails {
            command.env("STUB_FAIL", "1");
        }
        run(&mut command, "")
    }

    /// The arguments of every stub call, empty when the lint never ran.
    fn linted(&self) -> String {
        std::fs::read_to_string(self.tmp.path().join("calls"))
            .unwrap_or_default()
            .trim_end()
            .to_owned()
    }
}

fn says(out: &Output, line: &str) -> bool {
    out.text().lines().any(|printed| printed == line)
}

fn stage_source_and_docs(fixture: &Fixture) {
    fixture.stage("a.ts", "a\n");
    fixture.stage("b c.py", "b\n");
    fixture.stage("d.md", "d\n");
}

#[test]
fn lints_the_staged_source_files_and_passes() {
    let fixture = Fixture::new("green-lint");
    stage_source_and_docs(&fixture);
    let out = fixture.hook(false);
    assert_eq!(out.code, 0, "{}", out.text());
    assert_eq!(fixture.linted(), "lint a.ts b c.py");
    assert!(says(&out, "Running lint: stub lint"), "{}", out.text());
    assert!(
        says(&out, "All pre-commit checks passed."),
        "{}",
        out.text()
    );
}

#[test]
fn blocks_on_a_red_lint() {
    let fixture = Fixture::new("red-lint");
    stage_source_and_docs(&fixture);
    let out = fixture.hook(true);
    assert_eq!(out.code, 1, "{}", out.text());
    assert!(
        says(&out, "ERROR: Lint failed on staged files."),
        "{}",
        out.text()
    );
}

#[test]
fn runs_no_lint_when_no_source_file_is_staged() {
    let fixture = Fixture::new("no-source");
    fixture.stage("d.md", "d\n");
    let out = fixture.hook(false);
    assert_eq!(out.code, 0, "{}", out.text());
    assert_eq!(fixture.linted(), "");
}

#[test]
fn does_not_lint_a_deleted_source_file() {
    let fixture = Fixture::new("deleted-source");
    fixture.stage("a.ts", "a\n");
    git(&fixture.repo, &["commit", "-qm", "a"]);
    git(&fixture.repo, &["rm", "-q", "a.ts"]);
    let out = fixture.hook(false);
    assert_eq!(out.code, 0, "{}", out.text());
    assert_eq!(fixture.linted(), "");
}

#[test]
fn blocks_a_file_staged_under_docs_plans() {
    let fixture = Fixture::new("plans");
    fixture.stage("docs/plans/new.md", "new\n");
    let out = fixture.hook(false);
    assert_eq!(out.code, 1, "{}", out.text());
    assert!(says(&out, "  docs/plans/new.md"), "{}", out.text());
    assert!(
        says(&out, "Run 'git reset HEAD -- docs/plans/' to unstage them."),
        "{}",
        out.text()
    );
}

#[test]
fn blocks_a_file_staged_under_docs_scratch() {
    let fixture = Fixture::new("scratch");
    fixture.stage("docs/scratch/x.md", "x\n");
    let out = fixture.hook(false);
    assert_eq!(out.code, 1, "{}", out.text());
}

#[test]
fn allows_removing_a_tracked_file_under_docs_plans() {
    let fixture = Fixture::new("plans-removal");
    git(&fixture.repo, &["rm", "-q", "docs/plans/old.md"]);
    let out = fixture.hook(false);
    assert_eq!(out.code, 0, "{}", out.text());
}

#[test]
fn blocks_an_added_line_with_an_em_dash() {
    let fixture = Fixture::new("em-dash");
    fixture.stage("d.md", &format!("new {EM} line\n"));
    let out = fixture.hook(false);
    assert_eq!(out.code, 1, "{}", out.text());
    assert!(
        says(&out, &format!("  d.md: new {EM} line")),
        "{}",
        out.text()
    );
}

#[test]
fn skips_the_dash_check_mid_merge() {
    let fixture = Fixture::new("mid-merge");
    let repo = &fixture.repo;
    git(repo, &["checkout", "-q", "-b", "side"]);
    write(&repo.join("sub/keep"), &format!("theirs {EM}\n"));
    git(repo, &["commit", "-qam", "theirs"]);
    git(repo, &["checkout", "-q", "main"]);
    write(&repo.join("sub/keep"), "ours\n");
    git(repo, &["commit", "-qam", "ours"]);
    let merge = run(git_command(repo).args(["merge", "-q", "side"]), "");
    assert_ne!(merge.code, 0, "the fixture merge must stop on a conflict");
    assert!(repo.join(".git/MERGE_HEAD").is_file());
    fixture.stage("sub/keep", &format!("merged {EM}\n"));
    let out = fixture.hook(false);
    assert_eq!(out.code, 0, "{}", out.text());
}

#[test]
fn runs_no_lint_without_a_lint_command() {
    let fixture = Fixture::new("no-lint-cmd");
    write(&fixture.repo.join(".claude/project.env"), "");
    fixture.stage("a.ts", "a\n");
    let out = fixture.hook(false);
    assert_eq!(out.code, 0, "{}", out.text());
    assert_eq!(fixture.linted(), "");
}
