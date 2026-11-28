// Each test binary compiles its own copy of this module and uses only part of it.
#![allow(dead_code)]

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Own a scratch directory and remove it on drop.
pub struct TempDir(PathBuf);

impl TempDir {
    pub fn new(name: &str) -> TempDir {
        let path = std::env::temp_dir().join(format!("claude-hooks-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("create the scratch directory");
        TempDir(path)
    }

    pub fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

pub struct Output {
    pub code: i32,
    pub stdout: String,
    pub stderr: String,
}

// The environment is emptied because a test run started from a git hook inherits GIT_DIR,
// which would point every git call at the real repository, and because the hooks fall back
// to the environment for a key project.env leaves out.
fn isolated(program: &str) -> Command {
    let mut command = Command::new(program);
    command
        .env_clear()
        .env("PATH", std::env::var_os("PATH").unwrap_or_default())
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1");
    command
}

/// Build a call of one hook subcommand, with no project dir and no user git config.
pub fn hook(subcommand: &str) -> Command {
    let mut command = isolated(env!("CARGO_BIN_EXE_claude-hooks"));
    command.arg(subcommand);
    command
}

/// Run the command with `stdin` as its input and collect its exit code and output.
pub fn run(command: &mut Command, stdin: &str) -> Output {
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn the hook");
    // A hook may exit before it reads its input, so a broken pipe here is not a failure.
    let _ = child
        .stdin
        .take()
        .expect("piped stdin")
        .write_all(stdin.as_bytes());
    let out = child.wait_with_output().expect("wait for the hook");
    Output {
        code: out.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
    }
}

/// Run a fixture git command in `dir` and panic when it fails.
pub fn git(dir: &Path, args: &[&str]) {
    let out = isolated("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .env("GIT_AUTHOR_NAME", "t")
        .env("GIT_AUTHOR_EMAIL", "t@t")
        .env("GIT_COMMITTER_NAME", "t")
        .env("GIT_COMMITTER_EMAIL", "t@t")
        .output()
        .expect("spawn git");
    assert!(
        out.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// Write `text` to `path`, creating the parent directories.
pub fn write(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().expect("a parent directory")).expect("create parents");
    std::fs::write(path, text).expect("write the fixture file");
}

impl Output {
    /// Join stdout and stderr, for a check on text that may land on either.
    pub fn text(&self) -> String {
        format!("{}{}", self.stdout, self.stderr)
    }
}

/// Build a fixture git command in `dir`, for a call that may fail or whose output matters.
pub fn git_command(dir: &Path) -> Command {
    let mut command = isolated("git");
    command
        .arg("-C")
        .arg(dir)
        .env("GIT_AUTHOR_NAME", "t")
        .env("GIT_AUTHOR_EMAIL", "t@t")
        .env("GIT_COMMITTER_NAME", "t")
        .env("GIT_COMMITTER_EMAIL", "t@t");
    command
}

/// Run a fixture git command in `dir` and return its stdout without the trailing newline.
pub fn git_stdout(dir: &Path, args: &[&str]) -> String {
    let out = run(git_command(dir).args(args), "");
    assert_eq!(out.code, 0, "git {args:?} failed: {}", out.stderr);
    out.stdout.trim_end_matches('\n').to_owned()
}

/// Write an executable script at `path`, for a stub on PATH or an installed git hook.
pub fn executable(path: &Path, text: &str) {
    use std::os::unix::fs::PermissionsExt;
    write(path, text);
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))
        .expect("mark the script executable");
}

/// Put `dirs` ahead of the inherited PATH, first one searched first.
pub fn path_with(dirs: &[&Path]) -> std::ffi::OsString {
    let inherited = std::env::var_os("PATH").unwrap_or_default();
    let dirs = dirs.iter().map(|dir| dir.to_path_buf());
    std::env::join_paths(dirs.chain(std::env::split_paths(&inherited))).expect("join PATH")
}

/// Build the PreToolUse payload of a Bash call made from `cwd`.
pub fn bash_payload(cwd: &str, command: &str) -> String {
    serde_json::json!({"tool_input": {"command": command}, "cwd": cwd}).to_string()
}

/// Name a PreToolUse decision: "none", "ask", or the exit code or output that is neither.
pub fn decision(out: &Output) -> String {
    if out.code != 0 {
        return format!("exit {}", out.code);
    }
    if out.stdout.is_empty() {
        return "none".to_owned();
    }
    let output: serde_json::Value = serde_json::from_str(&out.stdout).unwrap_or_default();
    if output["hookSpecificOutput"]["permissionDecision"] == "ask" {
        "ask".to_owned()
    } else {
        format!("other: {}", out.stdout)
    }
}

/// Assert what `gate` decides for a Bash call, and that its output holds `reason`.
pub fn expect_decision(gate: &mut Command, want: &str, cwd: &str, command: &str, reason: &str) {
    let out = run(gate, &bash_payload(cwd, command));
    assert_eq!(decision(&out), want, "command: {command:?}");
    assert!(
        out.stdout.contains(reason),
        "command: {command:?}: want a reason holding {reason:?}, got {}",
        out.stdout
    );
}

/// The worker the worker-permission tests run as, on issue 12.
pub const WORKER: &str = "w5-12";

/// What the worker-permission hook decided, with the message of a deny.
pub struct Verdict {
    pub behavior: String,
    pub message: String,
}

impl Verdict {
    /// The request id a refusal names when a grant can follow it, or "" when it names none.
    pub fn id(&self) -> &str {
        self.message
            .split_once("request ")
            .and_then(|(_, rest)| rest.get(..64))
            .filter(|id| id.bytes().all(|c| c.is_ascii_hexdigit()))
            .unwrap_or("")
    }
}

/// Read the decision a PermissionRequest hook printed; no output reads as an empty verdict.
pub fn verdict(out: &Output) -> Verdict {
    let printed: serde_json::Value = serde_json::from_str(&out.stdout).unwrap_or_default();
    let decision = &printed["hookSpecificOutput"]["decision"];
    Verdict {
        behavior: decision["behavior"].as_str().unwrap_or("").to_owned(),
        message: decision["message"].as_str().unwrap_or("").to_owned(),
    }
}

/// Hold the lead's state directory, for the worker-permission hook and the grant command.
pub struct Lead {
    pub tmp: TempDir,
}

impl Lead {
    pub fn new(test: &str) -> Lead {
        Lead {
            tmp: TempDir::new(test),
        }
    }

    pub fn state(&self) -> PathBuf {
        self.tmp.path().join("state")
    }

    /// Build the hook of the session `worker`, with the fixture's state directory.
    pub fn hook(&self, worker: &str) -> Command {
        let mut command = hook("worker-permission");
        command
            .env("AGENT_WORKER", worker)
            .env("AGENT_WORK", self.state());
        command
    }

    /// Ask the hook, as `worker`, about a call of `tool` made from `cwd`.
    pub fn ask_as(
        &self,
        worker: &str,
        cwd: &Path,
        tool: &str,
        input: serde_json::Value,
    ) -> Verdict {
        let payload = serde_json::json!({
            "session_id": "s",
            "cwd": cwd,
            "permission_mode": "default",
            "hook_event_name": "PermissionRequest",
            "tool_name": tool,
            "tool_input": input,
        });
        verdict(&run(&mut self.hook(worker), &payload.to_string()))
    }

    /// Ask the hook about a Bash call of the test worker.
    pub fn bash(&self, cwd: &Path, command: &str) -> Verdict {
        let input = serde_json::json!({"command": command, "description": "d"});
        self.ask_as(WORKER, cwd, "Bash", input)
    }

    /// Run `claude-hooks grant <verb> <id>` on the fixture's state directory.
    pub fn grant(&self, verb: &str, id: &str) -> Output {
        let mut command = hook("grant");
        command.args([verb, id]).env("AGENT_WORK", self.state());
        run(&mut command, "")
    }

    /// Count the recorded requests.
    pub fn requests(&self) -> usize {
        std::fs::read_dir(self.state().join("requests")).map_or(0, |entries| entries.count())
    }

    /// Read the record of the request `id`.
    pub fn request(&self, id: &str) -> serde_json::Value {
        let file = self.state().join(format!("requests/{id}.json"));
        let text = std::fs::read_to_string(file).expect("a recorded request");
        serde_json::from_str(&text).expect("a request that is JSON")
    }

    /// Add a repository `name` with one commit on `branch`, and return its path.
    pub fn repo(&self, name: &str, branch: &str) -> PathBuf {
        let repo = self.tmp.path().join(name);
        git(self.tmp.path(), &["init", "-q", "-b", branch, name]);
        git(&repo, &["commit", "-q", "--allow-empty", "-m", "one"]);
        repo
    }

    /// Assert each Bash call is refused with a recorded request a grant can follow.
    pub fn assert_grant_route(&self, cwd: &Path, commands: &[&str]) {
        for command in commands {
            let verdict = self.bash(cwd, command);
            assert_eq!(verdict.behavior, "deny", "command: {command:?}");
            assert!(
                !verdict.id().is_empty(),
                "command: {command:?}: want a request id, got {}",
                verdict.message
            );
        }
    }

    /// Assert each Bash call is refused for `reason`, with no request for a grant to follow.
    pub fn assert_no_grant_route(&self, cwd: &Path, commands: &[&str], reason: &str) {
        for command in commands {
            let verdict = self.bash(cwd, command);
            assert_eq!(verdict.behavior, "deny", "command: {command:?}");
            assert!(
                verdict.message.contains(reason),
                "command: {command:?}: want a refusal holding {reason:?}, got {}",
                verdict.message
            );
        }
        assert_eq!(
            self.requests(),
            0,
            "a call with no grant route was recorded"
        );
    }
}

/// Set the file's mtime `minutes` into the past.
pub fn age(file: &Path, minutes: u64) {
    let then = std::time::SystemTime::now() - std::time::Duration::from_secs(minutes * 60);
    std::fs::File::options()
        .write(true)
        .open(file)
        .and_then(|file| file.set_modified(then))
        .expect("age the file");
}
