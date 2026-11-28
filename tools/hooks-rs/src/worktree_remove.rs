use crate::added_lines::git_loud;
use crate::common::{git, Config};
use crate::worktree_create::valid_name;
use std::path::Path;
use std::process::{exit, Command, Stdio};

const USAGE: &str = "Usage: scripts/worktree-remove.sh <name>... | --merged";

struct Repo {
    main_toplevel: String,
    prefix: String,
    trunk: String,
    /// AGENT_APPROVED_PLANS: the lead's approved-plans file, read by the push gate.
    plans: String,
    fetched: bool,
    merged_only: bool,
}

fn git_run(args: &[&str]) -> bool {
    Command::new("git")
        .args(args)
        .status()
        .is_ok_and(|status| status.success())
}

fn tip_of(branch: &str) -> String {
    git_loud(&["rev-parse", &format!("refs/heads/{branch}")]).unwrap_or_else(|code| exit(code))
}

fn delete_branch(branch: &str) {
    if !git_run(&["branch", "-D", branch]) {
        exit(1)
    }
}

fn remove_empty_dirs(dir: &Path) {
    for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        if entry.file_type().is_ok_and(|kind| kind.is_dir()) {
            remove_empty_dirs(&entry.path());
        }
    }
    let _ = std::fs::remove_dir(dir);
}

impl Repo {
    // A squash merge leaves the tip off the trunk, so only GitHub knows it shipped.
    // Returns the merged PR head that holds the tip: the tip itself, or a descendant
    // when commits were added on GitHub (a suggestion, Update branch). gh's stderr
    // stays visible: a failed lookup (no auth, say) reads as unshipped, and only
    // gh's error says it failed.
    fn shipped(&self, branch: &str, tip: &str) -> Option<String> {
        let listed = Command::new("gh")
            .args(["pr", "list", "--head", branch, "--base", &self.trunk])
            .args([
                "--state",
                "merged",
                "--json",
                "headRefOid",
                "--jq",
                ".[].headRefOid",
            ])
            .stderr(Stdio::inherit())
            .output();
        let oids = match listed {
            Ok(out) => String::from_utf8_lossy(&out.stdout).into_owned(),
            Err(error) => {
                eprintln!("gh: {error}");
                return None;
            }
        };
        oids.split_whitespace()
            .find(|oid| {
                (git(".", &["cat-file", "-e", &format!("{oid}^{{commit}}")]).is_some()
                    || git(".", &["fetch", "--quiet", "origin", oid]).is_some())
                    && git_loud(&["merge-base", "--is-ancestor", tip, oid]).is_ok()
            })
            .map(str::to_owned)
    }

    // Drops a deleted branch's line from the approved-plans file, so a later branch of
    // that name needs a fresh approval.
    fn unlist_plan(&self, branch: &str) {
        if !Path::new(&self.plans).is_file() {
            eprintln!("Kept {branch}'s line in approved-plans: AGENT_APPROVED_PLANS names no file here (the lead removes it)");
            return;
        }
        // A line holds its checkout's push URL, and another repository may name its branches as
        // this one does: only this repo's line of the branch goes.
        let Ok(url) = git_loud(&["remote", "get-url", "--push", "origin"]) else {
            eprintln!("Kept {branch}'s line in approved-plans: origin has no push URL here");
            return;
        };
        let rewritten = std::fs::read_to_string(&self.plans).and_then(|text| {
            let kept: Vec<&str> = text
                .lines()
                .filter(|line| {
                    let mut fields = line.split_whitespace();
                    !(fields.next() == Some(branch) && fields.next() == Some(&url))
                })
                .collect();
            std::fs::write(
                &self.plans,
                format!("{}\n", kept.join("\n").trim_end_matches('\n')),
            )
        });
        if let Err(error) = rewritten {
            eprintln!("{}: {error}", self.plans);
            exit(1)
        }
    }

    // Deletes a branch once a merged PR shipped its tip, and origin's copy too while it
    // still sits at that PR's head; or once origin's copy holds every commit. Otherwise
    // keeps it.
    fn drop_branch(&self, branch: &str, tip: &str, merged_head: Option<&str>) {
        let remote_tip = git(
            ".",
            &[
                "rev-parse",
                "-q",
                "--verify",
                &format!("refs/remotes/origin/{branch}"),
            ],
        );
        if let Some(merged_head) = merged_head {
            delete_branch(branch);
            self.unlist_plan(branch);
            if remote_tip.as_deref() == Some(merged_head) {
                // The lease has origin refuse the delete if the branch moved since the fetch.
                let lease = format!("--force-with-lease=refs/heads/{branch}:{merged_head}");
                let head = format!("refs/heads/{branch}");
                if !git_run(&["push", "--quiet", &lease, "origin", "--delete", &head]) {
                    eprintln!("Kept origin/{branch}: delete refused (see git's error above)");
                }
            } else if remote_tip.is_some() {
                eprintln!("Kept origin/{branch}: it moved past the merged tip");
            }
        } else if remote_tip
            .is_some_and(|remote| git_loud(&["merge-base", "--is-ancestor", tip, &remote]).is_ok())
        {
            delete_branch(branch);
            self.unlist_plan(branch);
        } else {
            eprintln!(
                "Kept branch {branch}: no merged PR shipped it, and it has commits no remote holds"
            );
        }
    }

    fn remove(&self, name: &str) {
        if !valid_name(name) {
            eprintln!("Skipped '{name}': invalid worktree name (letters, digits, . _ - only; no leading dash)");
            return;
        }
        let dir = format!(".worktrees/{name}");
        if !Path::new(&dir).is_dir() {
            eprintln!("Skipped {name}: no worktree at {dir}");
            return;
        }
        // A merged worker's open session recreates its .claude/.cc-writes/, a directory git
        // resolves to the main checkout.
        if git(&dir, &["rev-parse", "--show-toplevel"]).as_deref() == Some(&self.main_toplevel) {
            remove_empty_dirs(Path::new(&dir));
            if Path::new(&dir).is_dir() {
                eprintln!("Kept {dir}: not a registered worktree, and it holds files");
            } else {
                println!("Removed leftover {dir}: no worktree, no files");
            }
            return;
        }

        let Some(branch) = git(&dir, &["branch", "--show-current"]) else {
            eprintln!("Skipped {name}: {dir} is not a working worktree");
            return;
        };
        if branch.is_empty() {
            eprintln!("Kept {dir}: detached HEAD, whose commits no branch may hold");
            return;
        }
        // Before the removal: .worktrees/ can also hold worktrees other tools own, on
        // branches of their own.
        if !branch.starts_with(&format!("{}/", self.prefix)) {
            eprintln!("Kept {dir}: {branch} is not a {}/ branch", self.prefix);
            return;
        }
        let tip = tip_of(&branch);
        let mut merged_head = self.fetched.then(|| self.shipped(&branch, &tip)).flatten();
        if self.merged_only && merged_head.is_none() {
            eprintln!("Kept {dir}: no PR merged into the trunk shipped its tip");
            return;
        }
        // No --force: git refuses a worktree with uncommitted or untracked files.
        if !git_run(&["worktree", "remove", &dir]) {
            eprintln!("Kept {dir}");
            return;
        }
        println!("Removed {dir}");
        if !self.fetched {
            return;
        }

        // A commit made in the worktree since the check above is the branch's too.
        let now = tip_of(&branch);
        if now != tip {
            merged_head = None;
        }
        self.drop_branch(&branch, &now, merged_head.as_deref());
    }
}

pub fn run(args: &[String]) {
    // The main checkout, which holds .worktrees/ and outlives any of them.
    let common_dir = git_loud(&["rev-parse", "--path-format=absolute", "--git-common-dir"])
        .unwrap_or_else(|code| exit(code));
    if std::env::set_current_dir(Path::new(&common_dir).parent().unwrap_or(Path::new("/"))).is_err()
    {
        exit(1)
    }
    let main_toplevel =
        git_loud(&["rev-parse", "--show-toplevel"]).unwrap_or_else(|code| exit(code));
    let config = Config::load(".");
    let or_default = |key: &str, default: &str| {
        Some(config.get(key))
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| default.into())
    };

    let merged_only = args.first().is_some_and(|arg| arg == "--merged");
    if args.is_empty() || (merged_only && args.len() > 1) {
        eprintln!("{USAGE}");
        exit(1)
    }

    // A stale origin ref could vouch for commits the remote no longer has, so
    // without a fresh fetch every branch is kept.
    let fetched = git_run(&["fetch", "--prune", "--quiet", "origin"]);
    if !fetched {
        eprintln!("Fetch failed: keeping every branch");
        if merged_only {
            exit(1)
        }
    }
    let repo = Repo {
        main_toplevel,
        prefix: or_default("WORKTREE_BRANCH_PREFIX", "feature"),
        trunk: or_default("GIT_TRUNK", "main"),
        plans: config.get("AGENT_APPROVED_PLANS"),
        fetched,
        merged_only,
    };

    let mut names = args.to_vec();
    if merged_only {
        names = std::fs::read_dir(".worktrees")
            .into_iter()
            .flatten()
            .flatten()
            .filter(|entry| entry.path().is_dir())
            .filter_map(|entry| entry.file_name().into_string().ok())
            .filter(|name| !name.starts_with('.'))
            .collect();
        names.sort();
    }
    for name in &names {
        repo.remove(name);
    }

    // Before the branch pass: an entry whose directory is gone still reports its
    // worktreepath, and git refuses to delete a branch such an entry holds.
    if !git_run(&["worktree", "prune"]) {
        exit(1)
    }

    // A branch whose worktree went by hand, or whose removal kept the branch, has
    // no worktree left to list it: --merged also drops those a merged PR shipped.
    if merged_only {
        let format = "--format=%(refname:short) %(worktreepath)";
        let heads = format!("refs/heads/{}/", repo.prefix);
        for line in git(".", &["for-each-ref", format, &heads])
            .unwrap_or_default()
            .lines()
        {
            let (branch, worktree) = line.split_once(' ').unwrap_or((line, ""));
            if !worktree.is_empty() {
                continue;
            }
            let tip = tip_of(branch);
            let Some(merged_head) = repo.shipped(branch, &tip) else {
                continue;
            };
            println!("Removing branch {branch}: no worktree, and a merged PR shipped it");
            repo.drop_branch(branch, &tip, Some(&merged_head));
        }
    }
}
