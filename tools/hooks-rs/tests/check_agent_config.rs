mod common;

use common::{git, hook, run, write, Output, TempDir};
use std::fs;
use std::path::PathBuf;

const BROKEN: &str = "if true; then\n";
const OK: &str = "echo ok\n";
const LT: &str = ".claude/skills/setup-agent-env/lead-tools";
const LEAD: &str = "echo lead >>\"$MARK\"\n";
const RULES: &str = "echo rules >>\"$MARK\"\n";
// The fixture's own harness: each run appends this line to $MARK.
const HELD: &str = "held=\n";

struct Repo(TempDir);

impl Repo {
    fn new(test: &str) -> Repo {
        let repo = Repo(TempDir::new(test));
        git(repo.0.path(), &["init", "-q", "r"]);
        let mark =
            "echo \"held=${GATE_LOCK_HELD:-}${AGENT_REPO:-}${AGENT_GH_REPO:-}\" >>\"$MARK\"\n";
        repo.commit(&[
            ("README.md", "base\n"),
            ("tests/scripts/mark.test.sh", mark),
        ]);
        repo
    }

    /// A repository whose last commit holds `next`, on top of one holding `earlier`.
    fn changed(test: &str, earlier: &[(&str, &str)], next: &[(&str, &str)]) -> (Repo, Output) {
        let repo = Repo::new(test);
        if !earlier.is_empty() {
            repo.commit(earlier);
        }
        repo.commit(next);
        let out = repo.check(&[]);
        (repo, out)
    }

    fn dir(&self) -> PathBuf {
        self.0.path().join("r")
    }

    fn mark_file(&self) -> PathBuf {
        self.0.path().join("mark")
    }

    fn commit(&self, files: &[(&str, &str)]) {
        for (path, text) in files {
            write(&self.dir().join(path), text);
        }
        git(&self.dir(), &["add", "-A"]);
        git(&self.dir(), &["commit", "-qm", "next"]);
    }

    /// A harness that appends its own name to $MARK, then runs `extra`.
    fn harness(&self, path: &str, extra: &str) {
        let name = path.rsplit('/').next().unwrap_or(path);
        write(
            &self.dir().join(path),
            &format!("echo {name} >>\"$MARK\"\n{extra}\n"),
        );
    }

    fn command(&self, flags: &[&str]) -> std::process::Command {
        let _ = fs::remove_file(self.mark_file());
        let mut command = hook("check-agent-config");
        command
            .args(flags)
            .args(["HEAD^1", "HEAD"])
            .current_dir(self.dir());
        command.env("MARK", self.mark_file());
        command
    }

    fn check(&self, flags: &[&str]) -> Output {
        run(&mut self.command(flags), "")
    }

    /// What the harnesses of the last check wrote, one line per run.
    fn mark(&self) -> String {
        fs::read_to_string(self.mark_file()).unwrap_or_default()
    }

    /// Commit one file, run with --affected and return the sorted names of the harnesses that ran.
    fn affected_by(&self, path: &str, text: &str) -> Vec<String> {
        self.commit(&[(path, text)]);
        let out = self.check(&["--affected"]);
        assert_eq!(out.code, 0, "{path}: {}", out.stderr);
        let mut ran: Vec<String> = self.mark().lines().map(String::from).collect();
        ran.sort();
        ran
    }
}

#[test]
fn fails_on_a_changed_script_with_a_syntax_error() {
    let (_repo, out) = Repo::changed("broken", &[], &[("a.sh", BROKEN)]);
    assert_eq!(out.code, 1);
    assert!(
        out.stderr
            .contains("check-agent-config: bash -n failed on a.sh"),
        "{}",
        out.stderr
    );
}

#[test]
fn passes_a_broken_script_the_range_leaves_unchanged() {
    let (_repo, out) = Repo::changed(
        "unchanged",
        &[("a.sh", BROKEN)],
        &[("README.md", "edited\n")],
    );
    assert_eq!(out.code, 0, "{}", out.stderr);
}

#[test]
fn reports_every_broken_script() {
    let (_repo, out) = Repo::changed("two-broken", &[], &[("a.sh", BROKEN), ("b.sh", BROKEN)]);
    assert_eq!(out.code, 1);
    assert_eq!(
        out.stderr.matches("bash -n failed").count(),
        2,
        "{}",
        out.stderr
    );
    assert!(
        out.stderr.ends_with("check-agent-config: 2 failure(s)\n"),
        "{}",
        out.stderr
    );
}

#[test]
fn checks_an_extensionless_script_by_its_shebang() {
    let shell = "#!/usr/bin/env bash\nif true; then\n";
    let (_repo, out) = Repo::changed("shebang", &[], &[("scripts/x", shell)]);
    assert_eq!(out.code, 1);

    let (repo, out) = Repo::changed(
        "no-shebang",
        &[],
        &[("scripts/data", "name\nif true; then\n")],
    );
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert_eq!(
        repo.mark(),
        HELD,
        "any file under scripts/ runs the harnesses"
    );
}

#[test]
fn runs_the_harnesses_for_a_change_that_can_break_them() {
    let rows: [(&str, &str, &str); 4] = [
        ("workflow", ".github/workflows/ci.yml", "name: ci\n"),
        ("script", "scripts/ok.sh", OK),
        ("hook", ".claude/hooks/h.sh", OK),
        ("stray-harness", "tools/x.test.sh", OK),
    ];
    for (test, path, text) in rows {
        let (repo, out) = Repo::changed(test, &[], &[(path, text)]);
        assert_eq!(out.code, 0, "{path}: {}", out.stderr);
        assert_eq!(repo.mark(), HELD, "{path}");
        assert!(
            out.stdout.contains("tests/scripts/mark.test.sh passed ("),
            "{}",
            out.stdout
        );
    }
}

#[test]
fn runs_no_harness_for_a_change_outside_their_reach() {
    let rows = [
        ("hooks-readme", ".claude/hooks/README.md"),
        ("markdown", "docs/a.md"),
        ("project-test", "tests/unit/test_x.py"),
    ];
    for (test, path) in rows {
        let (repo, out) = Repo::changed(test, &[], &[(path, "text\n")]);
        assert_eq!(out.code, 0, "{path}: {}", out.stderr);
        assert_eq!(repo.mark(), "", "{path}");
    }
}

#[test]
fn runs_the_harnesses_for_a_deleted_script() {
    let repo = Repo::new("deleted");
    repo.commit(&[("scripts/gone.sh", OK)]);
    fs::remove_file(repo.dir().join("scripts/gone.sh")).expect("delete the script");
    repo.commit(&[]);
    assert_eq!(repo.check(&[]).code, 0);
    assert_eq!(repo.mark(), HELD);
}

#[test]
fn finds_a_new_harness_under_tests_hooks() {
    let (repo, out) = Repo::changed(
        "new-harness",
        &[],
        &[("tests/hooks/new.test.sh", "echo hooks >>\"$MARK\"\n")],
    );
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert!(
        repo.mark().lines().any(|line| line == "hooks"),
        "{}",
        repo.mark()
    );
}

#[test]
fn fails_on_a_failing_harness_and_prints_its_output() {
    let fail = (
        "tests/scripts/fail.test.sh",
        "echo first\necho second >&2\nexit 1\n",
    );
    let (_repo, out) = Repo::changed("failing", &[fail], &[("scripts/ok.sh", OK)]);
    assert_eq!(out.code, 1);
    let failed = out
        .stderr
        .find("check-agent-config: tests/scripts/fail.test.sh failed (");
    let body = &out.stderr[failed.expect("the failure line")..];
    assert!(
        body.contains("s):\n    first\n    second\n"),
        "{}",
        out.stderr
    );
    assert!(
        out.stderr.ends_with("check-agent-config: 1 failure(s)\n"),
        "{}",
        out.stderr
    );
}

// Three harnesses that each wait for the other two to start pass only when they run at once.
#[test]
fn runs_harnesses_at_once_and_prints_them_in_harness_order() {
    let repo = Repo::new("parallel");
    let seen = |name: &str| repo.0.path().join(name).display().to_string();
    let (a, b, c) = (seen("a"), seen("b"), seen("c"));
    let mut files = Vec::new();
    for name in ["a", "b", "c"] {
        let script = format!(
            "touch '{}'; for _ in $(seq 50); do [ -e '{a}' ] && [ -e '{b}' ] && [ -e '{c}' ] && exit 0; \
             sleep 0.1; done; exit 1\n",
            seen(name)
        );
        files.push((format!("tests/scripts/{name}.test.sh"), script));
    }
    files.push(("tests/scripts/z.test.sh".into(), "exit 1\n".into()));
    // Sorts first and finishes last, so a print in completion order puts it out of place.
    files.push(("tests/scripts/0.test.sh".into(), "sleep 1\nexit 1\n".into()));
    let files: Vec<(&str, &str)> = files
        .iter()
        .map(|(p, t)| (p.as_str(), t.as_str()))
        .collect();
    repo.commit(&files);
    let out = repo.check(&[]);
    assert_eq!(out.code, 1);
    let names = |text: &str, verdict: &str| -> Vec<String> {
        text.lines()
            .filter(|line| line.starts_with("check-agent-config: tests/") && line.contains(verdict))
            .map(|line| line.split(['/', ' ']).nth(3).unwrap_or_default().to_owned())
            .collect()
    };
    assert_eq!(
        names(&out.stdout, " passed ("),
        ["a.test.sh", "b.test.sh", "c.test.sh", "mark.test.sh"],
        "{}{}",
        out.stdout,
        out.stderr
    );
    assert_eq!(
        names(&out.stderr, " failed ("),
        ["0.test.sh", "z.test.sh"],
        "{}",
        out.stderr
    );
    assert!(
        out.stdout.lines().all(|line| line.ends_with("s)")),
        "{}",
        out.stdout
    );
}

#[test]
fn keeps_the_gate_lock_and_the_lead_overrides_out_of_the_harnesses() {
    let repo = Repo::new("held-env");
    repo.commit(&[(".claude/hooks/h.sh", OK)]);
    let mut command = repo.command(&[]);
    command
        .env("GATE_LOCK_HELD", "4242")
        .env("AGENT_REPO", "/real")
        .env("AGENT_GH_REPO", "o/r");
    assert_eq!(run(&mut command, "").code, 0);
    assert_eq!(repo.mark(), HELD);
}

#[test]
fn runs_the_lead_tools_harnesses_for_a_lead_tools_change_alone() {
    let lead = (format!("{LT}/l.test.sh"), LEAD);
    let lead = [(lead.0.as_str(), lead.1)];
    let readme = format!("{LT}/README.md");

    let (repo, out) = Repo::changed("lead-only", &lead, &[(&readme, "text\n")]);
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert_eq!(repo.mark(), "lead\n");

    let params = ".claude/skills/setup-agent-env/params/repo.env";
    let (repo, _) = Repo::changed("lead-params", &lead, &[(params, "X=1\n")]);
    assert_eq!(repo.mark(), "lead\n");

    let (repo, _) = Repo::changed("lead-scripts", &lead, &[("scripts/ok.sh", OK)]);
    assert_eq!(repo.mark(), HELD);

    let both = [(readme.as_str(), "text\n"), ("scripts/ok.sh", OK)];
    let (repo, _) = Repo::changed("lead-both", &lead, &both);
    let mark = repo.mark();
    let mut ran: Vec<&str> = mark.lines().collect();
    ran.sort();
    assert_eq!(ran, ["held=", "lead"]);

    let skill = ".claude/skills/setup-agent-env/SKILL.md";
    let (repo, out) = Repo::changed("lead-elsewhere", &lead, &[(skill, "text\n")]);
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert_eq!(repo.mark(), "");

    let fail = format!("{LT}/fail.test.sh");
    let (_repo, out) = Repo::changed(
        "lead-failing",
        &[(&fail, "exit 1\n")],
        &[(&readme, "text\n")],
    );
    assert_eq!(out.code, 1);
}

#[test]
fn runs_the_rules_harness_for_a_conventions_change() {
    let rules = [("tests/scripts/conventions-rules.test.sh", RULES)];
    let (repo, out) = Repo::changed(
        "rules-doc",
        &rules,
        &[("docs/conventions/core.md", "text\n")],
    );
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert_eq!(repo.mark(), "rules\n");

    let both = [("scripts/ok.sh", OK), (".claude/rules/core.md", "text\n")];
    let (repo, _) = Repo::changed("rules-full-set", &rules, &both);
    assert_eq!(repo.mark().matches("rules\n").count(), 1, "{}", repo.mark());
    assert!(repo.mark().contains(HELD), "{}", repo.mark());
}

// --affected, verify's run, keeps only the harnesses the range can affect.
#[test]
fn keeps_only_the_affected_harnesses() {
    let repo = Repo::new("affected");
    for path in ["tests/scripts/a.test.sh", "tests/scripts/pre-push.test.sh"] {
        repo.harness(path, "");
    }
    repo.harness("tests/scripts/b.test.sh", ": lib.sh");
    repo.harness("tests/scripts/c.test.sh", "");
    repo.harness("tests/scripts/conventions-rules.test.sh", "");
    repo.harness("tests/hooks/h.test.sh", "");
    repo.harness(&format!("{LT}/l.test.sh"), ": lib.sh");
    repo.harness(&format!("{LT}/m.test.sh"), "");
    repo.commit(&[("tests/scripts/c.test.mjs", "// scripts/helper.sh\n")]);

    let rows: [(&str, &str, &[&str]); 11] = [
        ("scripts/a.sh", OK, &["a.test.sh"]),
        (
            "scripts/pre-push",
            "#!/bin/bash\necho ok\n",
            &["pre-push.test.sh"],
        ),
        ("scripts/lib.sh", OK, &["b.test.sh"]),
        ("scripts/helper.sh", OK, &["c.test.sh"]),
        (
            "tests/scripts/b.test.sh",
            "echo b.test.sh >>\"$MARK\"\n",
            &["b.test.sh"],
        ),
        (".github/workflows/ci.yml", "name: ci\n", &[]),
        (
            "docs/conventions/core.md",
            "text\n",
            &["conventions-rules.test.sh"],
        ),
        (
            ".claude/rules/core.md",
            "text\n",
            &["conventions-rules.test.sh"],
        ),
        (".claude/hooks/h.sh", OK, &["h.test.sh"]),
        (
            ".claude/skills/setup-agent-env/params/repo.env",
            "X=2\n",
            &["l.test.sh", "m.test.sh"],
        ),
        ("scripts/other.sh", OK, &[]),
    ];
    for (path, text, want) in rows {
        assert_eq!(repo.affected_by(path, text), want, "{path}");
    }
    assert_eq!(repo.affected_by(&format!("{LT}/lib.sh"), OK), ["l.test.sh"]);

    repo.commit(&[("scripts/a.sh", "echo again\n")]);
    assert_eq!(repo.check(&[]).code, 0);
    assert!(
        repo.mark().contains("b.test.sh\n"),
        "without --affected every harness runs"
    );
}

// Five harnesses, four at a time: the one with the shortest last time starts alone, after the
// rest. z has no time yet, so it must count as the longest, not as 0.
#[test]
fn starts_the_longest_harness_first_and_records_each_time() {
    let repo = Repo::new("times");
    for name in ["a", "b", "c", "mark", "z"] {
        repo.harness(&format!("tests/scripts/{name}.test.sh"), "sleep 2");
    }
    repo.commit(&[]);
    let times = repo.dir().join(".git/check-agent-config.times");
    for (name, secs) in [("a", "2\n"), ("b", "4\n"), ("c", "3\n"), ("mark", "1\n")] {
        write(&times.join(format!("tests_scripts_{name}.test.sh")), secs);
    }
    repo.commit(&[("scripts/ok.sh", OK)]);
    assert_eq!(repo.check(&[]).code, 0);
    assert_eq!(
        repo.mark().lines().last(),
        Some("mark.test.sh"),
        "{}",
        repo.mark()
    );
    let recorded = fs::read_to_string(times.join("tests_scripts_z.test.sh")).unwrap_or_default();
    assert!(recorded.trim_end().parse::<u64>().is_ok(), "{recorded:?}");
}

#[test]
fn asks_for_both_revisions() {
    let repo = Repo::new("usage");
    let out = run(
        hook("check-agent-config")
            .arg("HEAD")
            .current_dir(repo.dir()),
        "",
    );
    assert_eq!(out.code, 1);
    assert!(
        out.stderr.starts_with("check-agent-config: usage: "),
        "{}",
        out.stderr
    );
}
