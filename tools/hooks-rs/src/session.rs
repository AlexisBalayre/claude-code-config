use crate::common::{git, grep, payload, stdin, Config};
use regex_lite::Regex;
use std::collections::BTreeSet;
use std::io::IsTerminal;
use std::process::exit;

/// The `cwd` the hook payload names, or an empty string.
pub fn cwd() -> String {
    payload()
        .and_then(|payload| Some(payload["cwd"].as_str()?.trim_end_matches('\n').to_owned()))
        .unwrap_or_default()
}

/// Move to `$CLAUDE_PROJECT_DIR`, or to the git top level of the cwd when it is unset.
pub fn enter_project() {
    let dir = std::env::var("CLAUDE_PROJECT_DIR")
        .ok()
        .filter(|dir| !dir.is_empty())
        .or_else(|| git(".", &["rev-parse", "--show-toplevel"]));
    if dir.is_some_and(|dir| std::env::set_current_dir(dir).is_err()) {
        exit(0)
    }
}

pub fn stop_hook_active() -> bool {
    !std::io::stdin().is_terminal()
        && stdin()
            .is_some_and(|input| grep(&input, r#""stop_hook_active"[[:space:]]*:[[:space:]]*true"#))
}

/// Matches a path ending in one of SOURCE_EXTENSIONS; None when the list is empty.
pub fn source_regex(config: &Config) -> Option<Regex> {
    // As `tr -s ' ' '|'` builds the alternation: every space becomes a bar, runs of bars collapse.
    let mut extensions: Vec<char> = config
        .get("SOURCE_EXTENSIONS")
        .chars()
        .map(|c| if c == ' ' { '|' } else { c })
        .collect();
    extensions.dedup_by(|a, b| *a == '|' && *b == '|');
    if extensions.is_empty() {
        return None;
    }
    Regex::new(&format!(r"\.({})$", String::from_iter(extensions))).ok()
}

/// The sorted, distinct output lines of several git commands run in the cwd.
pub fn git_lines(commands: &[&[&str]]) -> BTreeSet<String> {
    commands
        .iter()
        .filter_map(|args| git(".", args))
        .flat_map(|out| {
            out.split_terminator('\n')
                .map(str::to_owned)
                .collect::<Vec<_>>()
        })
        .collect()
}
