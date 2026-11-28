use crate::common::git;
use serde_json::Value;
use std::io::Read;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{exit, Command, Stdio};
use std::time::{Duration, Instant};

// A hook that outlives its 15 s settings timeout blocks the push, so ls-remote gets
// what is left of this after the hook's own work.
const BUDGET: Duration = Duration::from_secs(8);

// No reason holds a `"` or `\`, so none needs JSON escaping.
fn ask(reason: &str) -> ! {
    println!(
        r#"{{"hookSpecificOutput":{{"hookEventName":"PreToolUse","permissionDecision":"ask","permissionDecisionReason":"push-gate: {reason}"}}}}"#
    );
    exit(0)
}

fn is_word(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_'
}

fn is_space(c: u8) -> bool {
    b" \t\n\r\x0b\x0c".contains(&c)
}

fn is_blank(c: &u8) -> bool {
    *c == b' ' || *c == b'\t'
}

/// The text after each occurrence of `needle`.
fn after<'a>(text: &'a [u8], needle: &'a [u8]) -> impl Iterator<Item = &'a [u8]> {
    text.windows(needle.len())
        .enumerate()
        .filter(move |(_, window)| *window == needle)
        .map(move |(at, _)| &text[at + needle.len()..])
}

/// `gh` however quoting or a line continuation splits it, in a command or in its JSON
/// (where a newline reads `\n`).
fn gh_spelt(text: &[u8]) -> bool {
    text.iter().enumerate().any(|(at, c)| {
        c.eq_ignore_ascii_case(&b'g')
            && text[at + 1..]
                .iter()
                .find(|c| !b"\\\"'$n\n".contains(c))
                .is_some_and(|c| c.eq_ignore_ascii_case(&b'h'))
    })
}

/// The end of the `gh` word whose `g` sits at `start`, quotes and escapes inside it
/// skipped and those in `trailing` after it, or None when the letters belong to a longer word.
fn gh_end(text: &[u8], start: usize, trailing: &[u8]) -> Option<usize> {
    let mut at = start + 1;
    while text.get(at).is_some_and(|c| b"\\\"'$".contains(c)) {
        at += 1;
    }
    if !text.get(at)?.eq_ignore_ascii_case(&b'h') {
        return None;
    }
    at += 1;
    while text.get(at).is_some_and(|c| trailing.contains(c)) {
        at += 1;
    }
    let glued = text
        .get(at)
        .is_some_and(|&c| is_word(c) || b"./:=@-".contains(&c));
    (!glued).then_some(at)
}

/// Whether the text ends a command here: its end, a separator, or a `&` that is no redirect.
fn ends_command(rest: &[u8]) -> bool {
    match rest {
        [] | [b';' | b'|' | b'\n' | b')', ..] => true,
        [b'&', rest @ ..] => rest.first() != Some(&b'>'),
        _ => false,
    }
}

#[derive(Default)]
struct ApiWord {
    word: Option<Vec<u8>>,
    // The characters neither quoted nor escaped, which the shell may glob or brace-expand.
    bare: Vec<u8>,
    rewritten: bool,
    redirect_target: bool,
    // These two hold for the whole call: a read of a ref or a file is routine, a write is not.
    sends: bool,
    ref_path: bool,
}

impl ApiWord {
    fn push(&mut self, text: &[u8]) {
        self.word.get_or_insert_with(Vec::new).extend(text);
    }

    /// Ends the current word and tells whether `gh api` may take it.
    fn take(&mut self) -> bool {
        let Some(word) = self.word.take() else {
            return true;
        };
        let (bare, rewritten, target) = (
            std::mem::take(&mut self.bare),
            std::mem::take(&mut self.rewritten),
            std::mem::take(&mut self.redirect_target),
        );
        if target {
            return true;
        }
        let path = word.strip_prefix(b"/").unwrap_or(&word);
        let path = match path.strip_prefix(b"repos/{owner}/{repo}") {
            Some(rest) if matches!(rest.first(), None | Some(b'/' | b'?')) => rest,
            _ => &word,
        };
        let braces = bare.iter().position(|&c| c == b'{').is_some_and(|open| {
            bare.iter().rposition(|&c| c == b'}').is_some_and(|close| {
                open < close && {
                    let inner = &bare[open + 1..close];
                    inner.contains(&b',') || after(inner, b"..").next().is_some()
                }
            })
        });
        let placeholder = ["owner", "repo", "branch"].iter().any(|name| {
            after(path, format!("{{{name}}}").as_bytes())
                .next()
                .is_some()
                || after(path, format!(":{name}").as_bytes())
                    .any(|rest| !rest.first().is_some_and(|&c| is_word(c)))
        });
        let lower = word.to_ascii_lowercase();
        self.sends |= ["--method", "--field", "--raw-field"]
            .iter()
            .any(|flag| word.starts_with(flag.as_bytes()))
            || word.starts_with(b"-")
                && !word.starts_with(b"--")
                && word.iter().any(|c| b"XfF".contains(c));
        self.ref_path |= [
            &b"/git/refs"[..],
            b"/contents",
            b"/update-branch",
            b"/rename",
            b"/autofix/commits",
        ]
        .iter()
        .any(|path| after(&lower, path).next().is_some());
        let merges = lower
            .strip_prefix(b"--input")
            .is_some_and(|rest| matches!(rest.first(), None | Some(b'=')))
            || after(&lower, b"=@").next().is_some()
            || lower.contains(&b'%')
            || after(&lower, b"/merge").any(|rest| {
                [&b"s"[..], b"-upstream", b""].iter().any(|suffix| {
                    rest.strip_prefix(*suffix)
                        .is_some_and(|rest| !rest.first().is_some_and(|&c| is_word(c) || c == b'-'))
                })
            })
            || [
                &b"mergepullrequest"[..],
                b"mergebranch",
                b"enablepullrequestautomerge",
                b"enqueuepullrequest",
                b"updatepullrequestbranch",
                b"createref",
                b"updateref",
                b"deleteref",
                b"createcommitonbranch",
                b"createlinkedbranch",
                b"revertpullrequest",
            ]
            .iter()
            .any(|mutation| after(&lower, mutation).next().is_some());
        !(rewritten || bare.iter().any(|c| b"*?[".contains(c)) || braces || placeholder || merges)
    }
}

/// Reads a `gh api` call word by word to the end of its command: a merge path or mutation, a
/// write to a ref or a file, a body from a file and anything the shell or `gh` would rewrite
/// fail it, and so does a `(`, which zsh globs rewrite a word with.
fn api_ok(text: &[u8]) -> bool {
    let mut arg = ApiWord::default();
    let mut at = 0;
    while !(ends_command(&text[at..]) || text[at] == b'(') {
        let c = text[at];
        match c {
            b' ' | b'\t' => {
                if !arg.take() {
                    return false;
                }
                at += 1;
            }
            b'&' | b'>' | b'<' => {
                if !arg.take() {
                    return false;
                }
                arg.redirect_target = true;
                let optional = |at: usize, set: &[u8]| {
                    at + usize::from(text.get(at).is_some_and(|c| set.contains(c)))
                };
                at = match c {
                    b'&' => optional(at + 2, b">"),
                    b'>' => optional(optional(at + 1, b">"), b"&|"),
                    _ => optional(optional(optional(at + 1, b"<"), b"<"), b"&>"),
                };
            }
            b'\'' => {
                let body = &text[at + 1..];
                let end = body.iter().position(|&c| c == b'\'');
                arg.push(&body[..end.unwrap_or(body.len())]);
                at += 1 + end.map_or(body.len(), |end| end + 1);
            }
            b'"' => {
                let mut end = at + 1;
                loop {
                    match text.get(end) {
                        Some(b'\\') if end + 1 < text.len() => end += 2,
                        Some(b'"' | b'\\') | None => break,
                        Some(_) => end += 1,
                    }
                }
                let body = &text[at + 1..end];
                arg.rewritten |= body.iter().any(|c| b"$`".contains(c));
                arg.push(body);
                at = end + usize::from(text.get(end) == Some(&b'"'));
            }
            b'\\' if at + 1 < text.len() => {
                arg.push(&text[at + 1..at + 2]);
                at += 2;
            }
            b'$' | b'`' => {
                arg.rewritten = true;
                arg.push(&[c]);
                at += 1;
            }
            _ => {
                arg.push(&[c]);
                arg.bare.push(c);
                at += 1;
            }
        }
    }
    text.get(at) != Some(&b'(') && arg.take() && !(arg.sends && arg.ref_path)
}

/// `auth status`, with nothing after it but `2>&1`, to the end of its command.
fn auth_status(rest: &[u8]) -> bool {
    let unblank = |text: &'_ [u8]| -> usize { text.iter().take_while(|c| is_blank(c)).count() };
    let Some(rest) = rest[unblank(rest)..].strip_prefix(b"auth") else {
        return false;
    };
    let Some(mut rest) = Some(unblank(rest))
        .filter(|blanks| *blanks > 0)
        .and_then(|blanks| rest[blanks..].strip_prefix(b"status"))
    else {
        return false;
    };
    loop {
        let blanks = unblank(rest);
        match rest[blanks..].strip_prefix(b"2>&1") {
            Some(next) if blanks > 0 => rest = next,
            _ => return ends_command(&rest[blanks..]),
        }
    }
}

/// The text before a `gh` call, each closed quote and escape read as Q. A quote the shell may
/// pair another way stays: one opened by `$'`, or a `"` holding `$(`.
fn before_call(raw: &[u8]) -> Vec<u8> {
    let mut before = Vec::with_capacity(raw.len());
    let mut at = 0;
    while at < raw.len() {
        let escape = |at: usize| raw[at] == b'\\' && raw.get(at + 1).is_some_and(|&c| c != b'\n');
        let end = match raw[at] {
            b'\'' if at == 0 || raw[at - 1] != b'$' => raw[at + 1..]
                .iter()
                .position(|&c| c == b'\'')
                .map(|close| at + close + 2),
            b'"' => {
                let mut end = at + 1;
                loop {
                    match raw.get(end) {
                        Some(b'"') => break Some(end + 1),
                        Some(b'\\') if escape(end) => end += 2,
                        Some(b'$') if raw.get(end + 1) != Some(&b'(') => end += 1,
                        Some(b'\\' | b'$') | None => break None,
                        Some(_) => end += 1,
                    }
                }
            }
            b'\\' if escape(at) => Some(at + 2),
            _ => None,
        };
        match end {
            Some(end) => {
                before.push(b'Q');
                at = end;
            }
            None => {
                before.push(raw[at]);
                at += 1;
            }
        }
    }
    // "$( opens a command; the string before it must be plain text, or the rewrite would drop a
    // heredoc, comment or quote that the checks on this text need.
    if let Some(body) = before.strip_suffix(b"$(") {
        if let Some(open) = body.iter().rposition(|c| b"\"'$<#()".contains(c)) {
            if body[open] == b'"' {
                before.truncate(open);
                before.push(b'(');
            }
        }
    }
    before
}

/// Whether a call after this text starts a command: outside quotes, comments and heredocs, and
/// after only shell keywords and plain `NAME=value` words.
fn starts_command(before: &[u8]) -> bool {
    let comment = before.iter().enumerate().any(|(at, &c)| {
        c == b'#' && (at == 0 || is_space(before[at - 1]) || b";&|()<>!".contains(&before[at - 1]))
    });
    let heredoc = before.windows(2).enumerate().any(|(at, pair)| {
        pair == b"<<" && (at == 0 || before[at - 1] != b'<') && before.get(at + 2) != Some(&b'<')
    });
    if comment || heredoc || before.iter().any(|c| b"\"'".contains(c)) {
        return false;
    }
    let start = before
        .iter()
        .rposition(|c| b";&|\n(".contains(c))
        .map_or(0, |separator| separator + 1);
    let prefix = &before[start..];
    let assignment = |word: &&[u8]| {
        let name = word.iter().take_while(|&&c| is_word(c)).count();
        name > 0
            && !word[0].is_ascii_digit()
            && word.get(name) == Some(&b'=')
            && word[name + 1..]
                .iter()
                .all(|&c| is_word(c) || b"./:,@%+-".contains(&c))
    };
    prefix.last().is_none_or(is_blank)
        && prefix
            .split(is_blank)
            .filter(|word| !word.is_empty())
            .skip_while(|word| {
                [
                    &b"if"[..],
                    b"then",
                    b"elif",
                    b"else",
                    b"do",
                    b"while",
                    b"until",
                    b"!",
                    b"{",
                ]
                .contains(word)
            })
            .all(|word| assignment(&word))
}

/// A `gh` call can merge in forms the settings rules do not match (flags before the subcommand,
/// quotes, expansions, `gh api`), so each `gh` in the command's text must have a listed shape; a
/// mention in a message or heredoc fails too. Another word can rewrite a call, add words to it
/// or parse it again (`xargs`, `eval`, `bash -c`, a heredoc fed to a shell), so a call must
/// also start a command. Quotes are paired only as far as that needs, so text the shell may
/// pair another way fails. A merge the text does not show (a script file, `gh` held in a
/// variable, another API client) passes: branch protection is the backstop.
fn gh_calls_ok(command: &[u8]) -> bool {
    // A backslash-newline between two non-blank characters glues them in the join below, but a
    // comment, a quote or a quoted heredoc delimiter keeps the line break, so the call after it
    // would go unseen (`#x\<LF>gh`).
    if command
        .windows(4)
        .any(|w| !is_space(w[0]) && w[1] == b'\\' && w[2] == b'\n' && !is_space(w[3]))
    {
        return false;
    }
    // Line continuations go, pairing escapes left to right: the newline after `\\` stays.
    let mut text = Vec::with_capacity(command.len());
    let mut at = 0;
    while at < command.len() {
        if command[at] != b'\\' || at + 1 == command.len() {
            text.push(command[at]);
            at += 1;
            continue;
        }
        if command[at + 1] != b'\n' {
            text.extend(&command[at..at + 2]);
        }
        at += 2;
    }
    let text = &text[..];
    let is_g = |at: usize| text[at].eq_ignore_ascii_case(&b'g');

    // An escape (`$'..\ngh'`, `printf`, `echo -e`) can end in a letter right before a `gh` the
    // shell reads as a command, but the call scan below sees a word there.
    for (escape, _) in text.iter().enumerate().filter(|(_, &c)| c == b'\\') {
        let mut at = escape + 1;
        while text.get(at).is_some_and(|&c| is_word(c)) {
            if at > escape + 1 && is_g(at) && gh_end(text, at, b"").is_some() {
                return false;
            }
            at += 1;
        }
    }

    (0..text.len()).all(|start| {
        if !is_g(start)
            || start > 0 && (is_word(text[start - 1]) || b".-".contains(&text[start - 1]))
        {
            return true;
        }
        let Some(end) = gh_end(text, start, b"\\\"'") else {
            return true;
        };
        let (raw, rest) = (&text[..start], &text[end..]);
        let flat: Vec<u8> = rest
            .iter()
            .copied()
            .filter(|c| !b"\"'\\".contains(c))
            .collect();
        let mut words = flat.as_slice();
        let mut word = || {
            words = &words[words.iter().take_while(|c| is_blank(c)).count()..];
            let end = words
                .iter()
                .take_while(|&&c| !(is_space(c) || b";&|()<>`".contains(&c)))
                .count();
            let word = &words[..end];
            words = &words[end..];
            word
        };
        let (group, sub) = (word(), word());
        // A backtick strips escapes from the call it holds, and a `;` inside `${` ends no command.
        if words.first().is_some_and(|c| b"(`<".contains(c))
            || raw.contains(&b'`')
            || after(raw, b"${").next().is_some()
            || !starts_command(&before_call(raw))
        {
            return false;
        }
        match group {
            b"pr" => [
                &b"view"[..],
                b"list",
                b"diff",
                b"checks",
                b"status",
                b"create",
                b"comment",
                b"edit",
                b"ready",
                b"checkout",
            ]
            .contains(&sub),
            b"api" => api_ok(rest),
            b"auth" => auth_status(rest),
            b"repo" => {
                sub.first().is_some_and(u8::is_ascii_lowercase)
                    && sub.iter().all(|&c| c.is_ascii_lowercase() || c == b'-')
                    && sub != b"sync"
            }
            _ => [
                &b"run"[..],
                b"issue",
                b"workflow",
                b"search",
                b"label",
                b"release",
                b"browse",
            ]
            .contains(&group),
        }
    })
}

/// Whether `word` stands in the text as a word of its own.
fn names(text: &str, word: &str, dot_joins: bool) -> bool {
    let bytes = text.as_bytes();
    let joins = |c: &u8| c.is_ascii_alphanumeric() || *c == b'_' || *c == b'-';
    text.match_indices(word).any(|(at, _)| {
        !(at > 0 && (joins(&bytes[at - 1]) || dot_joins && bytes[at - 1] == b'.'))
            && !bytes.get(at + word.len()).is_some_and(joins)
    })
}

/// The `-C` path and branch of `git [-C <path>] push [-u|--set-upstream] origin <branch>`.
fn canonical(command: &str) -> Option<(&str, &str)> {
    let plain = |word: &str, extra: &str| {
        word.chars()
            .all(|c| c.is_ascii_alphanumeric() || extra.contains(c))
    };
    let words: Vec<&str> = command
        .split([' ', '\t'])
        .filter(|word| !word.is_empty())
        .collect();
    let (path, rest) = match words.as_slice() {
        ["git", "-C", path, rest @ ..] if plain(path, "_./-") => (*path, rest),
        ["git", rest @ ..] => (".", rest),
        _ => return None,
    };
    match rest {
        ["push", "-u" | "--set-upstream", "origin", branch] | ["push", "origin", branch]
            if plain(branch, "._/-") && !branch.starts_with('-') =>
        {
            Some((path, branch))
        }
        _ => None,
    }
}

/// git sends `git push origin <branch>` to another ref (`main`, say) under a remote.origin.push
/// mapping or mirror, or push.default upstream (old name tracking) with the branch tracking
/// another one, and a legacy remotes/origin or branches/origin file holds push lines git config
/// never shows (builtin/push.c refspec_append_mapped, remote.c).
pub(crate) fn remapped(path: &str) -> bool {
    // `config -l -z` ends each `key\nvalue` entry with a NUL, so a value holding a newline
    // cannot pass for a key; the last push.default wins, as in git.
    let config = git(path, &["config", "-l", "-z"]).unwrap_or_default();
    let mut default = "";
    for entry in config.split('\0') {
        let (key, value) = entry.split_once('\n').unwrap_or((entry, ""));
        match key {
            "remote.origin.push" | "remote.origin.mirror" => return true,
            "push.default" => default = value,
            _ => {}
        }
    }
    default == "upstream"
        || default == "tracking"
        || git(
            path,
            &[
                "rev-parse",
                "--path-format=absolute",
                "--git-path",
                "remotes/origin",
                "--git-path",
                "branches/origin",
            ],
        )
        .is_some_and(|files| {
            files
                .split('\n')
                .take(2)
                .any(|file| Path::new(file).exists())
        })
}

/// The push URL the approved-plans `file` lists for `branch`. The lead lists
/// `<branch> <push url> ...` for a branch whose Design or Approach the user approved.
pub(crate) fn planned_url(file: impl AsRef<Path>, branch: &str) -> Option<String> {
    let plans = std::fs::read(file).ok()?;
    String::from_utf8_lossy(&plans)
        .split('\n')
        .find_map(|line| {
            let mut words = line.split([' ', '\t']).filter(|word| !word.is_empty());
            (words.next() == Some(branch)).then(|| words.next().unwrap_or("").to_owned())
        })
        .filter(|url| !url.is_empty())
}

/// Origin's push URL in `path`, when it has one alone and that one runs no helper. A push goes
/// to every push URL of origin (pushurl, pushInsteadOf), which can differ from the fetch URL a
/// plain `ls-remote origin` reads, and a <transport>::<address> URL runs git-remote-<transport>
/// from PATH.
pub(crate) fn push_url(path: &str) -> Option<String> {
    git(path, &["remote", "get-url", "--push", "--all", "origin"])
        .filter(|url| !url.contains('\n') && !url.contains("::"))
}

/// Whether the push URL lists the branch, within what is left of the budget.
// The crate's only unsafe: setsid before exec and a kill of the probe's process group have no
// safe std equivalent.
#[allow(unsafe_code)]
fn on_remote(url: &str, branch: &str, start: Instant) -> bool {
    // bash counts the hook's SECONDS from an inherited value, and the harness ages the hook
    // that way; an inherited value can only shorten the budget here.
    let aged = std::env::var("SECONDS")
        .ok()
        .and_then(|seconds| seconds.parse().ok())
        .unwrap_or(0);
    let spent = start.elapsed() + Duration::from_secs(aged);
    let Some(left) = BUDGET
        .checked_sub(spent)
        .filter(|left| *left >= Duration::from_secs(1))
    else {
        return false;
    };
    // ls-remote runs from / with the URL, outside the repo: the hook runs outside the sandbox,
    // and a worker can point its worktree's .git file at a config whose core.sshCommand or
    // protocol.ext.allow would then run. The `--` keeps a URL such as --upload-pack=<cmd> from
    // reading as an option, and GIT_PROTOCOL_FROM_USER=0 refuses a scheme git has no built-in
    // transport for, which would run git-remote-<scheme> from PATH.
    let mut ls_remote = Command::new("git");
    ls_remote
        .args(["-C", "/", "-c", "protocol.file.allow=always"])
        .args(["ls-remote", "--exit-code", "--heads", "--", url])
        .arg(format!("refs/heads/{branch}"))
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_PROTOCOL_FROM_USER", "0")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    // Its own session has no controlling terminal, so an ssh passphrase or host-key prompt
    // fails at once instead of outliving git and reading the user's keystrokes.
    unsafe {
        ls_remote.pre_exec(|| {
            libc::setsid();
            Ok(())
        });
    }
    let Ok(mut child) = ls_remote.spawn() else {
        return false;
    };
    let group = -(child.id() as i32);
    std::thread::spawn(move || {
        std::thread::sleep(left);
        unsafe { libc::kill(group, libc::SIGKILL) };
    });
    child.wait().is_ok_and(|status| status.success())
}

pub fn run() {
    let start = Instant::now();
    let mut input = Vec::new();
    let _ = std::io::stdin().read_to_end(&mut input);
    let input = String::from_utf8_lossy(input.split(|&c| c == 0).next().unwrap_or_default());
    // This runs on every Bash call, so a payload that cannot be a candidate below exits before
    // any parsing. The payload's JSON keeps ASCII letters literal.
    if !(input.contains("push") || input.contains("send-pack") || gh_spelt(input.as_bytes())) {
        return;
    }

    let fields = serde_json::from_str::<Value>(&input)
        .ok()
        .and_then(|payload| {
            let text = |value: &Value| match value {
                Value::Null | Value::Bool(false) => Some(String::new()),
                Value::String(text) => Some(text.replace('\0', "")),
                _ => None,
            };
            let tool_input = &payload["tool_input"];
            ((payload.is_object() || payload.is_null())
                && (tool_input.is_object() || tool_input.is_null()))
            .then_some(())?;
            Some((text(&payload["cwd"])?, text(&tool_input["command"])?))
        });
    let Some((cwd, command)) = fields else {
        ask("the hook payload names a push or gh and does not parse, so the gate cannot read the call.")
    };
    let (cwd, command) = (cwd.as_str(), command.trim_end_matches('\n'));

    // The scan reads the command with the newline a here-string ends it with.
    if gh_spelt(command.as_bytes()) && !gh_calls_ok(format!("{command}\n").as_bytes()) {
        ask("a gh call that can merge, write a ref or print the token needs the user's go, and so does a gh shape the gate does not list, a mention in text included.");
    }

    // A push this misses runs unprompted under the settings' git allow rules, and aliases
    // (`-c alias.p=push`), quoting and `send-pack` defeat any shape match. So any command
    // naming git and push or send-pack is a candidate; a false match only costs a prompt.
    if !(names(command, "git", true)
        && (names(command, "push", false) || names(command, "send-pack", false)))
    {
        return;
    }
    let Some((path, branch)) = canonical(command) else {
        ask("only 'git [-C <path>] push [-u|--set-upstream] origin <branch>', alone in its own call, pushes without this prompt.");
    };

    if std::env::set_current_dir(if cwd.is_empty() { "." } else { cwd }).is_err() || remapped(path)
    {
        ask(&format!("origin's config (remote.origin.push or mirror, push.default upstream or tracking, or a legacy remotes/ or branches/ file) can send '{branch}' to another ref, so it needs the user's go."));
    }

    // git resolves the word as rev-parse does and pushes to the ref it finds: the target of a
    // symbolic ref, a tag, or the branch `heads/<name>` spells.
    let full = format!("refs/heads/{branch}");
    if git(path, &["rev-parse", "--symbolic-full-name", branch]).as_deref() != Some(&full) {
        ask(&format!("'{branch}' is not the local branch {full} (a symbolic ref, a tag, another spelling of a branch, or no ref at all), so the push can land on another ref and needs the user's go."));
    }

    // The plans sit in a file the user's settings `env` names: a path the command can't choose
    // and the worker can't write, so an absolute one, since a relative path would resolve in
    // the call's cwd. The push skips the prompt only to the listed URL, since `-C` or a
    // redirected .git file can aim origin anywhere.
    let trusted = std::env::var("AGENT_APPROVED_PLANS")
        .ok()
        .filter(|file| Path::new(file).is_absolute())
        .and_then(|file| planned_url(file, branch));

    // The agent sets the push URL and the probe runs outside the sandbox, so a probe of any URL
    // would carry what the URL holds to any host: it reaches only a URL the user's settings
    // `env` lists, whole or by a prefix that ends the host: `/`, or `:` in the scp form alone,
    // since in a URL `host:` is followed by a port. What follows the prefix is plain path
    // characters, since git reads a later `@[host]` as the host and ssh takes the last `@`.
    let probed = std::env::var("PUSH_GATE_REMOTES").unwrap_or_default();
    let listed = push_url(path).is_some_and(|url| {
        trusted.as_ref() == Some(&url)
            || probed.split_ascii_whitespace().any(|entry| {
                url == entry
                    || (entry.ends_with('/') || entry.ends_with(':') && !entry.contains("://"))
                        && url.strip_prefix(entry).is_some_and(|rest| {
                            rest.bytes()
                                .all(|c| c.is_ascii_alphanumeric() || b"._/-".contains(&c))
                        })
            }) && on_remote(&url, branch, start)
    });
    if !listed {
        ask(&format!("'{branch}' is not on origin's push URL (a first push, a push after merge, several push URLs, a push URL neither the plans nor PUSH_GATE_REMOTES list, or origin unreachable or slow), so it needs the user's go."));
    }
}

#[cfg(test)]
mod tests {
    use super::BUDGET;
    use serde_json::Value;

    // The harness kills a hook at its settings timeout, and a killed push gate blocks the push.
    #[test]
    fn budget_ends_before_the_settings_timeout() {
        let settings = concat!(env!("CARGO_MANIFEST_DIR"), "/../../.claude/settings.json");
        let settings: Value =
            serde_json::from_str(&std::fs::read_to_string(settings).expect("read the settings"))
                .expect("parse the settings");
        let timeouts: Vec<u64> = settings["hooks"]
            .as_object()
            .expect("a hooks object")
            .values()
            .flat_map(|matchers| matchers.as_array().into_iter().flatten())
            .flat_map(|matcher| matcher["hooks"].as_array().into_iter().flatten())
            .filter(|hook| {
                hook["command"]
                    .as_str()
                    .is_some_and(|command| command.contains("push-gate"))
            })
            .map(|hook| {
                hook["timeout"]
                    .as_u64()
                    .expect("a timeout on the push gate")
            })
            .collect();
        assert!(
            !timeouts.is_empty(),
            "no hook in the settings names push-gate"
        );
        for timeout in timeouts {
            assert!(
                BUDGET.as_secs() < timeout,
                "budget {BUDGET:?}, timeout {timeout} s"
            );
        }
    }
}
