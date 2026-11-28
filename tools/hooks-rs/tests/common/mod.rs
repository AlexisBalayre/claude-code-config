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
