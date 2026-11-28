use regex_lite::Regex;
use serde_json::Value;
use std::collections::HashMap;
use std::io::Read;
use std::path::Path;
use std::process::{exit, Command, Stdio};

pub fn block(message: &str) -> ! {
    eprintln!("BLOCKED: {message}");
    exit(2)
}

pub fn stdin() -> Option<String> {
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input).ok()?;
    Some(input)
}

pub fn payload() -> Option<Value> {
    serde_json::from_str(&stdin()?).ok()
}

/// Match as `grep -E` does: line by line, so `$` and `[[:space:]]` never cross a newline.
pub fn grep(text: &str, pattern: &str) -> bool {
    let re = Regex::new(pattern).unwrap_or_else(|_| exit(2));
    text.lines().any(|line| re.is_match(line))
}

pub fn project_dir() -> String {
    std::env::var("CLAUDE_PROJECT_DIR")
        .ok()
        .filter(|dir| !dir.is_empty())
        .unwrap_or_else(|| ".".into())
}

/// Run git in `dir` and return its stdout without the trailing newline, or None when it fails.
pub fn git(dir: impl AsRef<Path>, args: &[&str]) -> Option<String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir.as_ref())
        .args(args)
        .stderr(Stdio::null())
        .output()
        .ok()?;
    out.status.success().then(|| {
        String::from_utf8_lossy(&out.stdout)
            .trim_end_matches('\n')
            .to_owned()
    })
}

/// The `KEY=value` lines of `<dir>/.claude/project.env`.
pub struct Config(HashMap<String, String>);

impl Config {
    // The shell hooks source this file. Only plain assignments are read here: one
    // layer of quotes is dropped and nothing is expanded.
    pub fn load(dir: impl AsRef<Path>) -> Config {
        let text =
            std::fs::read_to_string(dir.as_ref().join(".claude/project.env")).unwrap_or_default();
        let mut values = HashMap::new();
        for line in text.lines() {
            let line = line.trim();
            let line = line.strip_prefix("export ").unwrap_or(line).trim_start();
            if line.starts_with('#') {
                continue;
            }
            if let Some((key, value)) = line.split_once('=') {
                let value = value.trim();
                let unquoted = ['"', '\'']
                    .iter()
                    .find_map(|q| value.strip_prefix(*q)?.strip_suffix(*q))
                    .unwrap_or(value);
                values.insert(key.to_owned(), unquoted.to_owned());
            }
        }
        Config(values)
    }

    /// A key the file leaves out falls back to the environment, as sourcing does.
    pub fn get(&self, key: &str) -> String {
        self.0
            .get(key)
            .cloned()
            .or_else(|| std::env::var(key).ok())
            .unwrap_or_default()
    }
}
