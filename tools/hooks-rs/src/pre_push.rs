use crate::added_lines;
use crate::common::{git, stdin, Config};
use std::process::exit;

pub fn run(_args: &[String]) {
    let rev_parse = |what: &str| git(".", &["rev-parse", what]).unwrap_or_default();
    let trunk = Some(Config::load(rev_parse("--show-toplevel")).get("GIT_TRUNK"))
        .filter(|trunk| !trunk.is_empty())
        .unwrap_or_else(|| "main".into());
    // scripts/verify.sh stamps a tree green in the pushing worktree's git dir.
    let stamps = std::fs::read_to_string(format!("{}/verify-stamp", rev_parse("--git-dir")))
        .unwrap_or_default();
    let in_claude = std::env::var("CLAUDECODE").is_ok_and(|value| value == "1");

    for line in stdin().unwrap_or_default().lines() {
        let mut fields = line.split_whitespace();
        let (_, local_sha, remote_ref) = (
            fields.next(),
            fields.next().unwrap_or(""),
            fields.next().unwrap_or(""),
        );
        // A delete pushes no tree, and a tag push (a release) runs no gates.
        if (!local_sha.is_empty() && local_sha.bytes().all(|b| b == b'0'))
            || remote_ref.starts_with("refs/tags/")
        {
            continue;
        }
        let tree = rev_parse(&format!("{local_sha}^{{tree}}"));
        if !stamps
            .lines()
            .any(|stamp| stamp.starts_with(&format!("{tree} ")))
        {
            println!();
            println!(
                "\u{1f6ab} BLOCKED: no green scripts/verify.sh run for {local_sha} (tree {tree})."
            );
            println!();
            println!(
                "Run 'bash scripts/verify.sh' on a clean checkout of that commit, then push again."
            );
            println!();
            exit(1)
        }
        // The cap binds agents only, so a push from a plain terminal skips it. It
        // counts added lines because a deletion is cheap to review. Only the user
        // writes size-waiver.
        let Some(branch) = remote_ref.strip_prefix("refs/heads/").filter(|_| in_claude) else {
            continue;
        };
        // By its full name: a bare origin/<trunk> resolves to a branch or tag of that name first.
        let trunk_ref = format!("refs/remotes/origin/{trunk}");
        let Some(base) = git(".", &["merge-base", &trunk_ref, local_sha]) else {
            continue;
        };
        let Ok(count) = added_lines::count(&base, local_sha) else {
            println!("ERROR: the size cap could not count '{branch}': git diff failed, or GENERATED_PATHS_REGEX is not a valid pattern.");
            exit(1)
        };
        let added = count.added;
        let waiver = format!("{}/size-waiver", rev_parse("--git-common-dir"));
        let waived = || {
            std::fs::read_to_string(&waiver)
                .is_ok_and(|text| text.lines().any(|line| line == branch))
        };
        if count.level == "warn" {
            println!("\u{26a0}\u{fe0f}  '{branch}' adds {added} production lines over origin/{trunk}; from 400, the PR body says why it can't split.");
        } else if count.level == "fail" && !waived() {
            let mut rows = count.rows;
            let lines = |row: &String| row.split('\t').next().and_then(|n| n.parse::<u64>().ok());
            rows.sort_by(|a, b| (lines(b), b).cmp(&(lines(a), a)));
            println!();
            println!(
                "\u{1f6ab} BLOCKED: '{branch}' adds {added} production lines over origin/{trunk}; the cap fails at {}.",
                count.fail_at
            );
            println!();
            println!("Biggest files:");
            for row in rows.iter().take(5) {
                println!("  {row}");
            }
            println!();
            println!("Split the branch, or ask the user to add '{branch}' to {waiver}.");
            println!();
            exit(1)
        }
    }
}
