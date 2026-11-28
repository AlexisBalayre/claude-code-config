use crate::common::{stdin, Config};
use regex_lite::Regex;
use serde_json::Value;
use std::path::Path;
use std::process::exit;

pub fn run() {
    let input = stdin().unwrap_or_default();
    if input.trim().is_empty() {
        return;
    }
    let payload: Value = serde_json::from_str(&input).unwrap_or_else(|_| exit(2));
    let file = payload["tool_input"]["file_path"].as_str().unwrap_or("");

    // Hooks are keyed on the directory the session started in, so a session working in a
    // sibling repo would otherwise hold that repo to this convention.
    let project = std::env::var("CLAUDE_PROJECT_DIR").unwrap_or_default();
    let Some(rel) = file
        .strip_prefix(&project)
        .filter(|_| !project.is_empty())
        .and_then(|rest| rest.strip_prefix('/'))
    else {
        return;
    };

    let config = Config::load(&project);
    let scope = config.get("FILE_NAMING_SCOPE_REGEX");
    let naming = config.get("FILE_NAMING_REGEX");
    // Overwriting an existing file doesn't choose a name, so legacy names stay writable.
    if scope.is_empty() || naming.is_empty() || Path::new(file).exists() {
        return;
    }
    if !Regex::new(&scope).is_ok_and(|re| re.is_match(rel)) {
        return;
    }
    let name = file
        .trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or(file);
    if Regex::new(&naming).is_ok_and(|re| re.is_match(name)) {
        return;
    }

    eprintln!(
        "BLOCKED: file name '{name}' does not match the project naming convention ({naming})."
    );
    let hint = config.get("FILE_NAMING_HINT");
    if !hint.is_empty() {
        eprintln!("{hint}");
    }
    eprintln!("Full naming rules: docs/conventions/core.md §Naming");
    exit(2)
}
