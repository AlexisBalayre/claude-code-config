use crate::common::stdin;
use regex_lite::Regex;
use serde_json::Value;

fn chars(value: &Value) -> Option<usize> {
    match value {
        Value::Null => Some(0),
        Value::String(text) => Some(text.chars().count()),
        _ => None,
    }
}

fn text(value: &Value) -> usize {
    match value {
        Value::Array(blocks) => blocks
            .iter()
            .filter_map(|block| chars(&block["text"]))
            .sum(),
        other => chars(other).unwrap_or(0),
    }
}

// A shape the rule does not expect counts as 0, as the shell hook's `catch 0` does.
fn size(tool: &str, response: &Value) -> Option<usize> {
    let object = response.is_object();
    Some(match tool {
        "Bash" if !object && !response.is_null() => 0,
        "Bash" if response["isImage"] == true => 0,
        "Bash" => chars(&response["stdout"])? + chars(&response["stderr"])?,
        "Read" if !object => 0,
        "Read" => {
            let lines = match &response["file"]["numLines"] {
                Value::Null => 0,
                count => count.as_u64()? as usize,
            };
            chars(&response["file"]["content"])? + 7 * lines
        }
        "WebFetch" if !object => 0,
        "WebFetch" => chars(&response["result"])?,
        "Write" => 0,
        _ if object => {
            let names = match &response["filenames"] {
                Value::Null => 0,
                Value::Array(names) => {
                    let total: Option<usize> = names.iter().map(chars).sum();
                    total? + names.len().saturating_sub(1)
                }
                _ => return None,
            };
            text(&response["content"]) + names
        }
        _ => text(response),
    })
}

pub fn run() {
    let Some(input) = stdin() else { return };
    // A result under the lowest threshold cannot fit in less than this many payload bytes,
    // even a Read of empty lines (8 counted characters for 2 bytes of JSON).
    if input.len() < 2000 {
        return;
    }
    let Ok(payload) = serde_json::from_str::<Value>(&input) else {
        return;
    };
    let tool = payload["tool_name"].as_str().unwrap_or("");
    let n = size(tool, &payload["tool_response"]).unwrap_or(0);
    let command = payload["tool_input"]["command"].as_str().unwrap_or("");
    let bash = tool == "Bash";
    if n <= 10000 || (n <= 20000 && !(bash && command.contains("scripts/verify.sh"))) {
        return;
    }

    let bare_only = bash
        && {
            let unquoted = Regex::new(r#""[^"]*"|'[^']*'"#)
                .unwrap()
                .replace_all(command, "\"\"");
            Regex::new(
            r"(^|[;&|(\n])\s*((bash\s+)?([^\s;&|()]*/)?scripts/verify\.sh|(bash\s+)?([^\s;&|()]*/)?worker-git\.sh(\s|$)|git(\s+-C\s+\S+)?\s+push\b)",
        )
        .unwrap()
        .is_match(&unquoted)
        };
    let hint = if bare_only {
        "keep that command bare (a pipe breaks it in the worker sandbox) and read only the failing lines of its result"
    } else if bash {
        "pipe the output through `tail -n 40` or a summary `grep` (pass/fail counts, failing names)"
    } else if tool == "Read" {
        "read a part with `offset` and `limit`"
    } else {
        "narrow the query or ask for fewer fields"
    };
    let context = Value::String(format!(
        "That {tool} result was {n} characters. Next time, {hint}."
    ));
    println!(
        r#"{{"hookSpecificOutput":{{"hookEventName":"PostToolUse","additionalContext":{context}}}}}"#
    );
}
