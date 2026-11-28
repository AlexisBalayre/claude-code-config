use crate::common::{git, Config};
use crate::session::{enter_project, git_lines, source_regex, stop_hook_active};
use regex_lite::Regex;
use std::collections::BTreeSet;
use std::path::Path;
use std::process::{exit, Command, Stdio};

fn is_space(byte: u8) -> bool {
    byte == b' ' || (b'\t'..=b'\r').contains(&byte)
}

/// The comment text on a source line, trimmed; empty when the line holds none.
///
/// Covers full-line and trailing `// ...`, full-line and trailing or mid-line `/* ... */`,
/// `*` doc-continuation lines, and `# ...` (hash then space or end of line, so `#include`,
/// `#[attr]` and `#private` fields don't count). The earliest marker on a line wins, so a
/// trailing block comment is not missed. URLs are stripped first so `https://` is not read
/// as a `//`. The scan also matches `/*` inside a string literal (a `**/*.ts` glob): the
/// cost is one spurious dispatch bounded by the memo, whereas a guard precise enough to
/// skip it would start missing real no-space trailing block comments.
fn comment(line: &[u8]) -> Vec<u8> {
    let indent = line.iter().take_while(|byte| is_space(**byte)).count();
    let body = &line[indent..];
    let text = if body.starts_with(b"/*")
        || body.starts_with(b"*")
        || body == b"#"
        || body.starts_with(b"# ")
    {
        body.to_vec()
    } else {
        let mut code = Vec::new();
        let mut at = 0;
        while at < line.len() {
            if !line[at].is_ascii_alphabetic() {
                code.push(line[at]);
                at += 1;
                continue;
            }
            let scheme = |byte: &&u8| byte.is_ascii_alphanumeric() || b"+.-".contains(byte);
            let end = at + line[at..].iter().take_while(scheme).count();
            if line[end..].starts_with(b"://") {
                at = end + 3;
            } else {
                code.extend(&line[at..end]);
                at = end;
            }
        }
        let find = |marker: &[u8]| code.windows(2).position(|pair| pair == marker);
        let hash = (1..code.len()).find(|&at| {
            code[at] == b'#'
                && is_space(code[at - 1])
                && matches!(code.get(at + 1), None | Some(b' '))
        });
        match [find(b"//"), find(b"/*"), hash].into_iter().flatten().min() {
            Some(marker) => code.split_off(marker),
            None => Vec::new(),
        }
    };
    let end = text
        .iter()
        .rposition(|byte| !is_space(*byte))
        .map_or(0, |last| last + 1);
    let start = text
        .iter()
        .take_while(|byte| is_space(**byte))
        .count()
        .min(end);
    text[start..end].to_vec()
}

pub fn run(args: &[String]) {
    enter_project();
    let Some(git_dir) = git(".", &["rev-parse", "--git-dir"]) else {
        return;
    };
    let memo = Path::new(&git_dir).join("comment-pruner-memo");
    // No commit yet: the memo's first line is empty as well, so a seal still holds.
    let head = git(".", &["rev-parse", "HEAD"]).unwrap_or_default();

    let config = Config::load(".");
    let Some(source) = source_regex(&config) else {
        return;
    };
    // Generated files and vendored or build dirs carry comments the agent must never touch;
    // the agent re-applies this filter, but excluding here avoids dispatching for them at all.
    let mut exclude = String::from(r"(^|/)(node_modules|vendor|dist|build|target|\.venv|archive)/");
    let generated = config.get("GENERATED_PATHS_REGEX");
    if !generated.is_empty() {
        exclude = format!("{exclude}|{generated}");
    }
    let exclude = Regex::new(&exclude).ok();
    let wanted = |file: &String| {
        source.is_match(file) && exclude.as_ref().is_some_and(|re| !re.is_match(file))
    };
    let tracked: Vec<String> = git_lines(&[&["diff", "HEAD", "--name-only"]])
        .into_iter()
        .filter(wanted)
        .collect();
    let untracked: Vec<String> = git_lines(&[&["ls-files", "--others", "--exclude-standard"]])
        .into_iter()
        .filter(wanted)
        .collect();

    // Source lines added versus HEAD: diff-added lines for tracked files, whole content
    // for untracked files (a new file's comments are all newly written).
    let mut added = Vec::new();
    if !tracked.is_empty() {
        let diff = Command::new("git")
            .args(["diff", "HEAD", "--unified=0", "--"])
            .args(&tracked)
            .stderr(Stdio::null())
            .output();
        for line in diff
            .map(|diff| diff.stdout)
            .unwrap_or_default()
            .split(|byte| *byte == b'\n')
        {
            if line.len() > 1 && line[0] == b'+' && line[1] != b'+' {
                added.extend(&line[1..]);
                added.push(b'\n');
            }
        }
    }
    for file in &untracked {
        added.extend(std::fs::read(file).unwrap_or_default());
    }
    // The comment text, not the whole line: editing the code around a comment does not re-trigger.
    let current: BTreeSet<Vec<u8>> = added
        .split(|byte| *byte == b'\n')
        .map(comment)
        .filter(|text| !text.is_empty())
        .collect();

    if args.first().is_some_and(|mode| mode == "seal") {
        let mut lines = head.into_bytes();
        for text in &current {
            lines.push(b'\n');
            lines.extend(text);
        }
        lines.push(b'\n');
        let _ = std::fs::write(memo, lines);
        return;
    }

    // The memo is valid only while HEAD is unchanged; a commit moves the diff base and resets it.
    let sealed = std::fs::read(memo).unwrap_or_default();
    let mut sealed = sealed.split(|byte| *byte == b'\n');
    let known: BTreeSet<&[u8]> = if sealed.next() == Some(head.as_bytes()) {
        sealed.collect()
    } else {
        BTreeSet::new()
    };
    let count = current
        .iter()
        .filter(|text| !known.contains(text.as_slice()))
        .count();
    if count == 0 {
        return;
    }
    // The memo is the primary brake; stop_hook_active caps a cycle at one dispatch so a memo hole cannot loop.
    if stop_hook_active() {
        eprintln!("\ncomment-pruner: {count} net-new comment line(s) still flagged after one dispatch this cycle; advisory only.");
        return;
    }
    let files: BTreeSet<&String> = tracked.iter().chain(&untracked).collect();
    eprintln!("\ncomment-pruner: {count} net-new comment line(s) added this session.");
    eprintln!("Dispatch the comment-pruner subagent (Agent tool, subagent_type: comment-pruner) to prune the");
    eprintln!("comments ADDED versus HEAD in the files below, then run from the repo root:");
    eprintln!("  .claude/hooks/comment-pruner.sh seal\n\nFiles:");
    for file in files {
        eprintln!("{file}");
    }
    exit(2)
}
