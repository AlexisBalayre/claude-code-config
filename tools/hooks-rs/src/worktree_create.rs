use crate::added_lines::git_loud;
use crate::common::Config;
use std::path::Path;
use std::process::{exit, Command};

pub fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && !name.starts_with('-')
        && !name.contains("..")
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
}

fn or_exit(command: &mut Command) {
    let code = command
        .status()
        .map_or(127, |status| status.code().unwrap_or(1));
    if code != 0 {
        exit(code)
    }
}

pub fn run(args: &[String]) {
    let root = git_loud(&["rev-parse", "--show-toplevel"]).unwrap_or_else(|code| exit(code));
    if std::env::set_current_dir(&root).is_err() {
        exit(1)
    }
    let config = Config::load(".");
    let or_default = |key: &str, default: &str| {
        Some(config.get(key))
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| default.into())
    };

    let Some(name) = args.first().filter(|name| !name.is_empty()) else {
        eprintln!("Usage: scripts/worktree-create.sh <name>");
        exit(1)
    };
    if !valid_name(name) {
        eprintln!(
            "Error: invalid worktree name '{name}' (letters, digits, . _ - only; no leading dash)."
        );
        exit(1)
    }
    let dir = format!(".worktrees/{name}");
    let branch = format!("{}/{name}", or_default("WORKTREE_BRANCH_PREFIX", "feature"));
    if Path::new(&dir).is_dir() {
        eprintln!("Error: Worktree '{dir}' already exists.");
        exit(1)
    }

    // From origin's trunk, not the checkout's HEAD, which may be stale or on another branch. By its
    // full name: a bare origin/<trunk> resolves to a local branch or tag of that name first.
    // --no-track: a branch tracking the trunk would aim a bare `git push` at it.
    let trunk = or_default("GIT_TRUNK", "main");
    or_exit(Command::new("git").args(["fetch", "--quiet", "origin", &trunk]));
    or_exit(Command::new("git").args([
        "worktree",
        "add",
        "--no-track",
        &dir,
        "-b",
        &branch,
        &format!("refs/remotes/origin/{trunk}"),
    ]));

    // Fresh worktrees start without installed dependencies.
    let install = config.get("INSTALL_CMD");
    if !install.is_empty() {
        or_exit(
            Command::new("bash")
                .args(["-c", &install])
                .current_dir(&dir),
        );
    }

    println!();
    println!("Worktree created:");
    println!("  Directory: {dir}");
    println!("  Branch:    {branch}");
    println!();
    println!("cd {dir}");
}
