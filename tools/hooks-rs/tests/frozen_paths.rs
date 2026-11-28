mod common;

use common::{git, hook, run, write, Output, TempDir};
use serde_json::json;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};

struct Fixture {
    tmp: TempDir,
    repo: PathBuf,
    other: PathBuf,
    allow: PathBuf,
}

impl Fixture {
    fn new(test: &str) -> Fixture {
        let tmp = TempDir::new(test);
        let repo = tmp.path().join("repo");
        let other = tmp.path().join("wt-other");
        let allow = tmp.path().join("frozen-allow.txt");
        for dir in ["apps/web", "apps/api", "packages/client-x", "packages/db"] {
            std::fs::create_dir_all(repo.join(dir)).expect("create a fixture directory");
        }
        write(&repo.join("packages/client/src/a.ts"), "x\n");
        write(
            &repo.join(".claude/project.env"),
            "FROZEN_PATHS_REGEX='^(packages/client|apps/web)/'\n",
        );
        git(tmp.path(), &["init", "-q", "-b", "feature/allowed", "repo"]);
        git(&repo, &["add", "-A"]);
        git(&repo, &["commit", "-qm", "init"]);
        git(
            &repo,
            &[
                "worktree",
                "add",
                "-q",
                "-b",
                "feature/other",
                "../wt-other",
            ],
        );
        symlink(&repo, tmp.path().join("link-to-repo")).expect("link the repo");
        std::fs::create_dir_all(other.join("apps/api")).expect("create apps/api");
        symlink(
            "../../packages/client/src/a.ts",
            other.join("apps/api/alias.ts"),
        )
        .expect("link the alias");
        write(
            &allow,
            "# comment line\n\
             feature/allowed packages/client/src/a.ts\n\
             feature/allowed apps/web/new/b.tsx\n",
        );
        Fixture {
            tmp,
            repo,
            other,
            allow,
        }
    }

    fn at(&self, base: &Path, rel: &str) -> String {
        format!("{}/{rel}", base.display())
    }

    fn in_repo(&self, rel: &str) -> String {
        self.at(&self.repo, rel)
    }

    fn in_tmp(&self, rel: &str) -> String {
        self.at(self.tmp.path(), rel)
    }

    fn call(&self, input: &str, allow: Option<&Path>, cwd: &Path) -> Output {
        let mut command = hook("frozen-paths");
        command
            .current_dir(cwd)
            .env("CLAUDE_PROJECT_DIR", &self.repo);
        if let Some(allow) = allow {
            command.env("AGENT_FROZEN_ALLOW", allow);
        }
        run(&mut command, input)
    }

    fn edit_payload(file: &str) -> String {
        json!({"tool_name": "Edit", "tool_input": {"file_path": file}}).to_string()
    }

    fn edit(&self, file: &str) -> i32 {
        self.call(
            &Self::edit_payload(file),
            Some(&self.allow),
            self.tmp.path(),
        )
        .code
    }

    fn notebook_edit(&self, file: &str) -> i32 {
        let payload = json!({
            "tool_name": "NotebookEdit",
            "tool_input": {"notebook_path": file, "file_path": "/tmp/decoy"},
        });
        self.call(&payload.to_string(), Some(&self.allow), self.tmp.path())
            .code
    }

    fn case_insensitive(&self) -> bool {
        self.tmp.path().join("REPO").is_dir()
    }
}

#[test]
fn allows_frozen_edit_when_the_variable_is_unset() {
    let f = Fixture::new("variable-unset");
    let payload = Fixture::edit_payload(&f.in_repo("packages/client/src/z.ts"));
    assert_eq!(f.call(&payload, None, f.tmp.path()).code, 0);
}

#[test]
fn allows_frozen_edit_without_a_frozen_regex() {
    let f = Fixture::new("no-regex");
    std::fs::remove_file(f.repo.join(".claude/project.env")).expect("remove project.env");
    assert_eq!(f.edit(&f.in_repo("packages/client/src/z.ts")), 0);
}

#[test]
fn allows_listed_branch_and_path() {
    let f = Fixture::new("listed");
    assert_eq!(f.edit(&f.in_repo("packages/client/src/a.ts")), 0);
}

#[test]
fn blocks_listed_path_on_another_branch() {
    let f = Fixture::new("other-branch");
    assert_eq!(f.edit(&f.at(&f.other, "packages/client/src/a.ts")), 2);
}

#[test]
fn blocks_unlisted_frozen_file_on_the_listed_branch() {
    let f = Fixture::new("unlisted-file");
    assert_eq!(f.edit(&f.in_repo("packages/client/src/z.ts")), 2);
}

#[test]
fn allows_lookalike_package_outside_the_freeze() {
    let f = Fixture::new("lookalike");
    assert_eq!(f.edit(&f.in_repo("packages/client-x/a.ts")), 0);
}

#[test]
fn allows_unfrozen_package() {
    let f = Fixture::new("unfrozen");
    assert_eq!(f.edit(&f.in_repo("packages/db/a.ts")), 0);
}

#[test]
fn blocks_write_into_a_frozen_dir_that_does_not_exist_yet() {
    let f = Fixture::new("missing-dir");
    assert_eq!(f.edit(&f.in_repo("apps/web/missing/c.tsx")), 2);
}

#[test]
fn allows_listed_new_file_under_a_missing_dir() {
    let f = Fixture::new("listed-missing-dir");
    assert_eq!(f.edit(&f.in_repo("apps/web/new/b.tsx")), 0);
}

#[test]
fn blocks_when_the_allow_list_is_missing() {
    let f = Fixture::new("allow-missing");
    let payload = Fixture::edit_payload(&f.in_repo("packages/client/src/a.ts"));
    let missing = f.tmp.path().join("no-such-file");
    assert_eq!(f.call(&payload, Some(&missing), f.tmp.path()).code, 2);
}

#[test]
fn blocks_parent_dir_hop_into_the_frozen_app() {
    let f = Fixture::new("dotdot");
    assert_eq!(f.edit(&f.in_repo("apps/api/../web/x.ts")), 2);
}

#[test]
fn blocks_parent_dir_hop_through_a_missing_dir() {
    let f = Fixture::new("dotdot-missing");
    assert_eq!(f.edit(&f.in_repo("apps/nope/../web/x.ts")), 2);
}

#[test]
fn allows_listed_file_through_a_symlinked_repo_dir() {
    let f = Fixture::new("symlink-listed");
    assert_eq!(
        f.edit(&f.in_tmp("link-to-repo/packages/client/src/a.ts")),
        0
    );
}

#[test]
fn blocks_unlisted_file_through_a_symlinked_repo_dir() {
    let f = Fixture::new("symlink-unlisted");
    assert_eq!(
        f.edit(&f.in_tmp("link-to-repo/packages/client/src/z.ts")),
        2
    );
}

#[test]
fn blocks_symlinked_file_pointing_into_the_freeze_from_another_branch() {
    let f = Fixture::new("symlink-file");
    assert_eq!(f.edit(&f.at(&f.other, "apps/api/alias.ts")), 2);
}

#[test]
fn blocks_wrong_case_path_into_the_frozen_app() {
    let f = Fixture::new("wrong-case");
    assert_eq!(f.edit(&f.in_repo("Apps/Web/x.ts")), 2);
}

#[test]
fn blocks_wrong_case_frozen_dir_not_yet_on_disk() {
    let f = Fixture::new("wrong-case-missing");
    assert_eq!(f.edit(&f.at(&f.other, "Apps/Web/x.ts")), 2);
}

#[test]
fn blocks_wrong_case_path_to_an_unlisted_frozen_file() {
    let f = Fixture::new("wrong-case-unlisted");
    assert_eq!(f.edit(&f.in_repo("Packages/Client/src/z.ts")), 2);
}

#[test]
fn blocks_wrong_case_repo_root_into_the_frozen_app() {
    let f = Fixture::new("wrong-case-root");
    if !f.case_insensitive() {
        eprintln!("skip: case-sensitive filesystem");
        return;
    }
    assert_eq!(f.edit(&f.in_tmp("REPO/apps/web/x.ts")), 2);
}

#[test]
fn allows_listed_file_through_a_wrong_case_root() {
    let f = Fixture::new("wrong-case-root-listed");
    if !f.case_insensitive() {
        eprintln!("skip: case-sensitive filesystem");
        return;
    }
    assert_eq!(f.edit(&f.in_tmp("REPO/Packages/Client/src/a.ts")), 0);
}

#[test]
fn blocks_newline_smuggling_a_listed_line() {
    let f = Fixture::new("newline");
    let file = format!(
        "{}\nfeature/allowed packages/client/src/a.ts",
        f.in_repo("packages/client/src/z.ts")
    );
    assert_eq!(f.edit(&file), 2);
}

#[test]
fn allows_path_outside_any_git_tree() {
    let f = Fixture::new("loose");
    assert_eq!(f.edit(&f.in_tmp("loose/packages/client/a.ts")), 0);
}

#[test]
fn takes_the_branch_from_the_file_not_the_cwd() {
    let f = Fixture::new("cwd-other-branch");
    let payload = Fixture::edit_payload(&f.in_repo("packages/client/src/a.ts"));
    assert_eq!(f.call(&payload, Some(&f.allow), &f.other).code, 0);
}

#[test]
fn deny_message_names_the_resolved_path() {
    let f = Fixture::new("deny-message");
    let payload = Fixture::edit_payload(&f.in_repo("apps/api/../web/x.ts"));
    let out = f.call(&payload, Some(&f.allow), f.tmp.path());
    assert!(
        out.stderr.contains("BLOCKED: apps/web/x.ts "),
        "stderr: {}",
        out.stderr
    );
}

#[test]
fn blocks_notebook_edit_on_an_unlisted_frozen_path() {
    let f = Fixture::new("notebook-frozen");
    assert_eq!(f.notebook_edit(&f.in_repo("apps/web/n.ipynb")), 2);
}

#[test]
fn allows_notebook_edit_outside_the_freeze() {
    let f = Fixture::new("notebook-unfrozen");
    assert_eq!(f.notebook_edit(&f.in_repo("packages/db/n.ipynb")), 0);
}

#[test]
fn blocks_payload_that_is_not_json() {
    let f = Fixture::new("not-json");
    assert_eq!(f.call("not json", Some(&f.allow), f.tmp.path()).code, 2);
}
