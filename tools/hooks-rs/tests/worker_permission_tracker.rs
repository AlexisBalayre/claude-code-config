mod common;

use common::{run, verdict, Lead, Verdict, WORKER};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

const COMMENT: &str = "gh issue comment 12 -R o/repo --body done";

/// Hold the checkout of the worker on issue 12, whose launcher named the repository `o/repo`.
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

    /// Ask the hook about a call of `tool` made from `cwd`.
    fn ask_as(&self, cwd: &Path, tool: &str, input: Value) -> Verdict {
        let payload = json!({"cwd": cwd, "tool_name": tool, "tool_input": input});
        let mut hook = self.lead.hook(WORKER);
        hook.env("AGENT_REPO", "o/repo");
        verdict(&run(&mut hook, &payload.to_string()))
    }

    fn ask(&self, command: &str) -> Verdict {
        self.ask_as(&self.repo, "Bash", json!({"command": command}))
    }

    /// Assert each call is refused with a request a grant can follow.
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
        "gh issue comment 12 -R o/repo --body done",
        "gh issue comment 12 --repo o/repo -b done",
        "gh issue comment 12 --repo=o/repo --body=done",
        "gh issue comment -R o/repo 12 --body done",
        "gh issue comment 12 --body done -R o/repo",
        "gh issue comment 12 -R o/repo -R o/repo --body done",
        "gh issue comment 12 -R o/repo --body 'Fixed in `src/x.rs`: costs $5; see #3 & more'",
        "gh issue comment 12 -R o/repo --body \"it's done, see (a) and [b]\"",
        "gh issue comment 12 -R o/repo --body \"a \\\"quoted\\\" word\"",
        "gh issue comment 12 -R o/repo --body 'first line\nsecond line'",
        "gh issue comment -R o/repo --body done 12",
        "gh issue comment 12 -R o/repo --edit-last --body done",
        "gh issue comment 12 -R o/repo --edit-last --create-if-none --body done",
        "gh issue comment 12 -R o/repo --body 'gh pr merge 5 is the lead to ask for'",
        "gh issue comment 12 -R o/repo --body '-R o/other'",
        " gh issue comment 12 -R o/repo --body done \n",
    ] {
        let verdict = tracker.ask(command);
        assert_eq!(verdict.behavior, "allow", "command: {command:?}");
    }
}

#[test]
fn a_new_body_for_the_workers_own_issue_is_allowed() {
    let tracker = Tracker::new("wpt-edit");
    for command in [
        "gh issue edit 12 -R o/repo --body 'the whole new body'",
        "gh issue edit 12 -R o/repo -b body",
        "gh issue edit --body=body --repo=o/repo 12",
    ] {
        let verdict = tracker.ask(command);
        assert_eq!(verdict.behavior, "allow", "command: {command:?}");
    }
}

#[test]
fn a_read_of_any_issue_of_the_repository_is_allowed() {
    let tracker = Tracker::new("wpt-view");
    for command in [
        "gh issue view 12 -R o/repo",
        "gh issue view 999 -R o/repo",
        "gh issue view 999 -R o/repo --comments",
        "gh issue view 7 -R o/repo -c",
        "gh issue view 7 -R o/repo --json title,body,comments",
        "gh issue view 7 -R o/repo --json body --jq .body",
        "gh issue view 7 -R o/repo --json body -q '.body | length'",
        "gh issue view 7 -R o/repo --json title --template '{{.title}}'",
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

// `-R` names the repository, so the remotes of the directory are never read.
#[test]
fn a_tracker_call_is_allowed_from_outside_the_checkout() {
    let tracker = Tracker::new("wpt-any-directory");
    let outside = tracker.lead.tmp.path();
    let verdict = tracker.ask_as(outside, "Bash", json!({"command": COMMENT}));
    assert_eq!(verdict.behavior, "allow");
}

#[test]
fn a_write_on_another_issue_takes_the_grant_route() {
    let tracker = Tracker::new("wpt-other-issue");
    tracker.assert_grant_route(&[
        "gh issue comment 13 -R o/repo --body done",
        "gh issue comment 1 -R o/repo --body done",
        "gh issue comment 120 -R o/repo --body done",
        "gh issue comment 012 -R o/repo --body done",
        "gh issue edit 13 -R o/repo --body body",
        "gh issue comment 12 13 -R o/repo --body done",
        "gh issue edit 12 13 -R o/repo --body body",
        "gh issue comment -R o/repo --body done",
        "gh issue comment https://github.com/o/repo/issues/12 -R o/repo --body done",
        "gh issue view o/repo#12 -R o/repo",
    ]);
}

#[test]
fn a_tracker_call_that_names_no_repository_or_another_takes_the_grant_route() {
    let tracker = Tracker::new("wpt-other-repository");
    tracker.assert_grant_route(&[
        "gh issue comment 12 --body done",
        "gh issue edit 12 --body body",
        "gh issue view 12",
        "gh issue comment 12 -R o/other --body done",
        "gh issue comment 12 --repo o/other --body done",
        "gh issue comment 12 --repo=o/other --body done",
        "gh issue comment 12 -R O/repo --body done",
        "gh issue comment 12 -R github.com/o/repo --body done",
        "gh issue comment 12 -R o/repo -R o/other --body done",
        "gh issue comment 12 -R o/other -R o/repo --body done",
        "gh issue comment 12 -R o/repo --repo=o/other --body done",
        "gh issue comment 12 -R=o/repo --body done",
        "gh issue comment 12 --body done -R",
        "gh issue edit 12 -R o/other --body body",
        "gh issue view 12 -R o/other",
    ]);
}

#[test]
fn a_session_the_launcher_named_no_repository_for_gets_no_tracker_allow() {
    let tracker = Tracker::new("wpt-no-repository");
    for (repo, command) in [
        (None, COMMENT),
        (Some(""), COMMENT),
        (Some(""), "gh issue comment 12 -R '' --body done"),
        (Some(""), "gh issue comment 12 --repo= --body done"),
    ] {
        let payload = json!({
            "cwd": &tracker.repo,
            "tool_name": "Bash",
            "tool_input": {"command": command},
        });
        let mut hook = tracker.lead.hook(WORKER);
        if let Some(repo) = repo {
            hook.env("AGENT_REPO", repo);
        }
        let verdict = verdict(&run(&mut hook, &payload.to_string()));
        assert!(
            !verdict.id().is_empty(),
            "repository: {repo:?}, command: {command:?}: {}",
            verdict.message
        );
    }
}

// `--body-file` would publish any file the worker can read.
#[test]
fn a_tracker_call_that_posts_a_file_takes_the_grant_route() {
    let tracker = Tracker::new("wpt-body-file");
    tracker.assert_grant_route(&[
        "gh issue comment 12 -R o/repo --body-file notes/status.md",
        "gh issue comment 12 -R o/repo -F notes/status.md",
        "gh issue comment 12 -R o/repo --body-file=notes/status.md",
        "gh issue comment 12 -R o/repo -F .env",
        "gh issue edit 12 -R o/repo --body-file notes/body.md",
        "gh issue edit 12 -R o/repo -F notes/body.md",
        "gh issue edit --body-file=notes/body.md -R o/repo 12",
    ]);
}

#[test]
fn a_tracker_write_beyond_a_comment_or_a_body_takes_the_grant_route() {
    let tracker = Tracker::new("wpt-tracker-wider");
    tracker.assert_grant_route(&[
        "gh issue create -R o/repo --title t --body b",
        "gh issue close 12 -R o/repo",
        "gh issue reopen 12 -R o/repo",
        "gh issue delete 12 -R o/repo --yes",
        "gh issue transfer 12 o/r -R o/repo",
        "gh issue edit 12 -R o/repo --title t",
        "gh issue edit 12 -R o/repo --add-label bug",
        "gh issue edit 12 -R o/repo --body body --add-assignee @me",
        "gh issue edit 12 -R o/repo --body body --milestone m",
        "gh issue comment 12 -R o/repo --delete-last --yes",
        "gh issue comment 12 -R o/repo --web",
        "gh issue comment 12 -R o/repo --editor",
        "gh issue comment 12 -R o/repo --edit-last=true --body done",
        "gh issue comment 12 -R o/repo --body",
        "gh issue view 12 -R o/repo --web",
        "gh issue list -R o/repo",
        "gh issue",
        "gh pr comment 12 -R o/repo --body done",
    ]);
}

#[test]
fn a_tracker_call_the_shell_would_rewrite_takes_the_grant_route() {
    let tracker = Tracker::new("wpt-tracker-shell");
    tracker.assert_grant_route(&[
        "gh issue comment 12 -R o/repo --body \"$(cat notes.md)\"",
        "gh issue comment 12 -R o/repo --body \"see `cat notes.md`\"",
        "gh issue comment 12 -R o/repo --body $BODY",
        "gh issue comment $N -R o/repo --body done",
        "gh issue comment 12 -R $AGENT_REPO --body done",
        "gh issue comment 1{2,3} -R o/repo --body done",
        "gh issue comment 12 -R o/repo --body *",
        "gh issue comment 12 -R o/repo --body ~/notes.md",
        "gh issue comment 12 -R o/repo --body done && echo ok",
        "gh issue comment 12 -R o/repo --body done; gh issue close 12",
        "gh issue comment 12 -R o/repo --body done\ngh issue close 12",
        "gh issue comment 12 -R o/repo --body done | cat",
        "gh issue comment 12 -R o/repo --body done > out.txt",
        "gh issue comment 12 -R o/repo --body done < notes.md",
        "gh issue comment 12 -R o/repo --body done &",
        "gh issue comment 12 -R o/repo --body done #note",
        "gh issue comment 12 -R o/repo --body 'open",
        "gh issue comment 12 -R o/repo --body done\\\n --repo o/other",
        "GH_REPO=o/other gh issue comment 12 -R o/repo --body done",
        "GH_HOST=example.com gh issue comment 12 -R o/repo --body done",
        "command gh issue comment 12 -R o/repo --body done",
        "/usr/local/bin/gh issue comment 12 -R o/repo --body done",
        "gh issue comment 12 -R o/repo --body =ls",
        "(gh issue comment 12 -R o/repo --body done)",
    ]);
}

#[test]
fn a_tracker_command_in_a_tool_that_is_not_bash_takes_the_grant_route() {
    let tracker = Tracker::new("wpt-not-bash");
    let verdict = tracker.ask_as(&tracker.repo, "mcp__x__run", json!({"command": COMMENT}));
    assert_eq!(verdict.behavior, "deny");
    assert!(!verdict.id().is_empty(), "{}", verdict.message);
}

#[test]
fn a_tracker_call_that_leaves_the_sandbox_takes_the_grant_route() {
    let tracker = Tracker::new("wpt-unsandboxed");
    for flag in [json!(true), json!("true"), json!(1)] {
        let input = json!({"command": COMMENT, "dangerouslyDisableSandbox": flag});
        let verdict = tracker.ask_as(&tracker.repo, "Bash", input);
        assert!(
            !verdict.id().is_empty(),
            "flag: {flag}: {}",
            verdict.message
        );
        assert_eq!(
            tracker.lead.request(verdict.id())["input"]["dangerouslyDisableSandbox"],
            flag
        );
    }
}

#[test]
fn a_tracker_call_that_keeps_the_sandbox_by_name_is_allowed() {
    let tracker = Tracker::new("wpt-sandboxed");
    let input = json!({"command": COMMENT, "dangerouslyDisableSandbox": false});
    let verdict = tracker.ask_as(&tracker.repo, "Bash", input);
    assert_eq!(verdict.behavior, "allow");
}
