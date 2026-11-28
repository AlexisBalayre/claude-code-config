use crate::added_lines::git_loud;
use crate::check_dashes;
use crate::common::{git, grep, Config};
use std::process::{exit, Command, Stdio};

const BLOCKED_PATHS: [&str; 2] = ["docs/plans/", "docs/scratch/"];

pub fn run(_args: &[String]) {
    let root = git_loud(&["rev-parse", "--show-toplevel"]).unwrap_or_else(|code| exit(code));
    if std::env::set_current_dir(&root).is_err() {
        exit(1)
    }
    let config = Config::load(".");
    // --diff-filter=d: removing a tracked file is always allowed.
    let staged = ["diff", "--cached", "--diff-filter=d", "--name-only"];

    for path in BLOCKED_PATHS {
        let files = git(".", &[&staged[..], &["--", path]].concat()).unwrap_or_default();
        if !files.is_empty() {
            println!("ERROR: Attempting to commit files under '{path}' which is local-only.");
            println!("Blocked files:");
            for file in files.lines() {
                println!("  {file}");
            }
            println!();
            println!("These files should not be committed to the repository.");
            println!("Run 'git reset HEAD -- {path}' to unstage them.");
            exit(1)
        }
    }

    // Skipped mid-merge: the other side's lines are not this commit's.
    if git(".", &["rev-parse", "-q", "--verify", "MERGE_HEAD"]).is_none() {
        check_dashes::run(&["--cached".into()]);
    }

    let extensions = config.get("SOURCE_EXTENSIONS");
    let extensions: Vec<&str> = extensions
        .split(' ')
        .filter(|ext| !ext.is_empty())
        .collect();
    let lint = config.get("LINT_CMD");
    if !lint.is_empty() && !extensions.is_empty() {
        let pattern = format!(r"\.({})$", extensions.join("|"));
        let files = git(".", &staged).unwrap_or_default();
        let source: Vec<&str> = files.lines().filter(|file| grep(file, &pattern)).collect();
        if !source.is_empty() {
            println!("Running lint: {lint}");
            let passed = Command::new("sh")
                .args(["-c", &format!("{lint} \"$@\""), "_"])
                .args(source)
                .stdin(Stdio::null())
                .status()
                .is_ok_and(|status| status.success());
            if !passed {
                println!();
                println!("ERROR: Lint failed on staged files.");
                exit(1)
            }
        }
    }

    println!("All pre-commit checks passed.");
}
