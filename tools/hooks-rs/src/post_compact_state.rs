use crate::common::{git, project_dir};
use crate::session::cwd;
use std::process::{Command, Stdio};

const PR_LINE: &str = r#".[] | "    #\(.number) \(.headRefName | gsub("[^A-Za-z0-9._/-]"; "?") | .[:60]) @\(.headRefOid[:7]) review: \(([.statusCheckRollup[] | select(.context == "claude-review") | .state][0]) // "none")""#;

// Paths and branch names are chosen by whoever made the worktree and land in Claude's
// context: keeping them to path-safe bytes stops one faking a line or a tag of this output.
fn safe(text: &str) -> String {
    text.bytes()
        .map(|byte| match byte {
            b'A'..=b'Z'
            | b'a'..=b'z'
            | b'0'..=b'9'
            | b'.'
            | b'_'
            | b'/'
            | b'('
            | b')'
            | b' '
            | b'-' => byte as char,
            _ => '?',
        })
        .collect()
}

pub fn run() {
    let cwd = cwd();
    // No git status, in any tree: refreshing the index runs the fsmonitor and clean filters
    // config names, and another session can point a worktree's .git file, or the shared
    // .git/config, at its own commands. The calls below only read refs and worktrees.
    if std::env::set_current_dir(project_dir()).is_err() {
        return;
    }
    let Some(top) = git(".", &["rev-parse", "--show-toplevel"]) else {
        return;
    };
    let branch = git(".", &["branch", "--show-current"])
        .filter(|branch| !branch.is_empty())
        .unwrap_or_else(|| "a detached HEAD".into());

    println!("Live state after compaction (read now; trust it over the summary):");
    if !cwd.is_empty() {
        println!("- session cwd: {}", safe(&cwd));
    }
    println!("- checkout: {} on {}", safe(&top), safe(&branch));

    println!("- other worktrees (path, branch):");
    let mut path = "";
    let worktrees = git(".", &["worktree", "list", "--porcelain"]).unwrap_or_default();
    for line in worktrees.split('\n') {
        let branch = if let Some(worktree) = line.strip_prefix("worktree ") {
            path = worktree;
            continue;
        } else if line.starts_with("branch ") {
            line.get("branch refs/heads/".len()..).unwrap_or("")
        } else if line.starts_with("detached") {
            "(detached)"
        } else {
            continue;
        };
        if path != top {
            println!("    {}  {}", safe(path), safe(branch));
        }
    }

    // The claude-review commit status sits on the PR's head commit, so "none" means the CI
    // review has not covered the current head. Only this user's PRs: on a busy repository,
    // others' would crowd the session's own out of the 20.
    let prs = Command::new("gh")
        .args([
            "pr", "list", "--author", "@me", "--state", "open", "--limit", "20",
        ])
        .args([
            "--json",
            "number,headRefName,headRefOid,statusCheckRollup",
            "--jq",
            PR_LINE,
        ])
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output();
    if let Ok(prs) = prs {
        let lines = String::from_utf8_lossy(&prs.stdout);
        if !lines.trim_end_matches('\n').is_empty() {
            println!("- open PRs:\n{}", lines.trim_end_matches('\n'));
        }
    }
}
