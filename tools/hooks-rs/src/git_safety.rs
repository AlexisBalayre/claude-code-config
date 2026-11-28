use regex_lite::Regex;
use std::io::Read;
use std::process::{exit, Command};

fn block(message: &str) -> ! {
    eprintln!("BLOCKED: {message}");
    exit(2)
}

// grep matches line by line, so `$` and `[[:space:]]` never cross a newline.
fn matches(command: &str, pattern: &str) -> bool {
    let re = Regex::new(pattern).unwrap_or_else(|_| exit(2));
    command.lines().any(|line| re.is_match(line))
}

fn trunk() -> String {
    let dir = std::env::var("CLAUDE_PROJECT_DIR").unwrap_or_else(|_| ".".into());
    std::fs::read_to_string(format!("{dir}/.claude/project.env"))
        .ok()
        .and_then(|env| {
            env.lines()
                .rev()
                .find_map(|line| line.trim().strip_prefix("GIT_TRUNK=").map(str::to_owned))
        })
        .map(|value| value.trim_matches(['"', '\'']).to_owned())
        .or_else(|| std::env::var("GIT_TRUNK").ok())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "main".into())
}

fn on_trunk(trunk: &str) -> bool {
    Command::new("git")
        .args(["branch", "--show-current"])
        .output()
        .is_ok_and(|out| String::from_utf8_lossy(&out.stdout).trim() == trunk)
}

fn main() {
    let mut input = String::new();
    // An unreadable payload blocks, as the shell hook does when jq fails.
    if std::io::stdin().read_to_string(&mut input).is_err() {
        exit(2);
    }
    let payload: serde_json::Value = serde_json::from_str(&input).unwrap_or_else(|_| exit(2));
    let command = payload["tool_input"]["command"].as_str().unwrap_or("");
    // Every rule needs one of these words; most commands hold none.
    if !["rm", "DROP", "git"].iter().any(|word| command.contains(word)) {
        exit(0);
    }

    if matches(command, r"rm[[:space:]]+-rf[[:space:]]|DROP[[:space:]]+TABLE") {
        block("destructive command (rm -rf / DROP TABLE) not allowed.");
    }
    if matches(command, r"git[[:space:]]+push[[:space:]]+.*(--force\b|-f\b)") {
        block("'git push --force' is not allowed. Use a regular push or ask the user.");
    }
    if matches(command, r"git[[:space:]]+reset[[:space:]]+--hard\b") {
        block("'git reset --hard' is destructive. Stash or revert instead.");
    }

    let trunk = trunk();
    if matches(command, r"git[[:space:]]+checkout[[:space:]]+-b\b") && on_trunk(&trunk) {
        block(&format!(
            "do not use 'git checkout -b' on {trunk}. Use 'scripts/worktree-create.sh <name>' instead."
        ));
    }
    // The trunk goes in unescaped, as the shell hook interpolates it.
    let to_trunk = format!(
        r"git[[:space:]]+push([[:space:]]+[^|&;[:space:]]+)*[[:space:]]+([^|&;[:space:]]*:)?(refs/heads/)?{trunk}([[:space:]]|$|[|&;])"
    );
    if matches(command, &to_trunk) {
        block(&format!(
            "never push directly to {trunk}. Create a PR from a feature branch."
        ));
    }
    if matches(command, r"git[[:space:]]+push[[:space:]]*$") && on_trunk(&trunk) {
        block(&format!(
            "bare 'git push' while on {trunk}. Create a PR from a feature branch."
        ));
    }
}
