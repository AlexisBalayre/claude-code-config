mod common;

use common::{
    bash_payload, decision, executable, expect_decision, git, hook, path_with, run, write, TempDir,
};
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const PUSH_NEW: &str = "git push origin feature/new";
const PUSH_X: &str = "git push origin feature/x";

/// Hold a bare `remote.git` and a clone `w` whose `feature/x` is on it.
struct Fixture {
    tmp: TempDir,
    root: String,
}

impl Fixture {
    fn new(test: &str) -> Fixture {
        let tmp = TempDir::new(test);
        let root = tmp.path().to_str().expect("a UTF-8 temp path").to_owned();
        let fixture = Fixture { tmp, root };
        git(fixture.tmp.path(), &["init", "-q", "--bare", "remote.git"]);
        fixture.repo("w", &fixture.at("remote.git"));
        // feature/merged is pushed then deleted: a branch origin once had must still ask.
        fixture.git("w", &["branch", "feature/merged"]);
        let pushed = ["push", "-q", "origin", "feature/x", "feature/merged"];
        fixture.git("w", &pushed);
        fixture.git("w", &["push", "-q", "origin", "--delete", "feature/merged"]);
        fixture
    }

    fn at(&self, name: &str) -> String {
        format!("{}/{name}", self.root)
    }

    fn path(&self, name: &str) -> PathBuf {
        self.tmp.path().join(name)
    }

    fn git(&self, repo: &str, args: &[&str]) {
        git(&self.path(repo), args);
    }

    fn repo(&self, name: &str, origin: &str) {
        git(self.tmp.path(), &["init", "-q", "-b", "feature/x", name]);
        self.git(name, &["remote", "add", "origin", origin]);
        self.git(name, &["commit", "-q", "--allow-empty", "-m", "one"]);
        self.git(name, &["branch", "feature/new"]);
    }

    /// Build a gate that may probe the fixture's directories and the unreachable ssh host.
    fn gate(&self) -> Command {
        let mut gate = hook("push-gate");
        let remotes = format!("{}/ git@example.invalid:", self.root);
        gate.env("PUSH_GATE_REMOTES", remotes);
        gate
    }

    /// Add `w3`, which fetches from `remote.git` and pushes to an empty `fork.git`.
    fn fork_pusher(&self) {
        git(self.tmp.path(), &["init", "-q", "--bare", "fork.git"]);
        self.repo("w3", &self.at("remote.git"));
        let fork = self.at("fork.git");
        self.git("w3", &["remote", "set-url", "--push", "origin", &fork]);
    }

    /// Put an executable stub named `name` in the fixture directory `dir`.
    fn stub(&self, dir: &str, name: &str, script: &str) {
        executable(&self.path(dir).join(name), script);
    }

    /// Build a gate whose PATH finds the stubs of the fixture directory `dir` first.
    fn gate_with_stubs(&self, dir: &str) -> Command {
        let mut gate = self.gate();
        gate.env("PATH", path_with(&[&self.path(dir)]));
        gate
    }

    /// Build a gate whose approved plans list `feature/new` for `remote.git`.
    fn gate_with_plan(&self) -> Command {
        let plans = self.path("plans");
        let line = format!("feature/new {} KEY-1 12:00\n", self.at("remote.git"));
        write(&plans, &line);
        let mut gate = self.gate();
        gate.env("AGENT_APPROVED_PLANS", plans);
        gate
    }

    fn expect(&self, want: &str, repo: &str, command: &str, reason: &str) {
        expect_decision(&mut self.gate(), want, &self.at(repo), command, reason);
    }

    fn expect_planned(&self, want: &str, repo: &str, command: &str, reason: &str) {
        expect_decision(
            &mut self.gate_with_plan(),
            want,
            &self.at(repo),
            command,
            reason,
        );
    }

    fn assert_asks(&self, commands: &[&str]) {
        for command in commands {
            self.expect("ask", "w", command, "");
        }
    }
}

#[test]
fn defers_for_a_call_that_is_no_git_push() {
    let fixture = Fixture::new("no-git-p");
    for command in [
        "git status",
        r#"git commit -m "fix the gate""#,
        "docker push registry/image",
    ] {
        fixture.expect("none", "w", command, "");
    }
}

#[test]
fn defers_for_a_branch_already_on_origin() {
    let fixture = Fixture::new("on-origin");
    for command in [
        PUSH_X,
        "git push -u origin feature/x",
        "git push --set-upstream origin feature/x",
    ] {
        fixture.expect("none", "w", command, "");
    }
    let absolute = format!("git -C {} push origin feature/x", fixture.at("w"));
    expect_decision(&mut fixture.gate(), "none", "/", &absolute, "");
    let relative = "git -C w push origin feature/x";
    expect_decision(&mut fixture.gate(), "none", &fixture.root, relative, "");
}

#[test]
fn reads_the_repo_dash_c_names_not_the_cwd() {
    let fixture = Fixture::new("dash-c");
    fixture.repo("w2", &fixture.at("unreachable.git"));
    let reachable = format!("git -C {} push origin feature/x", fixture.at("w"));
    fixture.expect("none", "w2", &reachable, "");
    let unreachable = format!("git -C {} push origin feature/x", fixture.at("w2"));
    fixture.expect("ask", "w", &unreachable, "");
}

#[test]
fn asks_for_a_branch_not_on_origin() {
    let fixture = Fixture::new("not-on-origin");
    fixture.repo("w2", &fixture.at("unreachable.git"));
    fixture.expect("ask", "w", PUSH_NEW, "feature/new");
    fixture.expect("ask", "w", "git push -u origin feature/new", "feature/new");
    fixture.expect(
        "ask",
        "w",
        "git push origin feature/merged",
        "feature/merged",
    );
    fixture.expect("ask", "w2", PUSH_X, "feature/x");
    expect_decision(&mut fixture.gate(), "ask", "/nonexistent", PUSH_X, "");
}

#[test]
fn asks_for_a_branch_only_on_the_fetch_url() {
    let fixture = Fixture::new("fetch-url-only");
    fixture.fork_pusher();
    fixture.expect("ask", "w3", PUSH_X, "feature/x");
}

#[test]
fn asks_for_a_branch_missing_from_one_of_several_urls() {
    let fixture = Fixture::new("several-urls");
    git(fixture.tmp.path(), &["init", "-q", "--bare", "fork.git"]);
    fixture.repo("w4", &fixture.at("remote.git"));
    for url in ["remote.git", "fork.git"] {
        let url = fixture.at(url);
        fixture.git(
            "w4",
            &["remote", "set-url", "--add", "--push", "origin", &url],
        );
    }
    fixture.expect("ask", "w4", PUSH_X, "feature/x");
}

#[test]
fn defers_for_an_approved_branch_not_yet_on_origin() {
    Fixture::new("approved").expect_planned("none", "w", PUSH_NEW, "");
}

#[test]
fn asks_for_an_approved_branch_aimed_at_another_url() {
    let fixture = Fixture::new("approved-other-url");
    fixture.repo("w2", &fixture.at("unreachable.git"));
    fixture.fork_pusher();
    fixture.expect_planned("ask", "w2", "git push -u origin feature/new", "feature/new");
    fixture.expect_planned("ask", "w3", PUSH_NEW, "feature/new");
}

#[test]
fn asks_for_a_branch_or_shape_the_plans_do_not_list() {
    let fixture = Fixture::new("not-approved");
    fixture.expect_planned("ask", "w", "git push origin feature/ne", "feature/ne");
    fixture.expect_planned(
        "ask",
        "w",
        "git push origin feature/merged",
        "feature/merged",
    );
    fixture.expect_planned("ask", "w", "git push origin feature/new && echo done", "");
}

#[test]
fn reads_a_plans_line_without_its_newline() {
    let fixture = Fixture::new("plans-no-newline");
    let plans = fixture.path("plans-nonl");
    write(&plans, &format!("feature/new {}", fixture.at("remote.git")));
    let mut gate = fixture.gate();
    gate.env("AGENT_APPROVED_PLANS", plans);
    expect_decision(&mut gate, "none", &fixture.at("w"), PUSH_NEW, "");
}

#[test]
fn defers_for_a_planned_url_without_the_remotes_variable() {
    let fixture = Fixture::new("plans-unlisted");
    let plans = fixture.path("plans-bare");
    write(
        &plans,
        &format!("feature/new {}\n", fixture.at("remote.git")),
    );
    let mut gate = hook("push-gate");
    gate.env("AGENT_APPROVED_PLANS", plans);
    expect_decision(&mut gate, "none", &fixture.at("w"), PUSH_NEW, "");
}

#[test]
fn asks_when_config_sends_an_approved_branch_to_another_ref() {
    let fixture = Fixture::new("remap");
    for (key, value) in [
        ("remote.origin.push", "refs/heads/feature/x:refs/heads/main"),
        ("remote.origin.mirror", "true"),
        ("push.default", "upstream"),
        ("push.default", "tracking"),
    ] {
        fixture.git("w", &["config", key, value]);
        let out = run(
            &mut fixture.gate_with_plan(),
            &bash_payload(&fixture.at("w"), PUSH_NEW),
        );
        assert_eq!(decision(&out), "ask", "setting: {key} {value}");
        assert!(out.stdout.contains("another ref"), "setting: {key} {value}");
        fixture.git("w", &["config", "--unset", key]);
    }
}

// git pushes a symbolic ref to the branch it points at, which an approved name could make main.
#[test]
fn asks_for_a_branch_that_is_a_symbolic_ref() {
    let fixture = Fixture::new("symref");
    let (alias, target) = ("refs/heads/feature/new", "refs/heads/feature/x");
    fixture.git("w", &["symbolic-ref", alias, target]);
    fixture.expect_planned("ask", "w", PUSH_NEW, "not the local branch");
}

// git reads `heads/feature/x` as the branch feature/x and pushes to that name, while origin
// lists a branch named heads/feature/x.
#[test]
fn asks_for_another_spelling_of_a_local_branch() {
    let fixture = Fixture::new("spelling");
    let listed = "feature/x:refs/heads/heads/feature/x";
    fixture.git("w", &["push", "-q", "origin", listed]);
    let push = "git push origin heads/feature/x";
    fixture.expect("ask", "w", push, "not the local branch");
}

// The check runs before the listed and on-origin paths split.
#[test]
fn asks_when_a_mirror_remaps_a_branch_already_on_origin() {
    let fixture = Fixture::new("mirror-on-origin");
    fixture.git("w", &["config", "remote.origin.mirror", "true"]);
    fixture.expect_planned("ask", "w", PUSH_X, "another ref");
}

#[test]
fn ignores_the_mapping_and_mirror_of_another_remote() {
    let fixture = Fixture::new("other-remote");
    let mapping = "refs/heads/feature/x:refs/heads/main";
    fixture.git("w", &["config", "remote.other.push", mapping]);
    fixture.git("w", &["config", "remote.other.mirror", "true"]);
    fixture.expect_planned("none", "w", PUSH_NEW, "");
}

#[test]
fn takes_the_last_push_default_as_git_does() {
    let fixture = Fixture::new("last-default");
    fixture.git("w", &["config", "push.default", "upstream"]);
    fixture.git("w", &["config", "--add", "push.default", "simple"]);
    fixture.expect_planned("none", "w", PUSH_NEW, "");
}

#[test]
fn reads_no_key_from_a_config_value_holding_a_newline() {
    let fixture = Fixture::new("newline-value");
    fixture.git("w", &["config", "push.default", "upstream"]);
    fixture.git("w", &["config", "alias.x", "a\npush.default=simple"]);
    fixture.expect_planned("ask", "w", PUSH_NEW, "another ref");
}

// git config never shows the push lines of these files, and a linked worktree, where
// agents push from, reads them from the common dir.
#[test]
fn asks_when_a_legacy_remote_file_remaps_the_push() {
    let fixture = Fixture::new("legacy-files");
    let remote = fixture.at("remote.git");
    let remotes_file = fixture.path("w/.git/remotes/origin");
    let mapping = "refs/heads/feature/x:refs/heads/main";
    write(&remotes_file, &format!("URL: {remote}\nPush: {mapping}\n"));
    fixture.expect_planned("ask", "w", PUSH_NEW, "another ref");
    std::fs::remove_file(&remotes_file).expect("remove remotes/origin");

    let branches_file = fixture.path("w/.git/branches/origin");
    write(&branches_file, &format!("{remote}#main\n"));
    fixture.expect_planned("ask", "w", PUSH_NEW, "another ref");
    let worktree = fixture.at("wt");
    fixture.git(
        "w",
        &["worktree", "add", "-q", "-b", "feature/wt", &worktree],
    );
    fixture.expect_planned("ask", "wt", PUSH_NEW, "another ref");
    std::fs::remove_file(&branches_file).expect("remove branches/origin");

    fixture.expect_planned("none", "wt", PUSH_NEW, "");
    fixture.expect_planned("none", "w", PUSH_NEW, "");
}

#[test]
fn reads_plans_only_from_the_file_the_environment_names() {
    let fixture = Fixture::new("plans-source");
    write(
        &fixture.path("w/.git/approved-plans"),
        "feature/new KEY-1 12:00\n",
    );
    fixture.expect("ask", "w", PUSH_NEW, "feature/new");
    let mut gate = fixture.gate();
    gate.env("AGENT_APPROVED_PLANS", fixture.at("missing"));
    expect_decision(&mut gate, "ask", &fixture.at("w"), PUSH_NEW, "feature/new");
}

// A relative path would resolve in the call's cwd, which the command chooses.
#[test]
fn reads_no_plans_from_a_relative_path() {
    let fixture = Fixture::new("plans-relative");
    let line = format!("feature/new {}\n", fixture.at("remote.git"));
    write(&fixture.path("w/plans"), &line);
    let mut gate = fixture.gate();
    gate.env("AGENT_APPROVED_PLANS", "plans")
        .current_dir(fixture.path("w"));
    expect_decision(&mut gate, "ask", &fixture.at("w"), PUSH_NEW, "feature/new");
}

#[test]
fn asks_for_a_payload_that_names_a_candidate_and_does_not_parse() {
    for payload in [
        r#"{"tool_input":{"command":"gh pr merge 5""#,
        r#"{"tool_input":{"command":123,"x":"gh pr merge 5"}}"#,
        r#"{"tool_input":"git push origin main"}"#,
    ] {
        let out = run(&mut hook("push-gate"), payload);
        assert_eq!(decision(&out), "ask", "payload: {payload:?}");
    }
}

#[test]
fn defers_for_a_payload_that_does_not_parse_and_names_no_candidate() {
    let out = run(&mut hook("push-gate"), "not json");
    assert_eq!(decision(&out), "none");
}

#[test]
fn starts_no_process_for_a_call_naming_no_push() {
    let fixture = Fixture::new("spy");
    let log = fixture.path("spy.log");
    let script = format!("#!/bin/sh\necho git >>\"{}\"\nexit 127\n", log.display());
    fixture.stub("spy", "git", &script);
    for command in ["git status", r#"git commit -m "fix the gate""#] {
        // A fixed cwd: the temp path could spell a word the early test looks for.
        expect_decision(
            &mut fixture.gate_with_stubs("spy"),
            "none",
            "/w",
            command,
            "",
        );
        assert!(!log.exists(), "command: {command:?} started a process");
    }
}

#[test]
fn defers_when_only_the_cwd_names_push() {
    let fixture = Fixture::new("cwd-name");
    std::fs::create_dir(fixture.path("push")).expect("create the push directory");
    fixture.expect("none", "push", "git status", "");
}

// A probe that outlived the hook's timeout would block the push, so a slow origin must ask
// inside it. An order, not a time, which load would stretch.
#[test]
fn asks_before_a_slow_origin_answers() {
    let fixture = Fixture::new("slow");
    fixture.repo("slow", "git@example.invalid:slow.git");
    let ended = fixture.at("ssh-ended");
    let mut gate = fixture.gate();
    gate.env("GIT_SSH_COMMAND", format!("sleep 30; touch '{ended}' #"));
    expect_decision(&mut gate, "ask", &fixture.at("slow"), PUSH_X, "feature/x");
    assert!(
        !Path::new(&ended).exists(),
        "the hook waited for the ssh to end"
    );
}

// The probe runs outside the sandbox and the agent sets origin's push URL, so a probe of any
// URL would carry what the URL holds to any host.
#[test]
fn probes_no_url_the_settings_do_not_list() {
    let fixture = Fixture::new("unlisted");
    fixture.repo("un", "git@example.invalid:un.git");
    let probed = fixture.at("probed");
    for remotes in [
        "",
        "git@example.invalid",
        "git@example.invalid:other.git",
        "https://example.invalid/",
    ] {
        let mut gate = hook("push-gate");
        gate.env("PUSH_GATE_REMOTES", remotes)
            .env("GIT_SSH_COMMAND", format!("touch '{probed}'; exit 1 #"));
        expect_decision(&mut gate, "ask", &fixture.at("un"), PUSH_X, "feature/x");
        assert!(
            !Path::new(&probed).exists(),
            "a probe started with {remotes:?} listed"
        );
    }
}

// In a URL the text after `host:` can be `x@other-host`, which ssh and curl connect to.
#[test]
fn probes_no_url_that_only_shares_a_url_prefix_ending_in_a_colon() {
    let fixture = Fixture::new("colon-prefix");
    fixture.repo("colon", "ssh://git@example.invalid:x@other.invalid/un.git");
    let probed = fixture.at("probed");
    let mut gate = hook("push-gate");
    gate.env("PUSH_GATE_REMOTES", "ssh://git@example.invalid:")
        .env("GIT_SSH_COMMAND", format!("touch '{probed}'; exit 1 #"));
    expect_decision(&mut gate, "ask", &fixture.at("colon"), PUSH_X, "feature/x");
    assert!(!Path::new(&probed).exists(), "a probe started");
}

#[test]
fn probes_a_url_the_settings_list_whole() {
    let fixture = Fixture::new("listed-whole");
    let mut gate = hook("push-gate");
    gate.env("PUSH_GATE_REMOTES", fixture.at("remote.git"));
    expect_decision(&mut gate, "none", &fixture.at("w"), PUSH_X, "");
}

#[test]
fn probes_origin_while_budget_is_left() {
    let fixture = Fixture::new("budget-left");
    fixture.repo("left", "git@example.invalid:left.git");
    let probed = fixture.at("probed");
    let mut gate = fixture.gate();
    gate.env("SECONDS", "5")
        .env("GIT_SSH_COMMAND", format!("touch '{probed}'; exit 1 #"));
    expect_decision(&mut gate, "ask", &fixture.at("left"), PUSH_X, "feature/x");
    assert!(Path::new(&probed).exists(), "no probe started");
}

// 7 s inherited plus the hook's own time leaves under the 1 s a probe needs.
#[test]
fn starts_no_probe_with_under_a_second_of_budget_left() {
    let fixture = Fixture::new("budget-edge");
    fixture.repo("edge", "git@example.invalid:edge.git");
    let probed = fixture.at("probed");
    let mut gate = fixture.gate();
    gate.env("SECONDS", "7")
        .env("GIT_SSH_COMMAND", format!("touch '{probed}'; exit 1 #"));
    expect_decision(&mut gate, "ask", &fixture.at("edge"), PUSH_X, "feature/x");
    assert!(!Path::new(&probed).exists(), "a probe started");
}

// With 6 s inherited the probe has 2 s. The 6 s bound sits between that and the whole budget.
#[test]
fn an_aged_hook_cuts_the_probe_at_what_is_left_of_the_budget() {
    let fixture = Fixture::new("budget-cut");
    fixture.repo("cut", "git@example.invalid:cut.git");
    let mut gate = fixture.gate();
    gate.env("SECONDS", "6")
        .env("GIT_SSH_COMMAND", "sleep 30 #");
    let start = std::time::Instant::now();
    expect_decision(&mut gate, "ask", &fixture.at("cut"), PUSH_X, "feature/x");
    let took = start.elapsed();
    assert!(took.as_secs() < 6, "the probe ran {took:?}");
}

#[test]
fn asks_without_probing_once_the_budget_is_spent() {
    let fixture = Fixture::new("aged");
    let mut gate = fixture.gate();
    gate.env("SECONDS", "8");
    expect_decision(&mut gate, "ask", &fixture.at("w"), PUSH_X, "feature/x");
}

// An ssh prompt on the controlling terminal could read the user's keys.
#[test]
fn probes_origin_without_a_controlling_terminal() {
    let fixture = Fixture::new("tty");
    fixture.repo("tty", "git@example.invalid:tty.git");
    let under_script = |shell: &str| {
        let mut script = Command::new("script");
        script
            .args(["-q", "/dev/null", "sh", "-c", shell])
            .env_clear()
            .env("PATH", std::env::var("PATH").unwrap_or_default())
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        script
    };
    let run_had_tty = fixture.at("run-had-tty");
    let _ = under_script(&format!("exec 3</dev/tty && touch '{run_had_tty}'")).status();
    if !Path::new(&run_had_tty).exists() {
        eprintln!("skip the terminal check: no terminal from script");
        return;
    }

    let (payload, answer) = (fixture.path("tty.json"), fixture.path("tty.out"));
    write(&payload, &bash_payload(&fixture.at("tty"), PUSH_X));
    let hook_had_tty = fixture.at("hook-had-tty");
    let ssh = format!("sh -c 'exec 3</dev/tty && touch {hook_had_tty}' #");
    let call = format!(
        "'{}' push-gate <'{}' >'{}'",
        env!("CARGO_BIN_EXE_claude-hooks"),
        payload.display(),
        answer.display()
    );
    let _ = under_script(&call)
        .env("GIT_SSH_COMMAND", ssh)
        .env("PUSH_GATE_REMOTES", "git@example.invalid:")
        .status();
    let answer = std::fs::read_to_string(&answer).unwrap_or_default();
    assert!(answer.contains(r#""permissionDecision":"ask""#), "{answer}");
    assert!(
        !Path::new(&hook_had_tty).exists(),
        "ls-remote reached the controlling terminal"
    );
}

#[test]
fn asks_for_any_other_shape_even_on_origin() {
    let fixture = Fixture::new("other-shapes");
    let w = fixture.at("w");
    fixture.assert_asks(&[
        "git push",
        "git push origin",
        "git push origin refs/tags/v1",
        "git push origin feature/x && gh pr edit 1 --body-file b.md",
        &format!("cd {w} && git push origin feature/x"),
        "git ls-remote origin feature/x; git push origin feature/x",
        "git push upstream feature/x",
        "git -c core.hooksPath=/dev/null push origin feature/x",
        "git push\norigin feature/x",
        "git push origin feature/x\necho done",
    ]);
}

// Each of these would run unprompted under the settings' allow rules if the gate missed it.
#[test]
fn asks_for_a_push_behind_options_quotes_or_aliases() {
    let fixture = Fixture::new("hidden");
    let w = fixture.at("w");
    symlink(&w, fixture.path("w{,}")).expect("link w{,}");
    fixture.assert_asks(&[
        &format!("git -C {w} \\\npush origin feature/new"),
        &format!("(cd {w} && git push)"),
        "git push; gh pr create",
        &format!("git -C {w} --work-tree {w} push origin feature/new"),
        &format!("git -C {w} --git-dir={w}/.git push origin feature/new"),
        r#"git -C "my dir" push origin feature/x"#,
        "git -c 'user.name=a b' push origin feature/x",
        r#"git -C "$ROOT"/.worktrees/x push -u origin feature/new"#,
        r#"git -c core.sshCommand="ssh -i k" push origin feature/x"#,
        r#"git -C x --git-dir="/a b/.git" push origin feature/new"#,
        r"git -C my\ dir push origin feature/x",
        r#"git -C . "push" origin feature/x"#,
        "git -C . -c alias.p=push p origin feature/x",
        "git -C . subtree push --prefix=d origin feature/x",
        "git -C . send-pack origin feature/x",
        &format!("git -C {w}{{,}} push origin feature/x"),
    ]);
}

// Literal `$HOME` and `~` directories exist here, so only the -C path check can ask.
#[test]
fn asks_for_a_dash_c_path_the_shell_would_expand() {
    let fixture = Fixture::new("literal-dirs");
    std::fs::create_dir(fixture.path("lit")).expect("create lit");
    symlink(fixture.path("w"), fixture.path("lit/$HOME")).expect("link $HOME");
    symlink(fixture.tmp.path(), fixture.path("lit/~")).expect("link ~");
    fixture.expect("ask", "lit", "git -C $HOME push origin feature/x", "");
    fixture.expect("ask", "lit", "git -C ~/w push origin feature/x", "");
}

// A false match costs only a prompt.
#[test]
fn asks_for_a_commit_message_naming_push() {
    Fixture::new("false-match").assert_asks(&[r#"git commit -m "push the fix""#]);
}

// The hook runs outside the sandbox, and a worker can point its worktree's .git file at a
// repo whose config it wrote, so the probe must not run that config.
#[test]
fn ignores_the_ssh_command_of_the_repo_config() {
    let fixture = Fixture::new("repo-ssh");
    fixture.repo("ev", "git@example.invalid:ev.git");
    let pwned = fixture.at("pwned-ssh");
    fixture.git(
        "ev",
        &["config", "core.sshCommand", &format!("touch {pwned} #")],
    );
    // The stub keeps the probe off the network.
    fixture.stub("offline", "ssh", "#!/bin/sh\nexit 255\n");
    let mut gate = fixture.gate_with_stubs("offline");
    expect_decision(&mut gate, "ask", &fixture.at("ev"), PUSH_X, "feature/x");
    assert!(!Path::new(&pwned).exists(), "ls-remote ran core.sshCommand");
}

#[test]
fn reads_no_option_from_the_url_of_origin() {
    let fixture = Fixture::new("url-option");
    fixture.repo("opt", "x");
    let pwned = fixture.at("pwned-opt");
    let remote = fixture.at("remote.git");
    let url = format!("--upload-pack=touch {pwned};git-upload-pack {remote}");
    fixture.git("opt", &["config", "remote.origin.url", &url]);
    fixture.expect("ask", "opt", PUSH_X, "feature/x");
    assert!(
        !Path::new(&pwned).exists(),
        "ls-remote read the URL as an option"
    );
}

// A <transport>::<address> URL, or a scheme with no built-in transport, runs
// git-remote-<transport> from PATH, ssh:: included.
#[test]
fn runs_no_remote_helper_the_url_names() {
    let fixture = Fixture::new("helper");
    let remote = fixture.at("remote.git");
    let pwned = fixture.at("pwned-helper");
    for transport in ["pwn", "ssh"] {
        let script = format!("#!/bin/sh\ntouch \"{pwned}\"\n");
        fixture.stub("helper", &format!("git-remote-{transport}"), &script);
    }
    fixture.repo("hp", "x");
    for url in [
        format!("pwn::{remote}"),
        "pwn://x".to_owned(),
        format!("ssh::{remote}"),
    ] {
        fixture.git("hp", &["remote", "set-url", "origin", &url]);
        let mut gate = fixture.gate_with_stubs("helper");
        expect_decision(&mut gate, "ask", &fixture.at("hp"), PUSH_X, "feature/x");
        assert!(
            !Path::new(&pwned).exists(),
            "ls-remote ran the helper of {url}"
        );
    }
}
