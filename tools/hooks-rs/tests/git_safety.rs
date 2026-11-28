mod common;

use common::{git, hook, run, write, TempDir};
use serde_json::json;

const TRUNK: &str = "main";
const FEATURE: &str = "feature/x";

struct Repo(TempDir);

impl Repo {
    fn on(test: &str, branch: &str) -> Repo {
        let tmp = TempDir::new(test);
        git(tmp.path(), &["init", "-q", "-b", branch]);
        write(&tmp.path().join(".claude/project.env"), "GIT_TRUNK=main\n");
        Repo(tmp)
    }

    fn exit_for_input(&self, input: &str) -> i32 {
        let dir = self.0.path();
        run(
            hook("git-safety")
                .current_dir(dir)
                .env("CLAUDE_PROJECT_DIR", dir),
            input,
        )
        .code
    }

    fn exit_for(&self, command: &str) -> i32 {
        let payload = json!({"tool_name": "Bash", "tool_input": {"command": command}});
        self.exit_for_input(&payload.to_string())
    }

    fn assert_rows(&self, want: i32, commands: &[&str]) {
        for command in commands {
            assert_eq!(self.exit_for(command), want, "command: {command:?}");
        }
    }
}

#[test]
fn allows_ordinary_commands() {
    Repo::on("ordinary", FEATURE).assert_rows(
        0,
        &["ls -la src", "rm -r build", "git reset --soft HEAD~1", ""],
    );
}

#[test]
fn blocks_recursive_force_remove() {
    Repo::on("rm-rf", FEATURE).assert_rows(2, &["rm -rf build"]);
}

#[test]
fn blocks_drop_table() {
    Repo::on("drop-table", FEATURE).assert_rows(2, &["psql -c 'DROP TABLE users'"]);
}

#[test]
fn blocks_force_push() {
    Repo::on("force-push", FEATURE).assert_rows(
        2,
        &[
            "git push --force origin feature/x",
            "git push -f origin feature/x",
            "git push --force-with-lease origin feature/x",
        ],
    );
}

#[test]
fn blocks_hard_reset() {
    Repo::on("hard-reset", FEATURE).assert_rows(2, &["git reset --hard HEAD~1"]);
}

#[test]
fn blocks_checkout_b_on_trunk() {
    Repo::on("checkout-b-trunk", TRUNK).assert_rows(2, &["git checkout -b feature/y"]);
}

#[test]
fn allows_checkout_b_off_trunk() {
    Repo::on("checkout-b-feature", FEATURE).assert_rows(0, &["git checkout -b feature/y"]);
}

#[test]
fn blocks_push_to_trunk() {
    Repo::on("push-trunk", FEATURE).assert_rows(
        2,
        &[
            "git push origin main",
            "git push origin HEAD:main",
            "git push origin HEAD:refs/heads/main",
            "git push origin :main",
            "git push origin main && echo done",
        ],
    );
}

#[test]
fn allows_push_that_only_mentions_trunk() {
    Repo::on("push-mentions-trunk", FEATURE).assert_rows(
        0,
        &[
            "git push origin feature/fix-main-red",
            "git push -u origin feature/x && gh pr create --base main",
        ],
    );
}

#[test]
fn allows_rule_words_split_across_lines() {
    Repo::on("split-lines", FEATURE).assert_rows(0, &["git push origin\nmain"]);
}

#[test]
fn blocks_bare_push_on_trunk() {
    Repo::on("bare-push-trunk", TRUNK).assert_rows(2, &["git push", "git add -A\ngit push"]);
}

#[test]
fn allows_bare_push_off_trunk() {
    Repo::on("bare-push-feature", FEATURE).assert_rows(0, &["git push"]);
}

#[test]
fn blocks_payload_that_is_not_json() {
    assert_eq!(Repo::on("not-json", FEATURE).exit_for_input("not json"), 2);
}

#[test]
fn reads_trunk_from_project_env() {
    let repo = Repo::on("trunk-from-env", FEATURE);
    write(
        &repo.0.path().join(".claude/project.env"),
        "GIT_TRUNK=develop\n",
    );
    let payload = json!({"tool_input": {"command": "git push origin develop"}});
    assert_eq!(repo.exit_for_input(&payload.to_string()), 2);
}
