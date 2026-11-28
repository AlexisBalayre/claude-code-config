mod common;

use common::{executable, git, git_command, git_stdout, hook, run, write, Output, TempDir};
use std::io::Write;
use std::path::{Path, PathBuf};

struct Fixture {
    tmp: TempDir,
    repo: PathBuf,
}

impl Fixture {
    /// Build a repo on feature/x with the hook installed and a bare remote as origin.
    fn new(test: &str) -> Fixture {
        let tmp = TempDir::new(test);
        let repo = tmp.path().join("w");
        git(tmp.path(), &["init", "-q", "--bare", "remote.git"]);
        git(tmp.path(), &["init", "-q", "-b", "feature/x", "w"]);
        // git runs the hook itself, so each push feeds it the ref lines a real push does.
        executable(
            &repo.join(".git/hooks/pre-push"),
            &format!(
                "#!/bin/sh\nexec '{}' pre-push \"$@\"\n",
                env!("CARGO_BIN_EXE_claude-hooks")
            ),
        );
        let remote = tmp.path().join("remote.git").display().to_string();
        git(&repo, &["remote", "add", "origin", &remote]);
        Fixture { tmp, repo }
    }

    fn push(&self, refs: &[&str]) -> Output {
        run(
            git_command(&self.repo)
                .args(["push", "-q", "origin"])
                .args(refs),
            "",
        )
    }
}

/// Commit `text` as the content of the file `f` in the worktree at `dir`.
fn commit(dir: &Path, text: &str) {
    write(&dir.join("f"), &format!("{text}\n"));
    git(dir, &["add", "f"]);
    git(dir, &["commit", "-qm", text]);
}

/// Record a green verify run for HEAD's tree in the git dir of the worktree at `dir`.
fn stamp(dir: &Path) {
    let tree = git_stdout(dir, &["rev-parse", "HEAD^{tree}"]);
    let sha = git_stdout(dir, &["rev-parse", "HEAD"]);
    let git_dir = dir.join(git_stdout(dir, &["rev-parse", "--git-dir"]));
    let mut stamps = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(git_dir.join("verify-stamp"))
        .expect("open the stamp file");
    writeln!(stamps, "{tree} 2026-01-01T00:00:00Z {sha}").expect("write the stamp");
}

fn assert_pushed(out: &Output) {
    assert_eq!(out.code, 0, "{}", out.text());
}

// The BLOCKED text tells the hook's refusal from a push that failed for another reason.
fn assert_blocked(out: &Output) {
    assert_eq!(out.code, 1, "{}", out.text());
    assert!(out.text().contains("BLOCKED"), "{}", out.text());
}

#[test]
fn blocks_a_push_with_no_stamp() {
    let fixture = Fixture::new("no-stamp");
    commit(&fixture.repo, "a");
    assert_blocked(&fixture.push(&["feature/x"]));
}

#[test]
fn allows_a_push_whose_tree_is_stamped() {
    let fixture = Fixture::new("stamped");
    commit(&fixture.repo, "a");
    stamp(&fixture.repo);
    assert_pushed(&fixture.push(&["feature/x"]));
}

#[test]
fn blocks_a_push_stamped_only_for_an_older_tree() {
    let fixture = Fixture::new("older-tree");
    commit(&fixture.repo, "a");
    stamp(&fixture.repo);
    commit(&fixture.repo, "b");
    assert_blocked(&fixture.push(&["feature/x"]));
}

#[test]
fn message_only_amend_keeps_the_tree_green() {
    let fixture = Fixture::new("amend");
    commit(&fixture.repo, "a");
    stamp(&fixture.repo);
    git(
        &fixture.repo,
        &["commit", "-q", "--amend", "-m", "a, reworded"],
    );
    assert_pushed(&fixture.push(&["feature/x"]));
}

#[test]
fn every_ref_in_one_push_needs_its_own_stamp() {
    let fixture = Fixture::new("two-refs");
    let refs = ["feature/x:refs/heads/feature/x3", "feature/y"];
    commit(&fixture.repo, "a");
    stamp(&fixture.repo);
    git(&fixture.repo, &["checkout", "-q", "-b", "feature/y"]);
    commit(&fixture.repo, "c");
    assert_blocked(&fixture.push(&refs));
    stamp(&fixture.repo);
    assert_pushed(&fixture.push(&refs));
}

#[test]
fn allows_a_stamped_ref_pushed_while_head_is_not_stamped() {
    let fixture = Fixture::new("other-ref");
    commit(&fixture.repo, "a");
    stamp(&fixture.repo);
    git(&fixture.repo, &["checkout", "-q", "-b", "feature/z"]);
    commit(&fixture.repo, "d");
    assert_pushed(&fixture.push(&["feature/x:refs/heads/feature/x2"]));
}

#[test]
fn allows_a_branch_delete() {
    let fixture = Fixture::new("delete");
    commit(&fixture.repo, "a");
    stamp(&fixture.repo);
    assert_pushed(&fixture.push(&["feature/x:refs/heads/feature/y"]));
    commit(&fixture.repo, "d");
    assert_pushed(&fixture.push(&[":feature/y"]));
}

#[test]
fn allows_a_tag_push_without_a_stamp() {
    let fixture = Fixture::new("tag");
    commit(&fixture.repo, "d");
    git(&fixture.repo, &["tag", "v1"]);
    assert_pushed(&fixture.push(&["v1"]));
}

#[test]
fn worktree_reads_its_own_stamp_also_under_git_dash_c() {
    let fixture = Fixture::new("worktree");
    let tmp = fixture.tmp.path();
    commit(&fixture.repo, "a");
    stamp(&fixture.repo);
    commit(&fixture.repo, "d");
    git(
        &fixture.repo,
        &["worktree", "add", "-q", "../w2", "-b", "feature/w2"],
    );
    let other = tmp.join("w2");
    let push = || {
        run(
            git_command(&other)
                .args(["push", "-q", "origin", "feature/w2"])
                .current_dir(tmp),
            "",
        )
    };
    assert_blocked(&push());
    stamp(&other);
    assert_pushed(&push());
}

/// Number the lines `from..=to`, as `seq` does.
fn seq(from: u32, to: u32) -> String {
    (from..=to).map(|n| n.to_string() + "\n").collect()
}

/// A fixture whose origin/main holds three 2,000-line files, for the size cap cases.
struct Sized {
    fixture: Fixture,
    base: String,
}

impl Sized {
    fn new(test: &str) -> Sized {
        let fixture = Fixture::new(test);
        let repo = &fixture.repo;
        commit(repo, "a");
        write(
            &repo.join(".claude/project.env"),
            "SOURCE_EXTENSIONS=\"ts\"\n\
             LINT_CMD=\"stub lint\"\n\
             TYPECHECK_CMD=\"stub typecheck\"\n\
             TEST_CMD=\"stub test\"\n\
             GENERATED_PATHS_REGEX='(^|/)generated/'\n",
        );
        for path in ["big", "src/fixtures/a.json", "src/lib/a.snap"] {
            write(&repo.join(path), &seq(1, 2000));
        }
        git(repo, &["add", "-A"]);
        git(repo, &["commit", "-qm", "big"]);
        git(repo, &["update-ref", "refs/remotes/origin/main", "HEAD"]);
        let base = git_stdout(repo, &["rev-parse", "HEAD"]);
        Sized { fixture, base }
    }

    fn repo(&self) -> &Path {
        &self.fixture.repo
    }

    /// Stage a file of `lines` numbered lines.
    fn add(&self, lines: u32, path: &str) {
        write(&self.repo().join(path), &seq(1, lines));
        git(self.repo(), &["add", path]);
    }

    /// Move a base file and grow it by 1,500 lines, which git still reports as a rename.
    fn move_and_grow(&self, from: &str, to: &str) {
        let target = self.repo().join(to);
        std::fs::create_dir_all(target.parent().expect("a parent directory"))
            .expect("create the target directory");
        git(self.repo(), &["mv", from, to]);
        write(&target, &seq(1, 3500));
        git(self.repo(), &["add", to]);
    }

    /// Commit what is staged and stamp the result.
    fn commit_stamped(&self) {
        git(self.repo(), &["commit", "-qm", "a"]);
        stamp(self.repo());
    }

    /// Commit and stamp `files`, each `(lines, path)`, on top of origin/main.
    fn branch(&self, files: &[(u32, &str)]) {
        for (lines, path) in files {
            self.add(*lines, path);
        }
        self.commit_stamped();
    }

    fn claude_push(&self, branch: &str) -> Output {
        run(
            git_command(self.repo())
                .args(["push", "-q", "origin", &format!("HEAD:refs/heads/{branch}")])
                .env("CLAUDECODE", "1"),
            "",
        )
    }

    /// The verdict line the PR size workflow reads over the same range.
    fn verdict(&self) -> String {
        let out = run(
            hook("added-lines")
                .args([&self.base, "HEAD"])
                .current_dir(self.repo()),
            "",
        );
        assert_eq!(out.code, 0, "{}", out.text());
        out.stdout.lines().next().unwrap_or_default().to_owned()
    }
}

fn warned(out: &Output) -> bool {
    out.text().contains("production lines")
}

#[test]
fn claude_push_adding_399_lines_passes_without_a_warning() {
    let sized = Sized::new("size-399");
    sized.branch(&[(399, "src/a.ts")]);
    let out = sized.claude_push("size-399");
    assert_pushed(&out);
    assert!(!warned(&out), "{}", out.text());
    assert_eq!(sized.verdict(), "pass 399 800");
}

#[test]
fn claude_push_adding_400_lines_warns() {
    let sized = Sized::new("size-400");
    sized.branch(&[(400, "src/a.ts")]);
    let out = sized.claude_push("size-400");
    assert_pushed(&out);
    assert!(warned(&out), "{}", out.text());
    assert_eq!(sized.verdict(), "warn 400 800");
}

#[test]
fn claude_push_adding_799_lines_passes() {
    let sized = Sized::new("size-799");
    sized.branch(&[(799, "src/a.ts")]);
    assert_pushed(&sized.claude_push("size-799"));
}

#[test]
fn claude_push_adding_800_lines_is_blocked() {
    let sized = Sized::new("size-over");
    sized.branch(&[(800, "src/a.ts")]);
    assert_blocked(&sized.claude_push("size-over"));
    assert_eq!(sized.verdict(), "fail 800 800");
}

#[test]
fn push_over_the_cap_outside_claude_code_passes() {
    let sized = Sized::new("size-terminal");
    sized.branch(&[(800, "src/a.ts")]);
    assert_pushed(&sized.fixture.push(&["HEAD:refs/heads/size-over"]));
}

#[test]
fn waived_branch_over_the_cap_passes() {
    let sized = Sized::new("size-waived");
    sized.branch(&[(800, "src/a.ts")]);
    write(&sized.repo().join(".git/size-waiver"), "size-waived\n");
    assert_pushed(&sized.claude_push("size-waived"));
}

#[test]
fn markdown_only_push_adding_1000_lines_passes() {
    let sized = Sized::new("size-docs");
    sized.branch(&[(990, "docs/a.md"), (10, "README.md")]);
    assert_pushed(&sized.claude_push("size-docs"));
    assert_eq!(sized.verdict(), "warn 1000 1001");
}

#[test]
fn markdown_only_push_adding_1001_lines_is_blocked() {
    let sized = Sized::new("size-docs-over");
    sized.branch(&[(990, "docs/a.md"), (10, "README.md"), (1, "docs/c.md")]);
    assert_blocked(&sized.claude_push("size-docs"));
}

#[test]
fn one_non_markdown_file_holds_the_cap_at_800() {
    let sized = Sized::new("size-docs-mixed");
    sized.branch(&[(999, "docs/a.md"), (1, "src/a.ts")]);
    assert_blocked(&sized.claude_push("size-docs-mixed"));
}

#[test]
fn non_ascii_markdown_path_stays_markdown_only() {
    let sized = Sized::new("size-docs-utf8");
    // git quotes this path unless told not to, and a quoted name does not end in .md.
    sized.branch(&[(990, "docs/\u{e9}.md"), (10, "README.md")]);
    assert_eq!(sized.verdict(), "warn 1000 1001");
}

#[test]
fn directory_that_only_ends_in_fixtures_counts() {
    let sized = Sized::new("size-prefixtures");
    sized.branch(&[(800, "src/prefixtures/a.ts")]);
    assert_eq!(sized.verdict(), "fail 800 800");
}

#[test]
fn tests_fixtures_mocks_and_migrations_do_not_count() {
    let sized = Sized::new("size-tests");
    sized.branch(&[
        (700, "src/a.ts"),
        (900, "src/a.test.ts"),
        (900, "scripts/x.test.sh"),
        (900, "app/test/unit/b.ts"),
        (900, "tools/tests/test_x.py"),
        (900, "pkg/x_test.go"),
        (900, "src/fixtures/c.json"),
        (900, "tooling/rule-fixtures/x/safe.patch"),
        (900, "src/a.mock.ts"),
        (900, "src/__mocks__/m.ts"),
        (900, "db/migrations/0001_x.sql"),
    ]);
    assert_pushed(&sized.claude_push("size-tests"));
    assert_eq!(sized.verdict(), "warn 700 800");
}

#[test]
fn path_matching_generated_paths_regex_does_not_count() {
    let sized = Sized::new("size-generated");
    sized.branch(&[(5000, "src/generated/api.ts"), (700, "src/a.ts")]);
    assert_pushed(&sized.claude_push("size-generated"));
}

#[test]
fn nested_lockfile_does_not_count() {
    let sized = Sized::new("size-lock");
    sized.branch(&[
        (3000, "services/api/uv.lock"),
        (3000, "web/package-lock.json"),
        (700, "src/a.ts"),
    ]);
    assert_pushed(&sized.claude_push("size-lock"));
}

#[test]
fn renamed_fixture_counts_under_its_new_name() {
    let sized = Sized::new("size-renamed");
    sized.move_and_grow("src/fixtures/a.json", "src/fixtures/b.json");
    sized.branch(&[(700, "src/a.ts")]);
    assert_pushed(&sized.claude_push("size-renamed"));
    assert_eq!(sized.verdict(), "warn 700 800");
}

#[test]
fn file_moved_into_snapshots_counts_under_its_new_name() {
    let sized = Sized::new("size-snapshot");
    sized.move_and_grow("src/lib/a.snap", "src/__snapshots__/a.snap");
    sized.branch(&[(700, "src/a.ts")]);
    assert_pushed(&sized.claude_push("size-snapshot"));
}

#[test]
fn moved_file_counts_under_its_new_path() {
    let sized = Sized::new("size-moved");
    sized.move_and_grow("big", "tests/big.txt");
    sized.branch(&[(700, "src/a.ts")]);
    assert_pushed(&sized.claude_push("size-moved"));
}

#[test]
fn deleted_lines_do_not_count() {
    let sized = Sized::new("size-deleted");
    git(sized.repo(), &["rm", "-q", "big"]);
    sized.branch(&[(700, "src/a.ts")]);
    assert_pushed(&sized.claude_push("size-deleted"));
}

// A branch or tag named origin/main resolves before refs/remotes/origin/main. Both decoys
// sit before the 2,000-line commit, so a base taken from one would count that commit too.
#[test]
fn local_branch_named_origin_main_does_not_move_the_size_cap_base() {
    let sized = Sized::new("size-decoy");
    sized.branch(&[(10, "src/a.ts")]);
    let before_big = format!("{}^", sized.base);
    git(sized.repo(), &["branch", "origin/main", &before_big]);
    let out = sized.claude_push("size-decoy");
    assert_pushed(&out);
    assert!(!warned(&out), "{}", out.text());
}

#[test]
fn tag_named_origin_main_does_not_move_the_size_cap_base() {
    let sized = Sized::new("size-decoy-tag");
    sized.branch(&[(10, "src/a.ts")]);
    let before_big = format!("{}^", sized.base);
    git(sized.repo(), &["tag", "origin/main", &before_big]);
    assert_pushed(&sized.claude_push("size-decoy-tag"));
}
