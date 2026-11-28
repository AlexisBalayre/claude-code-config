use crate::common::{block, git, project_dir, stdin, Config};
use regex_lite::Regex;
use serde_json::Value;
use std::path::{Component, Path, PathBuf};

// Resolve `..` and symlinks even through directories a Write has yet to create.
fn realpath(file: &str) -> PathBuf {
    let absolute = std::env::current_dir().unwrap_or_default().join(file);
    let mut real = PathBuf::new();
    for part in absolute.components() {
        match part {
            Component::CurDir => {}
            Component::ParentDir => {
                real.pop();
            }
            part => {
                real.push(part);
                if real.is_symlink() {
                    if let Ok(target) = real.canonicalize() {
                        real = target;
                    }
                }
            }
        }
    }
    real
}

pub fn run() {
    let allow = std::env::var("AGENT_FROZEN_ALLOW").unwrap_or_default();
    if allow.is_empty() {
        return;
    }
    // The session's own profile, never the edited file's tree, which nobody may have vetted.
    let frozen = Config::load(project_dir()).get("FROZEN_PATHS_REGEX");
    if frozen.is_empty() {
        return;
    }

    let input = stdin().unwrap_or_default();
    if input.trim().is_empty() {
        return;
    }
    let payload: Option<Value> = serde_json::from_str(&input).ok();
    let path = payload
        .as_ref()
        .filter(|payload| payload.is_object())
        .map(|payload| &payload["tool_input"])
        .filter(|input| input.is_object() || input.is_null())
        .map(|input| {
            if payload.as_ref().unwrap()["tool_name"] == "NotebookEdit" {
                &input["notebook_path"]
            } else {
                &input["file_path"]
            }
        });
    let file = match path {
        Some(Value::String(file)) => file.clone(),
        Some(Value::Null | Value::Bool(false)) => return,
        Some(other) => other.to_string(),
        None => block("the hook could not read the edit's path. Stop and ask the user; don't edit it another way."),
    };
    if file.is_empty() {
        return;
    }
    // The allow-list is read line by line, so a newline in the path could match any line.
    if file.contains('\n') {
        block("the file path holds a newline.");
    }

    let real = realpath(&file);
    let mut dir = real.parent().unwrap_or(Path::new("/"));
    while !dir.is_dir() {
        dir = dir.parent().unwrap_or(Path::new("/"));
    }
    // git prints the prefix in its on-disk case; stripping the toplevel from the typed
    // path would miss a wrong-case parent directory.
    let Some(prefix) = git(dir, &["rev-parse", "--show-prefix"]) else {
        return;
    };
    let rel = format!(
        "{prefix}{}",
        real.strip_prefix(dir).unwrap_or(&real).to_string_lossy()
    );
    // A part not yet on disk keeps its typed case, and on a case-insensitive filesystem a
    // Write there still lands in the frozen tree. The allow-list line stays exact.
    if !Regex::new(&format!("(?i){frozen}")).is_ok_and(|re| re.is_match(&rel)) {
        return;
    }

    let branch = git(dir, &["branch", "--show-current"]).unwrap_or_default();
    let line = format!("{branch} {rel}");
    if std::fs::read_to_string(&allow).is_ok_and(|list| list.lines().any(|entry| entry == line)) {
        return;
    }
    block(&format!(
        "{rel} is frozen, and {allow} has no '{line}' line. Stop and ask the user for that line; don't edit it another way."
    ));
}
