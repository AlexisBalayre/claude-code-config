mod common;

use common::{hook, run};
use serde_json::{json, Value};

const PIPE: &str = "tail -n 40";
const BARE: &str = "bare";

fn bash_out(size: usize) -> Value {
    json!({"stdout": "x".repeat(size), "stderr": "", "interrupted": false, "isImage": false})
}

fn read_text(size: usize) -> Value {
    json!({"type": "text", "file": {"filePath": "/f", "content": "x".repeat(size), "numLines": 1}})
}

fn stdout_for(tool: &str, command: &str, response: Value) -> String {
    let payload = json!({
        "tool_name": tool,
        "tool_input": {"command": command},
        "tool_response": response,
    });
    let out = run(&mut hook("tool-result-size"), &payload.to_string());
    assert_eq!(out.code, 0, "exit code for {tool} {command:?}");
    out.stdout
}

fn assert_silent(tool: &str, command: &str, response: Value) {
    let stdout = stdout_for(tool, command, response);
    assert!(
        stdout.is_empty(),
        "{tool} {command:?}: want silent, got {stdout}"
    );
}

fn assert_warns(want: &str, tool: &str, command: &str, response: Value) {
    let stdout = stdout_for(tool, command, response);
    let output: Value = serde_json::from_str(&stdout).unwrap_or(Value::Null);
    let hook_output = &output["hookSpecificOutput"];
    assert!(
        hook_output["hookEventName"] == "PostToolUse"
            && hook_output["additionalContext"]
                .as_str()
                .is_some_and(|context| context.contains(want)),
        "{tool} {command:?}: want a warning with {want:?}, got {stdout:?}"
    );
}

#[test]
fn read_result_at_the_threshold_stays_silent() {
    assert_silent("Read", "", read_text(19993));
}

#[test]
fn read_result_over_the_threshold_suggests_offset() {
    assert_warns("offset", "Read", "", read_text(19994));
}

#[test]
fn edit_result_counts_as_empty() {
    let response = json!({
        "filePath": "/f",
        "originalFile": "x".repeat(50000),
        "oldString": "a",
        "newString": "b",
        "structuredPatch": [],
    });
    assert_silent("Edit", "", response);
}

#[test]
fn bash_result_under_its_threshold_stays_silent() {
    assert_silent("Bash", "bash scripts/verify.sh", bash_out(10000));
    assert_silent("Bash", "make test", bash_out(12000));
}

#[test]
fn verify_result_warns_from_the_lower_threshold() {
    for command in [
        "bash scripts/verify.sh",
        "scripts/verify.sh",
        "cd .worktrees/x && scripts/verify.sh",
        "./scripts/verify.sh",
        "bash /abs/path/scripts/verify.sh",
        "cd x\nscripts/verify.sh",
    ] {
        assert_warns(BARE, "Bash", command, bash_out(12000));
    }
}

#[test]
fn gate_commands_are_told_to_stay_bare() {
    for command in [
        "bash /u/work/lead-tools/worker-git.sh merge x",
        "git push origin feature/x",
        "cd x && git -C x push origin feature/x",
        "cd x; git push origin feature/x",
        "  git push origin feature/x",
        "git status || git push origin x",
        "(git push origin x)",
        "git add . \\\n  && git push",
        "git commit -qm \"don't\" && git push origin x && echo 'ok'",
        "cat x | git push",
        "git -C \"$WT\" push origin feature/x",
    ] {
        assert_warns(BARE, "Bash", command, bash_out(25000));
    }
}

#[test]
fn other_bash_commands_are_told_to_pipe() {
    for command in [
        "make test",
        "scripts/worktree-create.sh x",
        "cat /u/work/lead-tools/worker-git.sh.bak",
        "bash /u/work/lead-tools/worker-git.sh.bak merge x",
        "grep -rn 'git push' docs",
        "echo \"git push\"",
        "grep -rnE 'scripts/verify.sh|git push' docs",
        "rg -n '(git push|worktree-create)' docs",
        "cat $(pwd)/scripts/verify.sh",
        "grep -rnE \"scripts/verify.sh|git push\" docs",
        "cat ./scripts/verify.sh",
        "head -n 80 /abs/scripts/verify.sh",
        "git log -p -- scripts/verify.sh",
    ] {
        assert_warns(PIPE, "Bash", command, bash_out(25000));
    }
}

#[test]
fn reading_the_verify_script_is_told_to_pipe() {
    assert_warns(PIPE, "Bash", "cat scripts/verify.sh", bash_out(12000));
}
