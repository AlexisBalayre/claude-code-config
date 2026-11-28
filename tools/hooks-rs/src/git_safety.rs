use crate::common::{block, git, grep, payload, project_dir, Config};

fn on_trunk(trunk: &str) -> bool {
    git(".", &["branch", "--show-current"]).is_some_and(|branch| branch == trunk)
}

pub fn run() {
    // An unreadable payload blocks, as the shell hook does when jq fails.
    let payload = payload().unwrap_or_else(|| block("git-safety could not read the hook payload."));
    let command = payload["tool_input"]["command"].as_str().unwrap_or("");
    // Every rule needs one of these words; most commands hold none.
    if !["rm", "DROP", "git"]
        .iter()
        .any(|word| command.contains(word))
    {
        return;
    }

    if grep(
        command,
        r"rm[[:space:]]+-rf[[:space:]]|DROP[[:space:]]+TABLE",
    ) {
        block("destructive command (rm -rf / DROP TABLE) not allowed.");
    }
    if grep(
        command,
        r"git[[:space:]]+push[[:space:]]+.*(--force\b|-f\b)",
    ) {
        block("'git push --force' is not allowed. Use a regular push or ask the user.");
    }
    if grep(command, r"git[[:space:]]+reset[[:space:]]+--hard\b") {
        block("'git reset --hard' is destructive. Stash or revert instead.");
    }

    let trunk = Some(Config::load(project_dir()).get("GIT_TRUNK"))
        .filter(|trunk| !trunk.is_empty())
        .unwrap_or_else(|| "main".into());
    if grep(command, r"git[[:space:]]+checkout[[:space:]]+-b\b") && on_trunk(&trunk) {
        block(&format!(
            "do not use 'git checkout -b' on {trunk}. Use 'scripts/worktree-create.sh <name>' instead."
        ));
    }
    // The trunk goes in unescaped, as the shell hook interpolates it.
    let to_trunk = format!(
        r"git[[:space:]]+push([[:space:]]+[^|&;[:space:]]+)*[[:space:]]+([^|&;[:space:]]*:)?(refs/heads/)?{trunk}([[:space:]]|$|[|&;])"
    );
    if grep(command, &to_trunk) {
        block(&format!(
            "never push directly to {trunk}. Create a PR from a feature branch."
        ));
    }
    if grep(command, r"git[[:space:]]+push[[:space:]]*$") && on_trunk(&trunk) {
        block(&format!(
            "bare 'git push' while on {trunk}. Create a PR from a feature branch."
        ));
    }
}
