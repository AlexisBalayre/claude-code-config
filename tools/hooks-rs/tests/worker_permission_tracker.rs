mod common;

use common::{git, run, verdict, Lead, Verdict, WORKER};
use serde_json::json;
use std::path::{Path, PathBuf};

const COMMENT: &str = "gh issue comment 12 --body done";

/// Hold the repository the worker on issue 12 was started in.
struct Tracker {
    lead: Lead,
    repo: PathBuf,
}

impl Tracker {
    fn new(test: &str) -> Tracker {
        let lead = Lead::new(test);
        let repo = lead.repo("repo", "feature/issue-12");
        Tracker { lead, repo }
    }

    /// Ask the hook about a Bash call made from `cwd` by a session started in the repository.
    fn ask_from(&self, cwd: &Path, command: &str) -> Verdict {
        let payload = json!({"cwd": cwd, "tool_name": "Bash", "tool_input": {"command": command}});
        let mut hook = self.lead.hook(WORKER);
        hook.env("CLAUDE_PROJECT_DIR", &self.repo);
        verdict(&run(&mut hook, &payload.to_string()))
    }

    fn ask(&self, command: &str) -> Verdict {
        self.ask_from(&self.repo, command)
    }

    /// Assert each call, made from the repository, is refused with a request a grant can follow.
    fn assert_grant_route(&self, commands: &[&str]) {
        for command in commands {
            let verdict = self.ask(command);
            assert!(
                !verdict.id().is_empty(),
                "command: {command:?}: {}",
                verdict.message
            );
        }
    }
}

#[test]
fn a_comment_on_the_workers_own_issue_is_allowed() {
    let tracker = Tracker::new("wpt-comment");
    for command in [
        "gh issue comment 12 --body done",
        "gh issue comment 12 -b done",
        "gh issue comment 12 --body=done",
        "gh issue comment 12 --body 'Fixed in `src/x.rs`: costs $5; see #3 & more'",
        "gh issue comment 12 --body \"it's done, see (a) and [b]\"",
        "gh issue comment 12 --body \"a \\\"quoted\\\" word\"",
        "gh issue comment 12 --body 'first line\nsecond line'",
        "gh issue comment 12 --body-file notes/status.md",
        "gh issue comment 12 -F notes/status.md",
        "gh issue comment 12 --body-file=notes/status.md",
        "gh issue comment --body done 12",
        "gh issue comment 12 --edit-last --body done",
        "gh issue comment 12 --edit-last --create-if-none --body done",
        "gh issue comment 12 --body 'gh pr merge 5 is the lead to ask for'",
        " gh issue comment 12 --body done \n",
    ] {
        let verdict = tracker.ask(command);
        assert_eq!(verdict.behavior, "allow", "command: {command:?}");
    }
}

#[test]
fn a_new_body_for_the_workers_own_issue_is_allowed() {
    let tracker = Tracker::new("wpt-edit");
    for command in [
        "gh issue edit 12 --body 'the whole new body'",
        "gh issue edit 12 -b body",
        "gh issue edit 12 --body-file notes/body.md",
        "gh issue edit 12 -F notes/body.md",
        "gh issue edit --body-file=notes/body.md 12",
    ] {
        let verdict = tracker.ask(command);
        assert_eq!(verdict.behavior, "allow", "command: {command:?}");
    }
}

#[test]
fn a_read_of_any_issue_is_allowed() {
    let tracker = Tracker::new("wpt-view");
    for command in [
        "gh issue view 12",
        "gh issue view 999",
        "gh issue view 999 --comments",
        "gh issue view 7 -c",
        "gh issue view 7 --json title,body,comments",
        "gh issue view 7 --json body --jq .body",
        "gh issue view 7 --json body -q '.body | length'",
        "gh issue view 7 --json title --template '{{.title}}'",
    ] {
        let verdict = tracker.ask(command);
        assert_eq!(verdict.behavior, "allow", "command: {command:?}");
    }
}

#[test]
fn a_tracker_call_with_a_standing_yes_leaves_no_request() {
    let tracker = Tracker::new("wpt-tracker-quiet");
    tracker.ask(COMMENT);
    assert_eq!(tracker.lead.requests(), 0);
    assert!(!tracker.lead.state().join("lead-inbox.txt").exists());
}

#[test]
fn a_write_on_another_issue_takes_the_grant_route() {
    let tracker = Tracker::new("wpt-other-issue");
    tracker.assert_grant_route(&[
        "gh issue comment 13 --body done",
        "gh issue comment 1 --body done",
        "gh issue comment 120 --body done",
        "gh issue comment 012 --body done",
        "gh issue edit 13 --body body",
        "gh issue comment 12 13 --body done",
        "gh issue edit 12 13 --body body",
        "gh issue comment --body done",
        "gh issue comment https://github.com/o/r/issues/12 --body done",
        "gh issue comment 12 -R o/r --body done",
        "gh issue comment 12 --repo o/r --body done",
        "gh issue comment 12 --repo=o/r --body done",
        "gh issue edit 12 -R o/r --body body",
        "gh issue view 12 -R o/r",
        "gh issue view o/r#12",
    ]);
}

#[test]
fn a_tracker_write_beyond_a_comment_or_a_body_takes_the_grant_route() {
    let tracker = Tracker::new("wpt-tracker-wider");
    tracker.assert_grant_route(&[
        "gh issue create --title t --body b",
        "gh issue close 12",
        "gh issue reopen 12",
        "gh issue delete 12 --yes",
        "gh issue transfer 12 o/r",
        "gh issue edit 12 --title t",
        "gh issue edit 12 --add-label bug",
        "gh issue edit 12 --body body --add-assignee @me",
        "gh issue edit 12 --body body --milestone m",
        "gh issue comment 12 --delete-last --yes",
        "gh issue comment 12 --web",
        "gh issue comment 12 --editor",
        "gh issue comment 12 --body",
        "gh issue view 12 --web",
        "gh issue list",
        "gh issue",
        "gh pr comment 12 --body done",
    ]);
}

#[test]
fn a_tracker_call_the_shell_would_rewrite_takes_the_grant_route() {
    let tracker = Tracker::new("wpt-tracker-shell");
    tracker.assert_grant_route(&[
        "gh issue comment 12 --body \"$(cat notes.md)\"",
        "gh issue comment 12 --body \"see `cat notes.md`\"",
        "gh issue comment 12 --body $BODY",
        "gh issue comment $N --body done",
        "gh issue comment 1{2,3} --body done",
        "gh issue comment 12 --body *",
        "gh issue comment 12 --body-file ~/notes.md",
        "gh issue comment 12 --body done && echo ok",
        "gh issue comment 12 --body done; gh issue close 12",
        "gh issue comment 12 --body done\ngh issue close 12",
        "gh issue comment 12 --body done | cat",
        "gh issue comment 12 --body done > out.txt",
        "gh issue comment 12 --body-file - < notes.md",
        "gh issue comment 12 --body done &",
        "gh issue comment 12 --body done #note",
        "gh issue comment 12 --body 'open",
        "gh issue comment 12 --body done\\\n --repo o/r",
        "GH_REPO=o/r gh issue comment 12 --body done",
        "command gh issue comment 12 --body done",
        "/usr/local/bin/gh issue comment 12 --body done",
        "gh issue comment 12 --body =ls",
        "(gh issue comment 12 --body done)",
    ]);
}

#[test]
fn a_tracker_call_from_a_worktree_of_the_repository_is_allowed() {
    let tracker = Tracker::new("wpt-worktree");
    let branch = "feature/issue-12-more";
    git(
        &tracker.repo,
        &["worktree", "add", "-q", "-b", branch, ".worktrees/issue-12"],
    );
    let worktree = tracker.repo.join(".worktrees/issue-12");
    assert_eq!(tracker.ask_from(&worktree, COMMENT).behavior, "allow");
}

#[test]
fn a_tracker_call_from_a_directory_inside_the_repository_is_allowed() {
    let tracker = Tracker::new("wpt-subdirectory");
    let inside = tracker.repo.join("src/deep");
    std::fs::create_dir_all(&inside).expect("create the subdirectory");
    assert_eq!(tracker.ask_from(&inside, COMMENT).behavior, "allow");
}

#[test]
fn a_tracker_call_from_another_clone_takes_the_grant_route() {
    let tracker = Tracker::new("wpt-other-clone");
    let source = tracker.repo.to_str().expect("a UTF-8 path");
    git(tracker.lead.tmp.path(), &["clone", "-q", source, "other"]);
    let other = tracker.lead.tmp.path().join("other");
    for command in [COMMENT, "gh issue edit 12 --body body", "gh issue view 12"] {
        let verdict = tracker.ask_from(&other, command);
        assert!(
            !verdict.id().is_empty(),
            "command: {command:?}: {}",
            verdict.message
        );
    }
}

#[test]
fn a_tracker_call_from_outside_any_repository_takes_the_grant_route() {
    let tracker = Tracker::new("wpt-no-repository");
    let verdict = tracker.ask_from(tracker.lead.tmp.path(), COMMENT);
    assert!(!verdict.id().is_empty(), "{}", verdict.message);
}
