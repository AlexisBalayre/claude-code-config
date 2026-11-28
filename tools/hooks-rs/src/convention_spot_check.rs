use crate::session::{enter_project, git_lines, stop_hook_active};
use regex_lite::Regex;
use std::path::Path;
use std::process::exit;

/// The escapes `echo -e` expands that a message plausibly holds.
fn unescape(text: &str) -> String {
    let mut out = String::new();
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some('r') => out.push('\r'),
            Some('a') => out.push('\x07'),
            Some('b') => out.push('\x08'),
            Some('e') => out.push('\x1b'),
            Some('f') => out.push('\x0c'),
            Some('v') => out.push('\x0b'),
            Some('\\') => out.push('\\'),
            Some(other) => out.extend(['\\', other]),
            None => out.push('\\'),
        }
    }
    out
}

pub fn run() {
    enter_project();
    let Ok(checks) = std::fs::read_to_string(".claude/spot-checks.tsv") else {
        return;
    };
    let is_check =
        |line: &str| !line.trim_start().is_empty() && !line.trim_start().starts_with('#');
    // On the re-run the hook is silent, so a finding the model judged a false positive cannot loop.
    if !checks.lines().any(is_check) || stop_hook_active() {
        return;
    }

    // A run of TABs is one separator, so columns may be aligned.
    let rules: Vec<(Regex, Regex, &str)> = checks
        .lines()
        .filter_map(|line| {
            let line = line.trim_matches('\t');
            let (path, rest) = line.split_once('\t').unwrap_or((line, ""));
            let rest = rest.trim_start_matches('\t');
            let (content, message) = rest.split_once('\t').unwrap_or((rest, ""));
            if path.is_empty() || path.starts_with('#') {
                return None;
            }
            Some((
                Regex::new(path).ok()?,
                Regex::new(content).ok()?,
                message.trim_start_matches('\t'),
            ))
        })
        .collect();

    let mut warnings = String::new();
    // Untracked files included: a freshly written file is where a violation most often lands.
    for file in git_lines(&[
        &["diff", "--name-only", "HEAD"],
        &["diff", "--cached", "--name-only"],
        &["ls-files", "--others", "--exclude-standard"],
    ]) {
        if !Path::new(&file).is_file() || !rules.iter().any(|(path, _, _)| path.is_match(&file)) {
            continue;
        }
        let bytes = std::fs::read(&file).unwrap_or_default();
        let text = String::from_utf8_lossy(&bytes);
        for (path, content, message) in &rules {
            if path.is_match(&file)
                && text
                    .split_terminator('\n')
                    .any(|line| content.is_match(line))
            {
                warnings += &unescape(&format!("  {file}: {message}\n"));
            }
        }
    }
    if warnings.is_empty() {
        return;
    }
    eprintln!(
        "\nConvention spot-check on files changed this session:\n{warnings}\nFix each real finding; if one is a heuristic misfire, say so and stop again (this check stays silent on the re-run)."
    );
    exit(2)
}
