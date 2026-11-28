mod common;

use common::{age, git, git_stdout, hook, run, verdict, write, Lead, WORKER};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

const DELETE_BRANCH: &str = "git branch -D feature/issue-9";

fn edit_payload(tool_input: &str) -> String {
    format!(r#"{{"cwd": "/x", "tool_name": "Edit", "tool_input": {tool_input}}}"#)
}

#[test]
fn a_session_that_is_no_worker_gets_no_decision() {
    let payload = json!({"tool_name": "Bash", "tool_input": {"command": "ls"}}).to_string();
    let out = run(&mut hook("worker-permission"), &payload);
    assert_eq!((out.code, out.stdout.as_str()), (0, ""));
}

#[test]
fn an_empty_agent_worker_gets_no_decision() {
    let payload = json!({"tool_name": "Bash", "tool_input": {"command": "ls"}}).to_string();
    let mut hook = hook("worker-permission");
    hook.env("AGENT_WORKER", "");
    let out = run(&mut hook, &payload);
    assert_eq!((out.code, out.stdout.as_str()), (0, ""));
}

#[test]
fn a_name_that_is_no_worker_name_is_denied_with_no_request() {
    let lead = Lead::new("wp-bad-name");
    for name in [
        "lead", "W5-12", "w5-", "-12", "5w-12", "w5-12a", "w5-12-3", "w5 -12", "w5-012", "w5-0",
    ] {
        let verdict = lead.ask_as(name, lead.tmp.path(), "Read", json!({"file_path": "/x"}));
        assert_eq!(verdict.behavior, "deny", "name: {name:?}");
        assert!(verdict.message.contains("AGENT_WORKER"), "name: {name:?}");
    }
    assert_eq!(lead.requests(), 0);
}

#[test]
fn an_unreadable_payload_is_denied_with_a_reason() {
    let lead = Lead::new("wp-unreadable");
    let verdict = verdict(&run(&mut lead.hook(WORKER), "{not json"));
    assert_eq!(verdict.behavior, "deny");
    assert!(verdict.message.contains("unreadable request"));
}

#[test]
fn a_state_directory_that_cannot_be_written_is_denied_with_a_reason() {
    let lead = Lead::new("wp-no-state");
    write(&lead.state(), "a file where the directory should be\n");
    let verdict = lead.bash(lead.tmp.path(), DELETE_BRANCH);
    assert_eq!(verdict.behavior, "deny");
    assert!(verdict.message.contains("could not record the request"));
}

#[test]
fn with_no_agent_work_the_state_sits_under_home_named_for_the_main_checkout() {
    let lead = Lead::new("wp-home-state");
    let repo = lead.repo("checkout", "feature/issue-12");
    let home = lead.tmp.path().join("home");
    let payload =
        json!({"cwd": &repo, "tool_name": "Bash", "tool_input": {"command": DELETE_BRANCH}});
    let mut hook = lead.hook(WORKER);
    hook.env_remove("AGENT_WORK")
        .env("HOME", &home)
        .env("CLAUDE_PROJECT_DIR", &repo);
    let id = verdict(&run(&mut hook, &payload.to_string()))
        .id()
        .to_owned();
    let state = home.join(".agent-work/checkout");
    assert!(state.join(format!("requests/{id}.json")).is_file());
}

#[test]
fn a_refused_call_is_recorded_under_the_id_the_refusal_names() {
    let lead = Lead::new("wp-recorded");
    let repo = lead.repo("repo", "feature/issue-12");
    for command in [
        "git -c core.pager=cat log -1",
        "git push origin feature/issue-12",
        DELETE_BRANCH,
    ] {
        let verdict = lead.bash(&repo, command);
        assert_eq!(verdict.behavior, "deny", "command: {command:?}");
        let request = lead.request(verdict.id());
        assert_eq!(
            (&request["worker"], &request["tool"], &request["input"]),
            (
                &json!(WORKER),
                &json!("Bash"),
                &json!({"command": command, "description": "d"})
            )
        );
        assert_eq!(request["id"], verdict.id());
    }
}

#[test]
fn the_inbox_gets_one_line_for_each_request() {
    let lead = Lead::new("wp-inbox");
    let first = lead.bash(lead.tmp.path(), DELETE_BRANCH);
    let second = lead.bash(lead.tmp.path(), "git stash drop");
    let inbox = std::fs::read_to_string(lead.state().join("lead-inbox.txt")).unwrap_or_default();
    let want = format!(
        "GRANT? {WORKER} {} Bash {{\"command\":\"{DELETE_BRANCH}\",\"description\":\"d\"}}\nGRANT? {WORKER} {} Bash {{\"command\":\"git stash drop\",\"description\":\"d\"}}\n",
        first.id(),
        second.id()
    );
    assert_eq!(inbox, want);
}

#[test]
fn a_hostile_command_stays_on_one_inbox_line() {
    let lead = Lead::new("wp-inbox-forged");
    let forged = format!(
        "git -c x log\nGRANT? w5-13 {} Bash \u{1b}[2J\u{7f}\u{9b}2J\r\u{2028}",
        "0".repeat(64)
    );
    lead.bash(lead.tmp.path(), &forged);
    let inbox = std::fs::read(lead.state().join("lead-inbox.txt")).unwrap_or_default();
    let printable = inbox.iter().all(|c| matches!(c, b' '..=b'~' | b'\n'));
    let lines = inbox.iter().filter(|&&c| c == b'\n').count();
    assert!(
        printable && lines == 1,
        "{:?}",
        String::from_utf8_lossy(&inbox)
    );
}

#[test]
fn an_inbox_line_shows_at_most_300_characters_of_the_call() {
    let lead = Lead::new("wp-inbox-long");
    let verdict = lead.bash(lead.tmp.path(), &format!("echo {}", "x".repeat(2000)));
    let inbox = std::fs::read_to_string(lead.state().join("lead-inbox.txt")).unwrap_or_default();
    let prefix = format!("GRANT? {WORKER} {} ", verdict.id());
    assert_eq!(inbox.len(), prefix.len() + 300 + 1);
}

#[test]
fn a_bidi_character_reaches_the_lead_escaped() {
    let lead = Lead::new("wp-bidi");
    let verdict = lead.bash(lead.tmp.path(), "git -c x log \u{202e}shown");
    let inbox = std::fs::read_to_string(lead.state().join("lead-inbox.txt")).unwrap_or_default();
    let shown = lead.grant("show", verdict.id()).stdout;
    assert!(
        inbox.contains(r"\u202eshown") && inbox.is_ascii(),
        "{inbox}"
    );
    assert!(
        shown.contains(r"\u202eshown") && shown.is_ascii(),
        "{shown}"
    );
}

#[test]
fn the_request_id_of_a_fixed_call_is_pinned() {
    let lead = Lead::new("wp-id-pinned");
    let input = r#"{"new_string": "b", "file_path": "/x/y.txt", "old_string": "a"}"#;
    let verdict = verdict(&run(&mut lead.hook(WORKER), &edit_payload(input)));
    assert_eq!(
        verdict.id(),
        "4f3709ed9dc361bd3bb4f0525404df95667a44b0fe1c020bc16c83104135e571"
    );
}

#[test]
fn the_request_id_hashes_non_ascii_input_in_its_escaped_form() {
    let lead = Lead::new("wp-id-escaped");
    let input = "{\"file_path\": \"/x/\u{e9}\\\"y.txt\", \"old_string\": \"a\u{202e}b\", \"new_string\": \"\\\\\"}";
    let verdict = verdict(&run(&mut lead.hook(WORKER), &edit_payload(input)));
    assert_eq!(
        verdict.id(),
        "b8654940de4490cdf8b8a1e567a29afcc53b8d7c657e1ccabdbc54de154d0c6f"
    );
}

#[test]
fn the_request_id_ignores_the_order_of_the_inputs_keys() {
    let lead = Lead::new("wp-id-key-order");
    let ids = [
        r#"{"file_path": "/x/y.txt", "old_string": "a", "new_string": "b"}"#,
        r#"{"new_string": "b", "old_string": "a", "file_path": "/x/y.txt"}"#,
    ]
    .map(|input| {
        verdict(&run(&mut lead.hook(WORKER), &edit_payload(input)))
            .id()
            .to_owned()
    });
    assert!(!ids[0].is_empty() && ids[0] == ids[1], "{ids:?}");
}

#[test]
fn the_request_id_of_a_bash_call_is_the_digest_of_worker_tool_input_directory_head_and_files() {
    let lead = Lead::new("wp-id-bash");
    let repo = lead.repo("repo", "feature/issue-12");
    let head = git_stdout(&repo, &["rev-parse", "HEAD"]);
    let verdict = lead.bash(&repo, "ls -la /tmp");
    let hashed = format!(
        "{WORKER}\nBash\n{}\n{}\n{head}\n{:x}",
        r#"{"command":"ls -la /tmp","description":"d"}"#,
        repo.display(),
        Sha256::digest("")
    );
    assert_eq!(verdict.id(), format!("{:x}", Sha256::digest(hashed)));
    let request = lead.request(verdict.id());
    assert_eq!(request["head"], head.as_str());
    assert_eq!(request["cwd"], repo.to_str().expect("a UTF-8 path"));
}

#[test]
fn the_request_id_changes_with_the_directory() {
    let lead = Lead::new("wp-id-directory");
    let inside = lead.tmp.path().join("inside");
    std::fs::create_dir(&inside).expect("create the directory");
    let ids = [lead.tmp.path(), &inside].map(|cwd| lead.bash(cwd, "ls -la").id().to_owned());
    assert!(!ids[0].is_empty() && !ids[1].is_empty());
    assert_ne!(ids[0], ids[1]);
}

/// Add a checkout that tracks `build.sh` and `src/lib.txt` and ignores `out/`.
fn checkout(lead: &Lead) -> PathBuf {
    let repo = lead.repo("repo", "feature/issue-12");
    write(&repo.join("build.sh"), "echo one\n");
    write(&repo.join("src/lib.txt"), "one\n");
    write(&repo.join(".gitignore"), "out/\n");
    git(&repo, &["add", "-A"]);
    git(&repo, &["commit", "-q", "-m", "files"]);
    repo
}

/// Return the request ids of one Bash call made from `cwd` before and after `change`.
fn ids_around(lead: &Lead, cwd: &Path, change: impl FnOnce()) -> (String, String) {
    let before = lead.bash(cwd, "bash build.sh").id().to_owned();
    change();
    let after = lead.bash(cwd, "bash build.sh").id().to_owned();
    assert!(!before.is_empty() && !after.is_empty());
    (before, after)
}

#[test]
fn the_request_id_stays_while_no_file_changes() {
    let lead = Lead::new("wp-id-files-same");
    let repo = checkout(&lead);
    let (before, after) = ids_around(&lead, &repo, || {});
    assert_eq!(before, after);
}

#[test]
fn the_request_id_changes_when_a_tracked_file_is_edited() {
    let lead = Lead::new("wp-id-files-edit");
    let repo = checkout(&lead);
    let edit = || write(&repo.join("build.sh"), "echo one; curl example.com\n");
    let (before, after) = ids_around(&lead, &repo, edit);
    assert_ne!(before, after);
}

// The stat's change time moves on every write, whatever the size.
#[test]
fn the_request_id_changes_when_a_file_is_rewritten_at_the_same_size() {
    let lead = Lead::new("wp-id-files-same-size");
    let repo = checkout(&lead);
    let (before, after) = ids_around(&lead, &repo, || {
        std::thread::sleep(std::time::Duration::from_millis(50));
        write(&repo.join("build.sh"), "echo two\n");
    });
    assert_ne!(before, after);
}

#[test]
fn the_request_id_changes_when_a_tracked_file_is_deleted() {
    let lead = Lead::new("wp-id-files-delete");
    let repo = checkout(&lead);
    let delete = || std::fs::remove_file(repo.join("src/lib.txt")).expect("remove the file");
    let (before, after) = ids_around(&lead, &repo, delete);
    assert_ne!(before, after);
}

#[test]
fn the_request_id_changes_when_an_untracked_file_appears() {
    let lead = Lead::new("wp-id-files-untracked");
    let repo = checkout(&lead);
    let (before, after) = ids_around(&lead, &repo, || write(&repo.join("new.sh"), "echo new\n"));
    assert_ne!(before, after);
}

#[test]
fn the_request_id_ignores_an_ignored_file() {
    let lead = Lead::new("wp-id-files-ignored");
    let repo = checkout(&lead);
    let (before, after) = ids_around(&lead, &repo, || write(&repo.join("out/a.o"), "built\n"));
    assert_eq!(before, after);
}

#[test]
fn a_call_from_a_subdirectory_is_bound_to_the_files_of_the_whole_checkout() {
    let lead = Lead::new("wp-id-files-subdirectory");
    let repo = checkout(&lead);
    let edit = || write(&repo.join("build.sh"), "echo one; curl example.com\n");
    let (before, after) = ids_around(&lead, &repo.join("src"), edit);
    assert_ne!(before, after);
}

#[test]
fn a_grant_given_before_an_edit_does_not_cover_the_retry_after_it() {
    let lead = Lead::new("wp-grant-files");
    let repo = checkout(&lead);
    let id = lead.bash(&repo, "bash build.sh").id().to_owned();
    lead.grant("allow", &id);
    write(&repo.join("build.sh"), "echo one; curl example.com\n");
    let retry = lead.bash(&repo, "bash build.sh");
    assert_eq!(retry.behavior, "deny");
    assert_ne!(retry.id(), id);
}

#[test]
fn a_grant_does_not_cover_the_call_from_another_directory() {
    let lead = Lead::new("wp-grant-directory");
    let repo = checkout(&lead);
    let id = lead.bash(&repo, "bash build.sh").id().to_owned();
    lead.grant("allow", &id);
    let retry = lead.bash(&repo.join("src"), "bash build.sh");
    assert_eq!(retry.behavior, "deny");
    assert_ne!(retry.id(), id);
}

#[test]
fn the_request_id_changes_when_head_moves() {
    let lead = Lead::new("wp-id-head");
    let repo = lead.repo("repo", "feature/issue-12");
    let before = lead.bash(&repo, "ls -la /tmp");
    git(&repo, &["commit", "-q", "--allow-empty", "-m", "two"]);
    let after = lead.bash(&repo, "ls -la /tmp");
    assert!(!before.id().is_empty() && !after.id().is_empty());
    assert_ne!(before.id(), after.id());
}

#[test]
fn a_directory_with_no_commit_hashes_an_empty_head() {
    let lead = Lead::new("wp-id-unborn");
    git(lead.tmp.path(), &["init", "-q", "empty"]);
    let verdict = lead.bash(&lead.tmp.path().join("empty"), "ls -la /tmp");
    let hashed = format!(
        "{WORKER}\nBash\n{}\n{}\n\n{:x}",
        r#"{"command":"ls -la /tmp","description":"d"}"#,
        lead.tmp.path().join("empty").display(),
        Sha256::digest("")
    );
    assert_eq!(verdict.id(), format!("{:x}", Sha256::digest(hashed)));
    assert!(lead.request(verdict.id())["head"].is_null());
}

#[test]
fn a_file_tool_call_is_bound_to_neither_head_nor_the_files() {
    let lead = Lead::new("wp-id-file-tool");
    let repo = lead.repo("repo", "feature/issue-12");
    let input = json!({"file_path": "/outside/a.md", "content": "x"});
    let before = lead.ask_as(WORKER, &repo, "Write", input.clone());
    git(&repo, &["commit", "-q", "--allow-empty", "-m", "two"]);
    write(&repo.join("new.sh"), "echo new\n");
    let after = lead.ask_as(WORKER, &repo, "Write", input);
    assert!(!before.id().is_empty() && before.id() == after.id());
}

#[test]
fn a_granted_call_is_allowed() {
    let lead = Lead::new("wp-granted");
    let id = lead.bash(lead.tmp.path(), DELETE_BRANCH).id().to_owned();
    lead.grant("allow", &id);
    assert_eq!(lead.bash(lead.tmp.path(), DELETE_BRANCH).behavior, "allow");
}

#[test]
fn a_grant_is_spent_by_its_first_use() {
    let lead = Lead::new("wp-grant-once");
    let id = lead.bash(lead.tmp.path(), DELETE_BRANCH).id().to_owned();
    lead.grant("allow", &id);
    lead.bash(lead.tmp.path(), DELETE_BRANCH);
    let second = lead.bash(lead.tmp.path(), DELETE_BRANCH);
    assert_eq!(
        (second.behavior.as_str(), second.id()),
        ("deny", id.as_str())
    );
}

#[test]
fn of_racing_retries_only_one_takes_the_grant() {
    let lead = Lead::new("wp-grant-race");
    let id = lead.bash(lead.tmp.path(), DELETE_BRANCH).id().to_owned();
    lead.grant("allow", &id);
    let allowed = std::thread::scope(|scope| {
        let retries: Vec<_> = (0..8)
            .map(|_| scope.spawn(|| lead.bash(lead.tmp.path(), DELETE_BRANCH).behavior))
            .collect();
        retries
            .into_iter()
            .filter_map(|retry| retry.join().ok())
            .filter(|behavior| behavior == "allow")
            .count()
    });
    assert_eq!(allowed, 1);
}

#[test]
fn a_grant_does_not_cover_a_changed_call() {
    let lead = Lead::new("wp-grant-changed");
    let id = lead.bash(lead.tmp.path(), DELETE_BRANCH).id().to_owned();
    lead.grant("allow", &id);
    let changed = lead.bash(lead.tmp.path(), "git branch -D feature/issue-8");
    assert_eq!(changed.behavior, "deny");
}

#[test]
fn a_grant_does_not_cover_another_workers_identical_call() {
    let lead = Lead::new("wp-grant-other-worker");
    let id = lead.bash(lead.tmp.path(), DELETE_BRANCH).id().to_owned();
    lead.grant("allow", &id);
    let input = json!({"command": DELETE_BRANCH, "description": "d"});
    let other = lead.ask_as("w5-13", lead.tmp.path(), "Bash", input);
    assert_eq!(other.behavior, "deny");
    assert_ne!(other.id(), id);
}

#[test]
fn a_grant_file_naming_another_worker_is_refused() {
    let lead = Lead::new("wp-grant-misnamed");
    let id = lead.bash(lead.tmp.path(), DELETE_BRANCH).id().to_owned();
    write(&lead.state().join("grants").join(&id), "w5-13\n");
    assert_eq!(lead.bash(lead.tmp.path(), DELETE_BRANCH).behavior, "deny");
}

#[test]
fn a_grant_31_minutes_old_is_refused() {
    let lead = Lead::new("wp-grant-stale");
    let id = lead.bash(lead.tmp.path(), DELETE_BRANCH).id().to_owned();
    lead.grant("allow", &id);
    age(&lead.state().join("grants").join(&id), 31);
    assert_eq!(lead.bash(lead.tmp.path(), DELETE_BRANCH).behavior, "deny");
}

#[test]
fn a_grant_25_minutes_old_is_taken() {
    let lead = Lead::new("wp-grant-recent");
    let id = lead.bash(lead.tmp.path(), DELETE_BRANCH).id().to_owned();
    lead.grant("allow", &id);
    age(&lead.state().join("grants").join(&id), 25);
    assert_eq!(lead.bash(lead.tmp.path(), DELETE_BRANCH).behavior, "allow");
}

#[test]
fn a_grant_given_before_a_commit_does_not_cover_the_retry_after_it() {
    let lead = Lead::new("wp-grant-head");
    let repo = lead.repo("repo", "feature/issue-12");
    let hidden = "bash -c 'git push origin feature/issue-12'";
    let id = lead.bash(&repo, hidden).id().to_owned();
    lead.grant("allow", &id);
    git(&repo, &["commit", "-q", "--allow-empty", "-m", "two"]);
    let retry = lead.bash(&repo, hidden);
    assert_eq!(retry.behavior, "deny");
    assert_ne!(retry.id(), id);
}
