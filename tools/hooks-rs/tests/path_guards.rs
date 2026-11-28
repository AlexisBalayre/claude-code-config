mod common;

use common::{hook, run, write, Output, TempDir};
use serde_json::json;
use std::path::{Path, PathBuf};

const GENERATED: &str = "protect-generated";
const NAMING: &str = "validate-file-naming";

struct Fixture {
    tmp: TempDir,
    proj: PathBuf,
    sibling: PathBuf,
}

impl Fixture {
    fn new(test: &str) -> Fixture {
        let tmp = TempDir::new(test);
        let proj = tmp.path().join("proj");
        let sibling = tmp.path().join("sibling");
        std::fs::create_dir_all(proj.join("src/gen")).expect("create src/gen");
        std::fs::create_dir_all(sibling.join("src")).expect("create the sibling");
        write(
            &proj.join(".claude/project.env"),
            concat!(
                "GENERATED_PATHS_REGEX='(^|/)src/gen/|\\.generated\\.ts$'\n",
                "FILE_NAMING_SCOPE_REGEX='^src/'\n",
                "FILE_NAMING_REGEX='^[a-z0-9.-]+$'\n",
                "FILE_NAMING_HINT=\"Use kebab-case.\"\n",
            ),
        );
        write(&proj.join("src/LegacyName.ts"), "x\n");
        Fixture { tmp, proj, sibling }
    }

    fn in_proj(&self, rel: &str) -> String {
        format!("{}/{rel}", self.proj.display())
    }

    fn call(&self, subcommand: &str, input: &str, project: Option<&Path>) -> Output {
        let mut command = hook(subcommand);
        command.current_dir(self.tmp.path());
        if let Some(project) = project {
            command.env("CLAUDE_PROJECT_DIR", project);
        }
        run(&mut command, input)
    }

    fn write_in(&self, subcommand: &str, file: &str, project: Option<&Path>) -> i32 {
        let payload = json!({"tool_name": "Write", "tool_input": {"file_path": file}});
        self.call(subcommand, &payload.to_string(), project).code
    }

    fn write(&self, subcommand: &str, file: &str) -> i32 {
        self.write_in(subcommand, file, Some(&self.proj))
    }
}

#[test]
fn blocks_file_in_a_generated_directory() {
    let f = Fixture::new("generated-dir");
    assert_eq!(f.write(GENERATED, &f.in_proj("src/gen/api.ts")), 2);
}

#[test]
fn blocks_file_with_a_generated_suffix() {
    let f = Fixture::new("generated-suffix");
    assert_eq!(f.write(GENERATED, &f.in_proj("src/types.generated.ts")), 2);
}

#[test]
fn allows_ordinary_source_file() {
    let f = Fixture::new("ordinary-source");
    assert_eq!(f.write(GENERATED, &f.in_proj("src/api.ts")), 0);
}

#[test]
fn treats_nothing_as_generated_without_project_env() {
    let f = Fixture::new("no-project-env");
    assert_eq!(
        f.write_in(GENERATED, &f.in_proj("src/gen/api.ts"), Some(&f.sibling)),
        0
    );
}

#[test]
fn allows_payload_without_a_path() {
    let f = Fixture::new("no-path");
    assert_eq!(f.write(GENERATED, ""), 0);
}

#[test]
fn allows_new_file_with_a_good_name() {
    let f = Fixture::new("good-name");
    assert_eq!(f.write(NAMING, &f.in_proj("src/user-card.ts")), 0);
}

#[test]
fn blocks_new_file_with_a_bad_name() {
    let f = Fixture::new("bad-name");
    assert_eq!(f.write(NAMING, &f.in_proj("src/UserCard.ts")), 2);
}

#[test]
fn allows_existing_file_to_keep_its_legacy_name() {
    let f = Fixture::new("legacy-name");
    assert_eq!(f.write(NAMING, &f.in_proj("src/LegacyName.ts")), 0);
}

#[test]
fn allows_bad_name_outside_the_scope() {
    let f = Fixture::new("outside-scope");
    assert_eq!(f.write(NAMING, &f.in_proj("docs/ReadMe.md")), 0);
}

#[test]
fn allows_bad_name_in_a_sibling_repo() {
    let f = Fixture::new("sibling-repo");
    let file = format!("{}/src/UserCard.ts", f.sibling.display());
    assert_eq!(f.write(NAMING, &file), 0);
}

#[test]
fn allows_bad_name_under_a_lookalike_prefix_of_the_project_dir() {
    let f = Fixture::new("lookalike-prefix");
    let file = format!("{}-x/src/UserCard.ts", f.proj.display());
    assert_eq!(f.write(NAMING, &file), 0);
}

#[test]
fn allows_bad_name_when_the_project_dir_is_unset() {
    let f = Fixture::new("project-unset");
    assert_eq!(f.write_in(NAMING, &f.in_proj("src/UserCard.ts"), None), 0);
}

#[test]
fn checks_nothing_without_naming_keys() {
    let f = Fixture::new("no-naming-keys");
    let file = format!("{}/src/UserCard.ts", f.sibling.display());
    assert_eq!(f.write_in(NAMING, &file, Some(&f.sibling)), 0);
}

#[test]
fn naming_message_carries_the_hint() {
    let f = Fixture::new("naming-message");
    let payload = json!({"tool_input": {"file_path": f.in_proj("src/UserCard.ts")}});
    let out = f.call(NAMING, &payload.to_string(), Some(&f.proj));
    assert_eq!(
        out.stderr.trim_end_matches('\n'),
        "BLOCKED: file name 'UserCard.ts' does not match the project naming convention (^[a-z0-9.-]+$).\n\
         Use kebab-case.\n\
         Full naming rules: docs/conventions/core.md §Naming"
    );
}
