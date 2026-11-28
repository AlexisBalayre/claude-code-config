mod common;

use common::{git, hook, run, write, Output, TempDir};
use serde_json::json;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

const PROFILE: &str = "SOURCE_EXTENSIONS=\"ts\"\n\
                       FORMAT_FIX_CMD=\"stub fix\"\n\
                       LINT_CMD=\"stub lint\"\n\
                       TYPECHECK_CMD=\"stub typecheck\"\n";

const STUB: &str = r#"#!/usr/bin/env bash
echo "$(pwd -P) stub $*" >>"$LOG"
[ "${STUB_FAIL:-}" = "$1" ] && { seq 200; exit 1; }
exit 0
"#;

const NOT_JSON: &str = "not json";

fn script(path: &Path, text: &str) {
    write(path, text);
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))
        .expect("make the script executable");
}

/// Create a repository on `main` holding one source file and the stub profile.
fn repo(dir: &Path, source: &str) {
    std::fs::create_dir_all(dir).expect("create the repository directory");
    git(dir, &["init", "-q", "-b", "main"]);
    write(&dir.join(source), "export const a = 1;\n");
    write(&dir.join(".claude/project.env"), PROFILE);
}

fn stop(cwd: &Path) -> String {
    json!({"cwd": cwd.to_str().expect("a UTF-8 path"), "stop_hook_active": false}).to_string()
}

struct Fixture {
    _tmp: TempDir,
    root: PathBuf,
    main: PathBuf,
    wt: PathBuf,
}

impl Fixture {
    /// Build a main checkout and a worktree of it, each with one dirty source file.
    fn new(test: &str) -> Fixture {
        let tmp = TempDir::new(test);
        // The stub logs `pwd -P`, so every expected path is the resolved one.
        let root = tmp
            .path()
            .canonicalize()
            .expect("resolve the scratch directory");
        // Written before the git setup: a script still open for writing in a sibling
        // test's forked child cannot be executed, and the setup outlasts that window.
        script(&root.join("bin/stub"), STUB);
        let (main, wt) = (root.join("main"), root.join("wt"));
        repo(&main, "a.ts");
        git(&main, &["add", "-A"]);
        git(&main, &["commit", "-qm", "init"]);
        git(
            &main,
            &["worktree", "add", "-q", "-b", "feature/x", "../wt"],
        );
        write(&wt.join("b.ts"), "export const b = 2;\n");
        write(&main.join("c.ts"), "export const c = 3;\n");
        Fixture {
            _tmp: tmp,
            root,
            main,
            wt,
        }
    }

    fn path_with(&self, first: &str) -> String {
        let inherited = std::env::var("PATH").unwrap_or_default();
        format!("{}:{inherited}", self.root.join(first).display())
    }

    fn hook(&self) -> Command {
        let mut command = hook("quality-checks");
        command
            .env("PATH", self.path_with("bin"))
            .env("LOG", self.root.join("calls.log"))
            .env("CLAUDE_PROJECT_DIR", &self.main);
        command
    }

    fn log(&self) -> String {
        std::fs::read_to_string(self.root.join("calls.log")).unwrap_or_default()
    }

    /// Run the hook on `stdin` and return the stub's call log.
    fn log_after(&self, stdin: &str) -> String {
        run(&mut self.hook(), stdin);
        self.log()
    }

    fn failing(&self, step: &str) -> Output {
        run(self.hook().env("STUB_FAIL", step), &stop(&self.wt))
    }
}

fn has_line(log: &str, line: &str) -> bool {
    log.lines().any(|logged| logged == line)
}

fn lints_in(log: &str, dir: &Path) -> bool {
    let prefix = format!("{} stub lint", dir.display());
    log.lines().any(|logged| logged.starts_with(&prefix))
}

fn runs_in(log: &str, dir: &Path) -> bool {
    let prefix = format!("{} ", dir.display());
    log.lines().any(|logged| logged.starts_with(&prefix))
}

#[test]
fn worktree_cwd_lints_its_dirty_file_in_the_worktree() {
    let fixture = Fixture::new("wt-lint");
    let log = fixture.log_after(&stop(&fixture.wt));
    let want = format!("{} stub lint b.ts", fixture.wt.display());
    assert!(has_line(&log, &want), "want {want:?} in {log:?}");
}

#[test]
fn worktree_cwd_formats_its_dirty_file_in_the_worktree() {
    let fixture = Fixture::new("wt-format");
    let log = fixture.log_after(&stop(&fixture.wt));
    let want = format!("{} stub fix b.ts", fixture.wt.display());
    assert!(has_line(&log, &want), "want {want:?} in {log:?}");
}

#[test]
fn worktree_cwd_never_runs_the_typecheck() {
    let fixture = Fixture::new("wt-no-typecheck");
    let log = fixture.log_after(&stop(&fixture.wt));
    assert!(
        !log.is_empty() && !log.contains(" stub typecheck"),
        "{log:?}"
    );
}

#[test]
fn cwd_outside_a_git_tree_falls_back_to_the_main_checkout() {
    let fixture = Fixture::new("no-git-cwd");
    let log = fixture.log_after(&stop(&fixture.root.join("bin")));
    assert!(lints_in(&log, &fixture.main), "{log:?}");
}

#[test]
fn sibling_repository_cwd_falls_back_to_the_main_checkout() {
    let fixture = Fixture::new("sibling");
    let sibling = fixture.root.join("sibling");
    repo(&sibling, "s.ts");
    let log = fixture.log_after(&stop(&sibling));
    assert!(!runs_in(&log, &sibling), "ran in the sibling: {log:?}");
    assert!(lints_in(&log, &fixture.main), "{log:?}");
}

#[test]
fn main_cwd_lints_in_main_without_typecheck() {
    let fixture = Fixture::new("main-cwd");
    let log = fixture.log_after(&stop(&fixture.main));
    assert!(lints_in(&log, &fixture.main), "{log:?}");
    assert!(!log.contains(" stub typecheck"), "{log:?}");
}

#[test]
fn main_checkout_path_with_a_space_is_linted() {
    let fixture = Fixture::new("spaced");
    let spaced = fixture.root.join("my main");
    repo(&spaced, "d.ts");
    run(
        fixture.hook().env("CLAUDE_PROJECT_DIR", &spaced),
        &stop(&spaced),
    );
    let log = fixture.log();
    assert!(lints_in(&log, &spaced), "{log:?}");
}

#[test]
fn git_that_echoes_path_format_still_falls_back_from_a_sibling() {
    let fixture = Fixture::new("old-git");
    let real_git = std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default())
        .map(|dir| dir.join("git"))
        .find(|candidate| candidate.is_file())
        .expect("git on PATH");
    // git before 2.31 prints --path-format back and exits 0.
    let old_git = format!(
        "#!/usr/bin/env bash\n\
         args=()\n\
         for a in \"$@\"; do case $a in --path-format*) echo \"$a\" ;; *) args+=(\"$a\") ;; esac; done\n\
         exec '{}' \"${{args[@]}}\"\n",
        real_git.display()
    );
    script(&fixture.root.join("oldgit/git"), &old_git);
    let sibling = fixture.root.join("sibling");
    repo(&sibling, "s.ts");
    let path = format!(
        "{}:{}",
        fixture.root.join("oldgit").display(),
        fixture.path_with("bin")
    );
    run(fixture.hook().env("PATH", path), &stop(&sibling));
    let log = fixture.log();
    assert!(!runs_in(&log, &sibling), "ran in the sibling: {log:?}");
    assert!(lints_in(&log, &fixture.main), "{log:?}");
}

#[test]
fn payload_without_cwd_falls_back_to_the_project_dir() {
    let fixture = Fixture::new("no-cwd");
    let log = fixture.log_after("{}");
    assert!(lints_in(&log, &fixture.main), "{log:?}");
}

#[test]
fn non_json_stdin_falls_back_to_the_project_dir() {
    let fixture = Fixture::new("not-json");
    let log = fixture.log_after(NOT_JSON);
    assert!(lints_in(&log, &fixture.main), "{log:?}");
}

#[test]
fn failing_lint_exits_2_with_at_most_40_output_lines() {
    let fixture = Fixture::new("lint-fails-tail");
    let out = fixture.failing("lint");
    let numbers = out
        .stderr
        .lines()
        .filter(|line| !line.is_empty() && line.bytes().all(|byte| byte.is_ascii_digit()))
        .count();
    assert_eq!(out.code, 2, "stderr: {}", out.stderr);
    assert!(numbers <= 40, "{numbers} output lines kept");
}

#[test]
fn failing_lint_prints_the_last_40_lines_then_the_verdict() {
    let fixture = Fixture::new("lint-fails-text");
    let out = fixture.failing("lint");
    let numbers: Vec<String> = (161..=200).map(|number| number.to_string()).collect();
    let mut want = numbers.join("\n");
    want += "\nLint failed on the files this session touched (stub lint). Fix the remaining issues above.\n";
    assert_eq!(out.stderr, want);
    assert_eq!(out.stdout, "");
}

#[test]
fn failing_formatter_is_silent() {
    let fixture = Fixture::new("format-fails");
    let out = fixture.failing("fix");
    assert_eq!(out.code, 0, "stderr: {}", out.stderr);
    assert_eq!(out.stdout + &out.stderr, "");
}

#[test]
fn modified_untracked_and_staged_sources_go_to_one_sorted_call() {
    let fixture = Fixture::new("dirty-set");
    write(&fixture.wt.join("a.ts"), "export const a = 2;\n");
    write(&fixture.wt.join("z.ts"), "export const z = 1;\n");
    git(&fixture.wt, &["add", "z.ts"]);
    write(&fixture.wt.join("notes.md"), "notes\n");
    let log = fixture.log_after(&stop(&fixture.wt));
    let want = format!("{} stub lint a.ts b.ts z.ts", fixture.wt.display());
    assert!(has_line(&log, &want), "want {want:?} in {log:?}");
}

#[test]
fn no_dirty_source_file_runs_nothing() {
    let fixture = Fixture::new("clean");
    std::fs::remove_file(fixture.wt.join("b.ts")).expect("remove the dirty file");
    let log = fixture.log_after(&stop(&fixture.wt));
    assert_eq!(log, "");
}

#[test]
fn deleted_tracked_source_is_not_passed_to_the_tools() {
    let fixture = Fixture::new("deleted");
    std::fs::remove_file(fixture.wt.join("a.ts")).expect("delete the tracked file");
    let log = fixture.log_after(&stop(&fixture.wt));
    let want = format!("{} stub lint b.ts", fixture.wt.display());
    assert!(has_line(&log, &want), "want {want:?} in {log:?}");
}
