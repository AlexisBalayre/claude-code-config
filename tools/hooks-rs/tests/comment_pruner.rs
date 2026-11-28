mod common;

use common::{git, hook, run, write, Output, TempDir};
use std::collections::BTreeSet;
use std::path::PathBuf;

const COMMITTED: &str = "export const a = 1;\nexport const b = 2;\n";
const WITH_CODE: &str = "export const a = 1;\nexport const b = 2;\nexport const c = 3;\n";
const TRACKED_NOTE: &str = "export const d = 4; // tracked note\n";

/// Source line and the comment text it holds.
const COMMENTS: [(&str, &str); 13] = [
    ("// full line", "// full line"),
    ("  x = 1; // trailing", "// trailing"),
    ("/* block */", "/* block */"),
    ("   * doc continuation", "* doc continuation"),
    ("y = 2; /* mid */ z = 3;", "/* mid */ z = 3;"),
    ("# hash", "# hash"),
    ("v = 1  # trailing hash", "# trailing hash"),
    ("t = 1;\t# hash after a tab", "# hash after a tab"),
    ("#", "#"),
    ("k = 1; /* first */ // second", "/* first */ // second"),
    ("w = \"http://a.b/c\"; // after a url", "// after a url"),
    ("// see https://example.com/doc", "// see example.com/doc"),
    (
        "# see https://example.com/doc",
        "# see https://example.com/doc",
    ),
];

const NOT_COMMENTS: [&str; 7] = [
    "#include <stdio.h>",
    "#[derive(Debug)]",
    "this.#secret = 1;",
    "n = 1; #nospace",
    "u = \"https://example.com/x\";",
    "plain = code;",
    "",
];

const DISPATCH: &str = "
comment-pruner: 2 net-new comment line(s) added this session.
Dispatch the comment-pruner subagent (Agent tool, subagent_type: comment-pruner) to prune the
comments ADDED versus HEAD in the files below, then run from the repo root:
  .claude/hooks/comment-pruner.sh seal

Files:
src/a.ts
src/b.py
";

struct Fixture {
    _tmp: TempDir,
    repo: PathBuf,
}

impl Fixture {
    /// Build a repository with one committed source file and a two-extension profile.
    fn new(test: &str) -> Fixture {
        let tmp = TempDir::new(test);
        let repo = tmp.path().join("repo");
        write(
            &repo.join(".claude/project.env"),
            "SOURCE_EXTENSIONS=\"ts  py\"\nGENERATED_PATHS_REGEX=\"\\.gen\\.ts$\"\n",
        );
        write(&repo.join("src/a.ts"), COMMITTED);
        git(tmp.path(), &["init", "-q", "-b", "main", "repo"]);
        git(&repo, &["add", "-A"]);
        git(&repo, &["commit", "-qm", "init"]);
        Fixture { _tmp: tmp, repo }
    }

    /// Build the repository with a comment in a tracked, an untracked and four excluded files.
    fn with_comments(test: &str) -> Fixture {
        let fixture = Fixture::new(test);
        fixture.source(&format!("{WITH_CODE}{TRACKED_NOTE}"));
        fixture.file("src/b.py", "# untracked note\nx = 1\n");
        fixture.file("src/x.gen.ts", "// generated\n");
        fixture.file("node_modules/dep/i.ts", "// vendored\n");
        fixture.file("dist/out.ts", "// built\n");
        fixture.file("notes.md", "// not a source file\n");
        fixture
    }

    fn sealed(test: &str) -> Fixture {
        let fixture = Fixture::with_comments(test);
        fixture.seal();
        fixture
    }

    fn file(&self, rel: &str, text: &str) {
        write(&self.repo.join(rel), text);
    }

    fn source(&self, text: &str) {
        self.file("src/a.ts", text);
    }

    fn stop(&self, stdin: &str) -> Output {
        run(
            hook("comment-pruner").env("CLAUDE_PROJECT_DIR", &self.repo),
            stdin,
        )
    }

    fn seal(&self) -> Output {
        run(
            hook("comment-pruner")
                .arg("seal")
                .env("CLAUDE_PROJECT_DIR", &self.repo),
            "",
        )
    }

    fn memo(&self) -> Option<String> {
        std::fs::read_to_string(self.repo.join(".git/comment-pruner-memo")).ok()
    }

    fn head(&self) -> String {
        let head = std::fs::read_to_string(self.repo.join(".git/refs/heads/main"));
        head.expect("read the branch ref").trim_end().to_owned()
    }
}

fn assert_silent(out: &Output) {
    assert_eq!(out.code, 0, "stderr: {}", out.stderr);
    assert_eq!(format!("{}{}", out.stdout, out.stderr), "");
}

fn assert_dispatches_one(out: &Output) {
    assert_eq!(out.code, 2, "stderr: {}", out.stderr);
    assert!(
        out.stderr
            .lines()
            .any(|line| line.starts_with("comment-pruner: 1 net-new ")),
        "stderr: {}",
        out.stderr
    );
}

#[test]
fn added_code_without_a_comment_is_silent() {
    let fixture = Fixture::new("code-only");
    fixture.source(WITH_CODE);
    assert_silent(&fixture.stop("{}"));
}

#[test]
fn new_comments_exit_2_with_the_count_and_the_files() {
    let out = Fixture::with_comments("dispatch").stop(r#"{"stop_hook_active": false}"#);
    assert_eq!(out.code, 2);
    assert_eq!(out.stderr, DISPATCH);
    assert_eq!(out.stdout, "");
}

#[test]
fn non_json_stdin_still_dispatches() {
    let out = Fixture::with_comments("not-json").stop("not json");
    assert_eq!(out.code, 2, "stderr: {}", out.stderr);
}

#[test]
fn stop_hook_active_exits_0_with_the_advisory() {
    let out = Fixture::with_comments("active").stop(r#"{"stop_hook_active":true}"#);
    assert_eq!(out.code, 0);
    assert_eq!(
        out.stderr,
        "\ncomment-pruner: 2 net-new comment line(s) still flagged after one dispatch this cycle; advisory only.\n"
    );
    assert_eq!(out.stdout, "");
}

#[test]
fn seal_records_head_then_each_comment_text() {
    let fixture = Fixture::with_comments("seal");
    assert_silent(&fixture.seal());
    let want = format!("{}\n# untracked note\n// tracked note\n", fixture.head());
    assert_eq!(fixture.memo().as_deref(), Some(want.as_str()));
}

#[test]
fn sealed_comments_are_silent() {
    assert_silent(&Fixture::sealed("after-seal").stop("{}"));
}

#[test]
fn code_edited_around_a_sealed_comment_is_silent() {
    let fixture = Fixture::sealed("edit-around");
    fixture.source(&format!(
        "{COMMITTED}export const c = 33;\nexport const dd = 44; // tracked note\n"
    ));
    assert_silent(&fixture.stop("{}"));
}

#[test]
fn comment_added_after_seal_is_the_only_one_counted() {
    let fixture = Fixture::sealed("after-seal-new");
    fixture.source(&format!("{WITH_CODE}{TRACKED_NOTE}/* another */\n"));
    assert_dispatches_one(&fixture.stop("{}"));
}

#[test]
fn nothing_added_since_a_commit_is_silent() {
    let fixture = Fixture::sealed("after-commit");
    git(&fixture.repo, &["add", "-A"]);
    git(&fixture.repo, &["commit", "-qm", "more"]);
    assert_silent(&fixture.stop("{}"));
}

#[test]
fn commit_resets_the_memo_so_a_sealed_text_dispatches_again() {
    let fixture = Fixture::sealed("memo-reset");
    git(&fixture.repo, &["add", "-A"]);
    git(&fixture.repo, &["commit", "-qm", "more"]);
    fixture.source(&format!(
        "{WITH_CODE}{TRACKED_NOTE}export const e = 5; // tracked note\n"
    ));
    assert_dispatches_one(&fixture.stop("{}"));
}

#[test]
fn each_comment_syntax_is_recorded_and_lookalikes_are_not() {
    let fixture = Fixture::new("syntax");
    let lines = COMMENTS.iter().map(|(line, _)| *line).chain(NOT_COMMENTS);
    fixture.file(
        "src/syntax.ts",
        &(lines.collect::<Vec<_>>().join("\n") + "\n"),
    );
    fixture.seal();
    let memo = fixture.memo().expect("a memo");
    let recorded: BTreeSet<&str> = memo.lines().skip(1).collect();
    for (line, text) in COMMENTS {
        assert!(
            recorded.contains(text),
            "{line:?}: {text:?} not in the memo"
        );
    }
    assert_eq!(
        recorded.len(),
        COMMENTS.len(),
        "a line that holds no comment was recorded"
    );
}

#[test]
fn no_source_extensions_is_silent_and_seal_writes_no_memo() {
    let fixture = Fixture::new("no-extensions");
    fixture.file(".claude/project.env", "SOURCE_EXTENSIONS=\"\"\n");
    fixture.file("src/c.ts", "// a note\n");
    assert_silent(&fixture.stop("{}"));
    fixture.seal();
    assert_eq!(fixture.memo(), None);
}
