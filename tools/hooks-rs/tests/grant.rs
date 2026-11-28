mod common;

use common::{age, hook, run, write, Lead, WORKER};
use std::path::PathBuf;

const WINDOW: &str = "past the 30 minute window";

/// Hold a state directory with one request, recorded by the hook for a refused Bash call.
struct Requested {
    lead: Lead,
    id: String,
}

impl Requested {
    fn new(test: &str) -> Requested {
        let lead = Lead::new(test);
        let id = lead.bash(lead.tmp.path(), "git branch -D feature/issue-9");
        let id = id.id().to_owned();
        Requested { lead, id }
    }

    /// Record a request for another call and return its id.
    fn another(&self, command: &str) -> String {
        self.lead
            .bash(self.lead.tmp.path(), command)
            .id()
            .to_owned()
    }

    fn request_file(&self, id: &str) -> PathBuf {
        self.lead.state().join(format!("requests/{id}.json"))
    }

    fn grant_file(&self) -> PathBuf {
        self.lead.state().join("grants").join(&self.id)
    }
}

#[test]
fn show_prints_the_hooks_record_of_the_call() {
    let requested = Requested::new("grant-show");
    let out = requested.lead.grant("show", &requested.id);
    let shown: serde_json::Value = serde_json::from_str(&out.stdout).unwrap_or_default();
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert_eq!(
        shown,
        serde_json::json!({
            "id": requested.id,
            "worker": WORKER,
            "tool": "Bash",
            "input": {"command": "git branch -D feature/issue-9", "description": "d"},
        })
    );
}

#[test]
fn show_leaves_the_request_for_allow() {
    let requested = Requested::new("grant-show-keeps");
    requested.lead.grant("show", &requested.id);
    assert!(requested.request_file(&requested.id).is_file());
    assert!(!requested.grant_file().exists());
}

#[test]
fn show_prints_only_printable_ascii_whatever_the_record_holds() {
    let requested = Requested::new("grant-show-escapes");
    let record = format!(
        "{{\"id\": \"{}\", \"worker\": \"{WORKER}\", \"tool\": \"Bash\", \"input\": {{\"command\": \"a\u{202e}b\u{7f}\u{9b}2J\"}}}}",
        requested.id
    );
    write(&requested.request_file(&requested.id), &record);
    let out = requested.lead.grant("show", &requested.id);
    assert!(
        out.stdout.contains(r"a\u202eb\u007f\u009b2J"),
        "{}",
        out.stdout
    );
    assert!(
        out.stdout.bytes().all(|c| matches!(c, b' '..=b'~' | b'\n')),
        "{}",
        out.stdout
    );
}

#[test]
fn allow_writes_a_grant_that_names_the_worker() {
    let requested = Requested::new("grant-allow");
    let out = requested.lead.grant("allow", &requested.id);
    let grant = std::fs::read_to_string(requested.grant_file()).unwrap_or_default();
    assert_eq!((out.code, grant.as_str()), (0, "w5-12\n"), "{}", out.stderr);
    assert!(out.stdout.contains(&requested.id) && out.stdout.contains(WORKER));
}

#[test]
fn allow_drops_the_request_so_it_cannot_be_granted_twice() {
    let requested = Requested::new("grant-allow-once");
    requested.lead.grant("allow", &requested.id);
    let again = requested.lead.grant("allow", &requested.id);
    assert!(!requested.request_file(&requested.id).exists());
    assert_eq!(again.code, 1);
}

#[test]
fn allow_appends_the_granted_call_to_the_log() {
    let requested = Requested::new("grant-allow-log");
    let second = requested.another("git stash drop");
    requested.lead.grant("allow", &requested.id);
    requested.lead.grant("allow", &second);
    let log =
        std::fs::read_to_string(requested.lead.state().join("grants.log")).unwrap_or_default();
    let lines: Vec<&str> = log.lines().collect();
    assert_eq!(lines.len(), 2, "{log}");
    assert!(
        lines[0].contains(&format!("\"id\":\"{}\"", requested.id))
            && lines[0].contains("git branch -D feature/issue-9")
            && lines[1].contains(&format!("\"id\":\"{second}\"")),
        "{log}"
    );
}

#[test]
fn a_grant_that_cannot_be_logged_is_not_given() {
    let requested = Requested::new("grant-no-log");
    std::fs::create_dir_all(requested.lead.state().join("grants.log")).expect("block the log");
    let out = requested.lead.grant("allow", &requested.id);
    assert_eq!(out.code, 1);
    assert!(!requested.grant_file().exists());
    assert!(requested.request_file(&requested.id).is_file());
}

#[test]
fn a_request_31_minutes_old_is_refused_and_pruned() {
    for verb in ["show", "allow"] {
        let requested = Requested::new(&format!("grant-stale-{verb}"));
        age(&requested.request_file(&requested.id), 31);
        let out = requested.lead.grant(verb, &requested.id);
        assert_eq!(out.code, 1, "verb: {verb}");
        assert!(out.stderr.contains(WINDOW), "verb: {verb}: {}", out.stderr);
        assert!(
            !requested.request_file(&requested.id).exists(),
            "verb: {verb}"
        );
        assert!(!requested.grant_file().exists(), "verb: {verb}");
    }
}

#[test]
fn a_pruned_id_is_reported_as_possibly_past_the_window() {
    let requested = Requested::new("grant-pruned");
    age(&requested.request_file(&requested.id), 31);
    requested.lead.grant("show", &requested.id);
    let out = requested.lead.grant("allow", &requested.id);
    assert_eq!(out.code, 1);
    assert!(
        out.stderr.contains(&format!("pruned {WINDOW}")),
        "{}",
        out.stderr
    );
}

#[test]
fn a_request_25_minutes_old_is_granted() {
    let requested = Requested::new("grant-recent");
    age(&requested.request_file(&requested.id), 25);
    let out = requested.lead.grant("allow", &requested.id);
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert!(requested.grant_file().is_file());
}

#[test]
fn a_retry_of_the_call_renews_the_requests_window() {
    let requested = Requested::new("grant-renewed");
    age(&requested.request_file(&requested.id), 31);
    requested.another("git branch -D feature/issue-9");
    assert_eq!(requested.lead.grant("allow", &requested.id).code, 0);
}

#[test]
fn the_prune_takes_every_stale_request_and_no_other_file() {
    let requested = Requested::new("grant-prune");
    let stale = requested.another("git stash drop");
    let note = requested.lead.state().join("requests/note.txt");
    write(&note, "not a request\n");
    age(&requested.request_file(&stale), 31);
    age(&note, 31);
    requested.lead.grant("show", &requested.id);
    assert!(!requested.request_file(&stale).exists());
    assert!(note.is_file());
    assert!(requested.request_file(&requested.id).is_file());
}

#[test]
fn an_id_the_hook_never_recorded_is_refused() {
    let requested = Requested::new("grant-unknown");
    let out = requested.lead.grant("allow", &"a".repeat(64));
    assert_eq!(out.code, 1);
    assert!(out.stderr.contains("no request"), "{}", out.stderr);
}

#[test]
fn an_id_that_is_not_the_whole_digest_is_a_usage_error() {
    let requested = Requested::new("grant-short-id");
    for id in [
        &requested.id[..12],
        "../../x",
        &requested.id.to_uppercase(),
        &format!("{}0", requested.id),
    ] {
        let out = requested.lead.grant("allow", id);
        assert_eq!(out.code, 2, "id: {id:?}");
        assert!(out.stderr.contains("usage:"), "id: {id:?}");
    }
    assert!(!requested.lead.state().join("grants").exists());
}

#[test]
fn a_verb_other_than_show_or_allow_is_a_usage_error() {
    let requested = Requested::new("grant-verb");
    for args in [
        vec!["deny", requested.id.as_str()],
        vec!["allow"],
        vec!["allow", requested.id.as_str(), "extra"],
        vec![],
    ] {
        let mut command = hook("grant");
        command
            .args(&args)
            .env("AGENT_WORK", requested.lead.state());
        let out = run(&mut command, "");
        assert_eq!(out.code, 2, "args: {args:?}");
    }
}

#[test]
fn a_request_file_that_is_not_a_record_is_refused() {
    let requested = Requested::new("grant-corrupt");
    write(&requested.request_file(&requested.id), "{not json\n");
    let out = requested.lead.grant("allow", &requested.id);
    assert_eq!(out.code, 1);
    assert!(out.stderr.contains("unreadable"), "{}", out.stderr);
    assert!(!requested.grant_file().exists());
}

#[test]
fn with_no_state_directory_to_find_grant_is_a_tool_error() {
    let requested = Requested::new("grant-no-state");
    let mut command = hook("grant");
    command
        .args(["show", &requested.id])
        .current_dir(requested.lead.tmp.path());
    assert_eq!(run(&mut command, "").code, 2);
}
