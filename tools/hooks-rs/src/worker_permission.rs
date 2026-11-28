use crate::common::{agent_work, git, grep, payload, project_dir, Config};
use crate::push_gate::{is_word, planned, push_url, remapped};
use regex_lite::Regex;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::io::Write;
use std::os::unix::fs::MetadataExt;
use std::path::Path;
use std::process::exit;
use std::time::Duration;

/// How long a request waits for its grant, and a grant for the worker's retry.
const WINDOW: Duration = Duration::from_secs(30 * 60);

const PUSH_SHAPE: &str = "a push is tied to HEAD only as the whole call, git push [-u|--set-upstream] <remote> <checked-out branch>, with no &&, cd, -C, path or env prefix, alias, refspec or other branch";

// Exit 2 is no decision on a PermissionRequest hook, and a worker session denies an undecided
// call without telling the worker why, so every refusal is a printed deny.
fn decide(decision: Value) -> ! {
    println!(
        "{}",
        json!({"hookSpecificOutput": {"hookEventName": "PermissionRequest", "decision": decision}})
    );
    exit(0)
}

fn allow() -> ! {
    decide(json!({"behavior": "allow"}))
}

fn deny(message: &str) -> ! {
    decide(json!({"behavior": "deny", "message": format!("worker-permission: {message}")}))
}

/// The issue number in a worker name `<handle>-<N>`, or None for any other name.
// The dash keeps `lead`, the one name workers take approvals from, out of reach. The name is
// also a tmux window target, where `:` and `.` would parse as separators.
fn issue_of(name: &str) -> Option<&str> {
    let (handle, issue) = name.split_once('-')?;
    let mut letters = handle.bytes();
    (letters.next()?.is_ascii_lowercase()
        && letters.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        && issue
            .bytes()
            .next()
            .is_some_and(|c| (b'1'..=b'9').contains(&c))
        && issue.bytes().all(|c| c.is_ascii_digit()))
    .then_some(issue)
}

/// The text with each character outside printable ASCII as a `\uXXXX` escape: a bidi or format
/// character would show the lead one call while another runs, and a control byte could start a
/// line of its own in the lead's inbox. JSON stays the same JSON, since only its strings hold
/// such characters.
pub(crate) fn ascii(text: &str) -> String {
    let mut plain = String::with_capacity(text.len());
    for c in text.chars() {
        if (' '..='~').contains(&c) {
            plain.push(c);
        } else {
            for unit in c.encode_utf16(&mut [0; 2]) {
                plain.push_str(&format!("\\u{unit:04x}"));
            }
        }
    }
    plain
}

/// Whether the file was written within the window. Its mtime is its age: a retry rewrites a
/// request, and a rename keeps a grant's.
pub(crate) fn fresh(file: &Path) -> bool {
    std::fs::metadata(file)
        .and_then(|meta| meta.modified())
        .is_ok_and(|at| at.elapsed().is_ok_and(|age| age < WINDOW))
}

/// The words of a command the shell runs exactly as written: blank-separated, each made of
/// plain characters and of quoted text that expands nothing. None for anything else, such as a
/// separator, a redirect, a glob, a comment or an expansion.
fn literal_words(command: &str) -> Option<Vec<String>> {
    let mut words = Vec::new();
    let mut word: Option<String> = None;
    let mut chars = command.chars();
    while let Some(c) = chars.next() {
        match c {
            ' ' | '\t' => words.extend(word.take()),
            '\'' => {
                let text = word.get_or_insert_default();
                loop {
                    match chars.next()? {
                        '\'' => break,
                        c => text.push(c),
                    }
                }
            }
            '"' => {
                let text = word.get_or_insert_default();
                loop {
                    match chars.next()? {
                        '"' => break,
                        '$' | '`' => return None,
                        '\\' => match chars.next()? {
                            '\n' => {}
                            c @ ('$' | '`' | '"' | '\\') => text.push(c),
                            c => text.extend(['\\', c]),
                        },
                        c => text.push(c),
                    }
                }
            }
            // zsh expands a word that starts with `=` to a command's path.
            '=' if word.is_some() => word.get_or_insert_default().push(c),
            c if c.is_ascii_alphanumeric() || "_./:@%+,-".contains(c) => {
                word.get_or_insert_default().push(c)
            }
            _ => return None,
        }
    }
    words.extend(word);
    Some(words)
}

/// Whether the command is a tracker call a worker makes with no grant: a comment on its own
/// issue of `repo`, a new body for it, or a read of any issue there. Only the listed options
/// pass, so one that deletes a comment, or that posts a file (`--body-file` would publish any
/// file the worker can read), takes the grant route.
// `gh` reads the repository from the remotes of the directory it runs in, which the worker
// writes (`git remote set-url`, the gh-resolved key), so the call names it with `-R`.
fn tracker_call(command: &str, issue: &str, repo: &str) -> bool {
    let Some(words) = literal_words(command.trim_matches([' ', '\t', '\n'])) else {
        return false;
    };
    let words: Vec<&str> = words.iter().map(String::as_str).collect();
    let ["gh", "issue", verb, args @ ..] = words.as_slice() else {
        return false;
    };
    let body = ["--body", "-b"];
    let (valued, plain): (&[&str], &[&str]) = match *verb {
        "view" => (
            &["--json", "--jq", "-q", "--template", "-t"],
            &["--comments", "-c"],
        ),
        "comment" => (&body, &["--edit-last", "--create-if-none"]),
        "edit" => (&body, &[]),
        _ => return false,
    };
    let mut numbers = Vec::new();
    let mut aimed = false;
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        let (option, inline) = arg
            .split_once('=')
            .filter(|(option, _)| option.starts_with("--"))
            .map_or((*arg, None), |(option, value)| (option, Some(value)));
        let names_repo = matches!(option, "-R" | "--repo");
        if names_repo || valued.contains(&option) {
            let Some(value) = inline.or_else(|| args.next().copied()) else {
                return false;
            };
            if names_repo {
                if value != repo {
                    return false;
                }
                aimed = true;
            }
        } else if option.starts_with('-') {
            if inline.is_some() || !plain.contains(&option) {
                return false;
            }
        } else {
            numbers.push(*arg);
        }
    }
    aimed
        && match numbers.as_slice() {
            [number] if *verb == "view" => number.bytes().all(|c| c.is_ascii_digit()),
            [number] => *number == issue,
            _ => false,
        }
}

/// The words of a command as Python's shlex splits them in POSIX mode: quotes and escapes
/// read, and each run of `();<>|&` a word of its own. None for an open quote or a trailing
/// backslash.
fn shell_words(command: &str) -> Option<Vec<String>> {
    const PUNCTUATION: &str = "();<>|&";
    let mut words = Vec::new();
    let mut word: Option<String> = None;
    let mut chars = command.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            ' ' | '\t' | '\r' | '\n' => words.extend(word.take()),
            '\\' => word.get_or_insert_default().push(chars.next()?),
            '\'' => {
                let text = word.get_or_insert_default();
                loop {
                    match chars.next()? {
                        '\'' => break,
                        c => text.push(c),
                    }
                }
            }
            '"' => {
                let text = word.get_or_insert_default();
                loop {
                    match chars.next()? {
                        '"' => break,
                        '\\' => match chars.next()? {
                            c @ ('"' | '\\') => text.push(c),
                            c => text.extend(['\\', c]),
                        },
                        c => text.push(c),
                    }
                }
            }
            c if PUNCTUATION.contains(c) => {
                words.extend(word.take());
                let mut run = String::from(c);
                while let Some(next) = chars.next_if(|next| PUNCTUATION.contains(*next)) {
                    run.push(next);
                }
                words.push(run);
            }
            c => word.get_or_insert_default().push(c),
        }
    }
    words.extend(word);
    Some(words)
}

/// The commands of a call, split at its separators, with each redirect of stdout or stderr to
/// /dev/null or to another descriptor dropped. None for a call that cannot be read as plain
/// commands: any other redirect, a subshell, a quoted operator.
fn split_commands(words: Vec<String>) -> Option<Vec<Vec<String>>> {
    let mut commands = vec![Vec::new()];
    let mut words = words.into_iter().peekable();
    while let Some(word) = words.next() {
        let current = commands.last_mut()?;
        let from_fd = matches!(current.last().map(String::as_str), Some("1" | "2"));
        if matches!(word.as_str(), "|" | "||" | "&&" | ";" | "&") {
            commands.push(Vec::new());
        } else if from_fd
            && words
                .next_if(|target| match word.as_str() {
                    ">" => target == "/dev/null",
                    ">&" => {
                        target == "-"
                            || !target.is_empty() && target.bytes().all(|c| c.is_ascii_digit())
                    }
                    _ => false,
                })
                .is_some()
        {
            current.pop();
        } else if !word.is_empty() && word.chars().all(|c| "();<>|&".contains(c)) {
            return None;
        } else {
            current.push(word);
        }
    }
    commands.retain(|command| !command.is_empty());
    Some(commands)
}

/// git's subcommand, past its global options, or `alias` when one of them can define one.
fn git_subcommand<'a>(args: &[&'a str]) -> Option<&'a str> {
    let mut at = 0;
    while let Some(&arg) = args.get(at) {
        if arg.starts_with("--config-env") {
            return Some("alias");
        }
        if [
            "-c",
            "-C",
            "--git-dir",
            "--work-tree",
            "--namespace",
            "--super-prefix",
        ]
        .contains(&arg)
        {
            let value = args.get(at + 1).copied().unwrap_or_default();
            if arg == "-c" && value.to_ascii_lowercase().starts_with("alias.") {
                return Some("alias");
            }
            at += 2;
        } else if arg.starts_with('-') {
            at += 1;
        } else {
            return Some(arg);
        }
    }
    None
}

/// Whether the command's words show git running a push: through a path, an env prefix, an
/// alias or a global option, an expansion where the command word or the subcommand stands, or
/// a git command that runs another (`rebase -x`, `submodule foreach`, `subtree`).
fn runs_push(command: &[&str]) -> bool {
    command.iter().enumerate().any(|(at, word)| {
        let later = &command[at + 1..];
        let alias_key = word
            .to_ascii_lowercase()
            .strip_prefix("git_config_key")
            .and_then(|rest| rest.split_once('='))
            .is_some_and(|(key, value)| key.bytes().all(is_word) && value.starts_with("alias."));
        if alias_key {
            return true;
        }
        at == 0 && word.starts_with('$') && later.iter().any(|w| matches!(*w, "push" | "send-pack"))
            || word.rsplit('/').next() == Some("git")
                && git_subcommand(later).is_some_and(|sub| {
                    sub.starts_with('$')
                        || matches!(sub, "push" | "send-pack" | "alias")
                        || matches!(
                            sub,
                            "rebase" | "submodule" | "subtree" | "filter-branch" | "bisect"
                        ) && later.iter().any(|w| grep(w, r"\bpush\b"))
                })
    })
}

pub fn run() {
    // The hook can sit in settings every session reads. A session that is no worker gets no
    // decision from it, so its dialog opens as usual.
    let Some(worker) = std::env::var("AGENT_WORKER")
        .ok()
        .filter(|name| !name.is_empty())
    else {
        return;
    };
    let Some(issue) = issue_of(&worker) else {
        deny("AGENT_WORKER is not a worker name (<handle>-<issue number>), so the launcher did not start this session");
    };
    let payload = payload().unwrap_or_else(|| deny("unreadable request"));
    let tool = payload["tool_name"].as_str().unwrap_or_default();
    let input = &payload["tool_input"];
    let cwd = payload["cwd"].as_str().unwrap_or(".");
    let command = match tool {
        "Bash" => input["command"].as_str().unwrap_or_default(),
        _ => "",
    };
    // serde_json keeps an object's keys sorted, so a call has one text whatever order its keys
    // came in.
    let shown = ascii(&input.to_string());

    // A standing yes is for a call the sandbox holds. One that asks to leave it takes the grant
    // route, where the lead sees the flag in the input.
    let sandboxed = matches!(
        input["dangerouslyDisableSandbox"],
        Value::Null | Value::Bool(false)
    );
    // The launcher names the repository in the session's environment, which no call of the
    // worker's can change.
    let repo = std::env::var("AGENT_REPO").unwrap_or_default();
    if sandboxed && !repo.is_empty() && tracker_call(command, issue, &repo) {
        allow();
    }

    let flat = command.replace('\n', ";");
    let parsed = shell_words(&flat).and_then(split_commands);
    let commands: Vec<Vec<&str>> = parsed
        .iter()
        .flatten()
        .map(|command| command.iter().map(String::as_str).collect())
        .collect();
    // HEAD names what a push sends only when the push is the whole call, run in the session's
    // directory for the checked-out branch. A push in any other shape is refused with no grant
    // route. A push the words do not show (a string a shell runs, an expansion) takes the grant
    // route like any Bash call, with HEAD hashed in.
    let ref_name = "([A-Za-z0-9._/@][A-Za-z0-9._/@-]*)";
    let lone_push = Regex::new(&format!(
        "^git push(?: -u| --set-upstream)? {ref_name} {ref_name}(?: 2>&1| 2>/dev/null)?$"
    ))
    .unwrap_or_else(|_| deny("the push pattern did not build"));
    // Only the blanks the shell drops: a trailing no-break space belongs to the last word.
    let lone_push = lone_push.captures(command.trim_matches([' ', '\t', '\n']));
    if lone_push.is_none() && commands.iter().any(|command| runs_push(command)) {
        deny(&format!("{PUSH_SHAPE}. Run the push on its own."));
    }

    // A grant is for the HEAD the lead saw: a retry after a commit is a new request. A
    // directory with no commit fails here and hashes an empty HEAD.
    let rev = (tool == "Bash")
        .then(|| git(cwd, &["rev-parse", "HEAD", "--symbolic-full-name", "HEAD"]))
        .flatten();
    let (head, checked_out) = rev
        .as_deref()
        .and_then(|rev| rev.split_once('\n'))
        .unwrap_or_default();
    let Some(work) = agent_work(project_dir()) else {
        deny("no state directory: AGENT_WORK is unset and the project is no git checkout");
    };

    if let Some(push) = lone_push {
        let (remote, branch) = (&push[1], &push[2]);
        // A detached HEAD reads `HEAD`.
        let Some(current) = checked_out.strip_prefix("refs/heads/") else {
            deny("could not read HEAD in the session's directory, so the push cannot be tied to a commit");
        };
        if current != branch {
            deny(&format!(
                "{PUSH_SHAPE}: {branch} is not the checked-out branch."
            ));
        }
        // The approved plan is the go for every push of the worker's own branch, to the one URL
        // the lead listed and under no setting that sends it to another ref or sends more.
        let prefix = Some(Config::load(project_dir()).get("WORKTREE_BRANCH_PREFIX"))
            .filter(|prefix| !prefix.is_empty())
            .unwrap_or_else(|| "feature".into());
        if sandboxed
            && remote == "origin"
            && *branch == format!("{prefix}/issue-{issue}")
            && !remapped(cwd)
            && push_url(cwd).is_some_and(|url| planned(branch, &url))
        {
            allow();
        }
    }

    // A grant is also for the files the call can run: a script edited since the request makes
    // the retry a new request. The files are read by their stat, as git's own index does, since
    // `git status` would run the clean filters the repository's config names, and the worker
    // writes that config while this hook runs outside its sandbox. Ignored files are left out.
    let mut files = Sha256::new();
    if tool == "Bash" {
        // With an untracked cache, a listing of untracked files runs the core.fsmonitor command.
        let listed = git(
            cwd,
            &[
                "-c",
                "core.fsmonitor=false",
                "ls-files",
                "-z",
                "--cached",
                "--others",
                "--exclude-standard",
                ":/",
            ],
        )
        .unwrap_or_default();
        for file in listed.split('\0').filter(|file| !file.is_empty()) {
            let stat = std::fs::symlink_metadata(Path::new(cwd).join(file))
                .map(|meta| (meta.size(), meta.ctime(), meta.ctime_nsec()))
                .ok();
            files.update(format!("{file}\0{stat:?}\n"));
        }
    }

    // The whole digest: the worker picks every hashed byte, so a short id is one it could
    // collide, reusing a grant for a call the user never saw.
    let id = format!(
        "{:x}",
        Sha256::digest(format!(
            "{worker}\n{tool}\n{shown}\n{}\n{head}\n{:x}",
            ascii(cwd),
            files.finalize()
        ))
    );
    let grant = work.join("grants").join(&id);
    let used = grant.with_extension(format!("used.{}", std::process::id()));
    // The rename is the single use: of two calls racing on one grant, only one moves it.
    if std::fs::rename(&grant, &used).is_ok() {
        let granted =
            fresh(&used) && std::fs::read_to_string(&used).is_ok_and(|to| to.trim_end() == worker);
        let _ = std::fs::remove_file(&used);
        if granted {
            allow();
        }
    }

    let mut request = json!({"id": id, "worker": worker, "tool": tool, "input": input, "cwd": cwd});
    if !head.is_empty() {
        request["head"] = head.into();
    }
    // The inbox line is the hook's own account of the call, never the worker's.
    let call: String = format!("{} {shown}", ascii(tool))
        .chars()
        .take(300)
        .collect();
    let requests = work.join("requests");
    let recorded = std::fs::create_dir_all(&requests)
        .and_then(|()| {
            std::fs::write(
                requests.join(format!("{id}.json")),
                ascii(&request.to_string()) + "\n",
            )
        })
        .and_then(|()| {
            std::fs::File::options()
                .append(true)
                .create(true)
                .open(work.join("lead-inbox.txt"))
        })
        .and_then(|mut inbox| inbox.write_all(format!("GRANT? {worker} {id} {call}\n").as_bytes()));
    if recorded.is_err() {
        deny("could not record the request");
    }
    deny(&format!("this call needs the user's go, which the lead relays. Send the lead request {id} and the call. Once the lead reports the grant, retry this exact call, its description included, from this directory, within 30 minutes and with no commit or file change in between: a changed call, another directory, a moved HEAD or a changed working tree is a new request."));
}
