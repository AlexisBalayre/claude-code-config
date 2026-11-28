mod common;

use common::{git, hook, run, write, TempDir};
use std::path::Path;

const EM: char = '\u{2014}';
const EN: char = '\u{2013}';

/// Build a repo with one commit: a.md holds a dashed line and a plain one.
fn repo(test: &str) -> TempDir {
    let tmp = TempDir::new(test);
    git(tmp.path(), &["init", "-q"]);
    write(
        &tmp.path().join("a.md"),
        &format!("old {EM} line\nplain line\n"),
    );
    git(tmp.path(), &["add", "a.md"]);
    git(tmp.path(), &["commit", "-qm", "init"]);
    tmp
}

fn stage(dir: &Path, file: &str, text: &str) {
    write(&dir.join(file), text);
    git(dir, &["add", file]);
}

fn exit_for(dir: &Path, args: &[&str]) -> i32 {
    run(hook("check-dashes").args(args).current_dir(dir), "").code
}

#[test]
fn blocks_an_added_line_with_an_em_dash() {
    let tmp = repo("em-dash");
    stage(
        tmp.path(),
        "a.md",
        &format!("old {EM} line\nplain line\nnew {EM} line\n"),
    );
    assert_eq!(exit_for(tmp.path(), &["--cached"]), 1);
}

#[test]
fn blocks_an_added_line_with_an_en_dash_in_a_new_file() {
    let tmp = repo("en-dash");
    stage(tmp.path(), "b.ts", &format!("pages 2{EN}5\n"));
    assert_eq!(exit_for(tmp.path(), &["--cached"]), 1);
}

#[test]
fn allows_a_dash_on_an_unchanged_neighbouring_line() {
    let tmp = repo("neighbour");
    stage(
        tmp.path(),
        "a.md",
        &format!("old {EM} line\nplain line, edited\n"),
    );
    assert_eq!(exit_for(tmp.path(), &["--cached"]), 0);
}

#[test]
fn allows_a_dash_on_a_removed_line() {
    let tmp = repo("removed");
    stage(tmp.path(), "a.md", "plain line\n");
    assert_eq!(exit_for(tmp.path(), &["--cached"]), 0);
}

#[test]
fn blocks_an_added_line_starting_with_two_pluses() {
    let tmp = repo("plus-plus");
    // In the diff this line reads `+++ counter`, the shape of a file header.
    stage(
        tmp.path(),
        "a.md",
        &format!("old {EM} line\nplain line\n++ counter {EM}\n"),
    );
    assert_eq!(exit_for(tmp.path(), &["--cached"]), 1);
}

#[test]
fn blocks_a_dash_in_a_committed_range() {
    let tmp = repo("range");
    stage(
        tmp.path(),
        "a.md",
        &format!("old {EM} line\nplain line\nnew {EN} line\n"),
    );
    git(tmp.path(), &["commit", "-qm", "next"]);
    assert_eq!(exit_for(tmp.path(), &["HEAD^1", "HEAD"]), 1);
}
