mod common;

use common::{git, git_stdout, run, verdict, write, Lead, Verdict, WORKER};
use serde_json::json;
use std::path::{Path, PathBuf};

const URL: &str = "git@github.com:o/repo.git";
const OTHER_URL: &str = "git@github.com:o/other.git";
const PUSH: &str = "git push origin feature/issue-12";

/// Hold a checkout of `feature/issue-12` whose origin pushes to URL, and an approved-plans file
/// that lists the branch for it.
struct Plan {
    lead: Lead,
    repo: PathBuf,
    plans: PathBuf,
}

impl Plan {
    fn new(test: &str) -> Plan {
        let lead = Lead::new(test);
        let repo = lead.repo("plan", "feature/issue-12");
        git(&repo, &["remote", "add", "origin", URL]);
        let plans = lead.tmp.path().join("plans");
        let lines = [
            format!("feature/issue-12 {URL} issue-12 13:40"),
            format!("feature/issue-5 {URL} issue-5 13:40"),
            format!("feature/issue-4 {OTHER_URL} issue-4 13:40"),
            format!("feature/other {URL} issue-6 13:40"),
            format!("dev {URL} issue-7 13:40"),
            format!("main {URL} issue-7 13:40"),
        ];
        write(&plans, &format!("{}\n", lines.join("\n")));
        Plan { lead, repo, plans }
    }

    /// Ask the hook about a Bash call made from `cwd`, with the plans file named.
    fn ask_from(&self, cwd: &Path, command: &str) -> Verdict {
        let payload = json!({"cwd": cwd, "tool_name": "Bash", "tool_input": {"command": command}});
        let mut hook = self.lead.hook(WORKER);
        hook.env("AGENT_APPROVED_PLANS", &self.plans);
        verdict(&run(&mut hook, &payload.to_string()))
    }

    fn ask(&self, command: &str) -> Verdict {
        self.ask_from(&self.repo, command)
    }

    fn config(&self, key: &str, value: &str) {
        git(&self.repo, &["config", key, value]);
    }

    /// Assert the listed branch's push is refused with a request a grant can follow.
    fn assert_grant_route(&self, why: &str) {
        let verdict = self.ask(PUSH);
        assert!(!verdict.id().is_empty(), "{why}: {}", verdict.message);
    }
}

#[test]
fn a_push_that_is_not_the_whole_call_has_no_grant_route() {
    let lead = Lead::new("wpp-unbound");
    let repo = lead.repo("repo", "feature/issue-12");
    lead.assert_no_grant_route(
        &repo,
        &[
            "git add -A && git commit -m x && git push origin feature/issue-12",
            "git status\ngit push",
            "cd /other && git push",
            "\"git\" push; true",
            "{ git push origin feature/issue-12; }",
            "then git push origin feature/issue-12",
            "! git push origin feature/issue-12",
            "echo \\\"; git push origin other-branch; echo \\\"",
            "cat x | xargs git push",
            "exec git push origin feature/issue-12",
            "xargs git push origin feature/issue-12",
            "/usr/bin/git push origin feature/issue-12",
            "GIT_TERMINAL_PROMPT=0 git push origin feature/issue-12",
            "git -C /other push origin feature/issue-12",
            "git -P push origin feature/issue-12",
            "git --git-dir .git push origin feature/issue-12",
            "git push",
            "git push origin",
            "git push origin a b",
            "git push origin other-branch",
            "git push origin HEAD:feature/issue-12",
            "git push origin \"feature/issue-12:other\"",
            "git push origin +feature/issue-12",
            "git push origin feature/*",
            "git push origin --output=x",
            "git push --all",
            "git push --tags origin",
            "git push --force feature/issue-12",
            "git push --force origin feature/issue-12",
            "git push -u -f origin feature/issue-12",
            "git push -- origin feature/issue-12",
            "git push origin feature/issue-12 2 >/dev/null",
            "git push origin feature/issue-12 \"2\">/dev/null",
            "git push origin feature/issue-12 2>&1 1",
            "git push origin feature/issue-12\u{a0}",
            "git send-pack origin",
            "git \"push\" origin",
            "git $'push' origin feature/issue-12",
            "git \"$(echo push)\" origin feature/issue-12",
        ],
        "whole call",
    );
}

#[test]
fn a_push_through_an_alias_a_variable_or_a_git_runner_has_no_grant_route() {
    let lead = Lead::new("wpp-unbound-indirect");
    let repo = lead.repo("repo", "feature/issue-12");
    lead.assert_no_grant_route(
        &repo,
        &[
            "git -c alias.p=push p",
            "git -c Alias.p=push p",
            "git --config-env=alias.p=V p",
            "GIT_CONFIG_KEY_0=alias.p git p",
            "GIT_CONFIG_KEY_0=Alias.p git p",
            "p=push; git $p origin feature/issue-12",
            "G=git; $G push origin feature/issue-12",
            "$TOOLS/git -c alias.p=push p",
            "git rebase -x 'git push origin feature/issue-12' main",
            "git submodule foreach 'git push'",
            "git subtree push --prefix=x origin main",
        ],
        "whole call",
    );
}

#[test]
fn a_call_whose_words_show_no_push_takes_the_grant_route_bound_to_head() {
    let lead = Lead::new("wpp-hidden");
    let repo = lead.repo("repo", "feature/issue-12");
    let head = git_stdout(&repo, &["rev-parse", "HEAD"]);
    for command in [
        "git commit -m \"fix: bind a push grant to HEAD\"",
        "git stash push -m wip",
        "git log --grep=push",
        "git log --grep=push $(git merge-base HEAD origin/main)..HEAD",
        "gh pr create --body 'git push'",
        "env FOO=1 gh pr create --body 'run git push after'",
        "env -i PATH=/usr/bin gh pr create --body 'run git push after'",
        "git diff --name-only | xargs grep -n \"git push\"",
        "awk '/git push/ {n++}' x.log",
        "grep -rn GIT_CONFIG_GLOBAL scripts/",
        "GIT_CONFIG_GLOBAL=/dev/null git -C /tmp/fx commit -q --allow-empty -m one",
        "git commit -F - <<'EOF'\nmention git push here\nEOF",
        "git commit -F - <<'EOF'\nrun `git push` later\nEOF",
        "git commit -m \"$(cat <<'EOF'\nfix: git push grant\nEOF\n)\"",
        "bash -c 'echo push'",
        "bash -c 'git push origin feature/issue-12'",
        "sh -c 'git push origin feature/issue-12'",
        "eval 'git push origin feature/issue-12'",
        "env -S 'git push origin feature/issue-12'",
        "env -u A -u B -S 'git push origin feature/issue-12'",
        "sudo -s 'git push origin feature/issue-12'",
        "python3 -c 'import os; os.system(\"git push origin feature/issue-12\")'",
        "python3.12 -c 'import os; os.system(\"git push\")'",
        "perl -e 'system(q(git push))'",
        "php -r 'system(\"git push\");'",
        "echo 'git push origin feature/issue-12' | sh",
        "nohup git push origin feature/issue-12 > /tmp/push.log 2>&1",
        "time git push origin feature/issue-12 > /tmp/log",
        "git push origin feature/issue-12 > log",
        "g\"it\" push origin feature/issue-12 > log",
        "\\git push origin feature/issue-12 > log",
        "(git push origin feature/issue-12)",
        "$(which git) push origin feature/issue-12",
        "git $(echo push) origin feature/issue-12",
        "git `echo push` origin feature/issue-12",
        "git push origin $(git branch --show-current)",
        "FOO=1 $G push origin feature/issue-12",
        "x=$(printf 'gi%s' t); $x push origin feature/issue-12",
        "HOME=/tmp/h git p",
        "git -c include.path=/tmp/c p",
        "GIT_CONFIG_GLOBAL=/tmp/c git p",
        "if :; then git push origin feature/issue-12; fi #'",
    ] {
        let verdict = lead.bash(&repo, command);
        assert!(
            !verdict.id().is_empty(),
            "command: {command:?}: {}",
            verdict.message
        );
        assert_eq!(
            lead.request(verdict.id())["head"],
            head.as_str(),
            "command: {command:?}"
        );
    }
}

#[test]
fn a_lone_push_with_no_approved_plan_takes_the_grant_route_bound_to_head() {
    let lead = Lead::new("wpp-lone");
    let repo = lead.repo("repo", "feature/issue-12");
    let head = git_stdout(&repo, &["rev-parse", "HEAD"]);
    for command in [
        PUSH,
        "git push -u origin feature/issue-12",
        "git push --set-upstream origin feature/issue-12",
        "git push origin feature/issue-12 2>&1",
        "git push origin feature/issue-12 2>/dev/null",
        "git push origin feature/issue-12 \n",
    ] {
        let verdict = lead.bash(&repo, command);
        assert!(
            !verdict.id().is_empty(),
            "command: {command:?}: {}",
            verdict.message
        );
        assert_eq!(
            lead.request(verdict.id())["head"],
            head.as_str(),
            "command: {command:?}"
        );
    }
}

#[test]
fn a_granted_lone_push_is_allowed_at_the_head_the_lead_saw() {
    let lead = Lead::new("wpp-lone-granted");
    let repo = lead.repo("repo", "feature/issue-12");
    let id = lead.bash(&repo, PUSH).id().to_owned();
    lead.grant("allow", &id);
    assert_eq!(lead.bash(&repo, PUSH).behavior, "allow");
}

#[test]
fn a_granted_lone_push_is_a_new_request_once_head_moves() {
    let lead = Lead::new("wpp-lone-moved");
    let repo = lead.repo("repo", "feature/issue-12");
    let id = lead.bash(&repo, PUSH).id().to_owned();
    lead.grant("allow", &id);
    git(&repo, &["commit", "-q", "--allow-empty", "-m", "two"]);
    let retry = lead.bash(&repo, PUSH);
    let head = git_stdout(&repo, &["rev-parse", "HEAD"]);
    assert_eq!(retry.behavior, "deny");
    assert_eq!(lead.request(retry.id())["head"], head.as_str());
}

#[test]
fn a_push_from_a_directory_with_no_readable_head_has_no_grant_route() {
    let lead = Lead::new("wpp-no-head");
    lead.assert_no_grant_route(Path::new("/nonexistent"), &[PUSH], "could not read HEAD");
}

#[test]
fn a_push_from_a_detached_head_has_no_grant_route() {
    let plan = Plan::new("wpp-detached");
    git(&plan.repo, &["checkout", "-q", "--detach"]);
    let verdict = plan.ask(PUSH);
    assert_eq!(verdict.behavior, "deny");
    assert!(
        verdict.message.contains("could not read HEAD"),
        "{}",
        verdict.message
    );
    assert_eq!(plan.lead.requests(), 0);
}

#[test]
fn an_approved_plans_push_is_allowed() {
    let plan = Plan::new("wpp-approved");
    for command in [
        PUSH,
        "git push -u origin feature/issue-12",
        "git push --set-upstream origin feature/issue-12",
        "git push origin feature/issue-12 2>&1",
    ] {
        assert_eq!(plan.ask(command).behavior, "allow", "command: {command:?}");
    }
}

#[test]
fn an_approved_plans_push_leaves_no_request() {
    let plan = Plan::new("wpp-approved-quiet");
    plan.ask(PUSH);
    assert_eq!(plan.lead.requests(), 0);
}

// worktree-remove prunes only the file the variable names, so a default path would keep the
// line of a deleted branch.
#[test]
fn with_no_plans_file_named_the_state_directorys_one_approves_nothing() {
    let plan = Plan::new("wpp-default-plans");
    std::fs::create_dir_all(plan.lead.state()).expect("create the state directory");
    std::fs::copy(&plan.plans, plan.lead.state().join("approved-plans")).expect("copy the plans");
    let verdict = plan.lead.bash(&plan.repo, PUSH);
    assert!(!verdict.id().is_empty(), "{}", verdict.message);
}

// A relative path would resolve in a directory the worker chooses.
#[test]
fn a_plans_file_named_by_a_relative_path_approves_nothing() {
    let plan = Plan::new("wpp-relative-plans");
    let payload = json!({"cwd": &plan.repo, "tool_name": "Bash", "tool_input": {"command": PUSH}});
    let mut hook = plan.lead.hook(WORKER);
    hook.env("AGENT_APPROVED_PLANS", "plans")
        .current_dir(plan.lead.tmp.path());
    let verdict = verdict(&run(&mut hook, &payload.to_string()));
    assert!(!verdict.id().is_empty(), "{}", verdict.message);
}

#[test]
fn a_branch_listed_for_two_repositories_is_approved_for_each() {
    let plan = Plan::new("wpp-two-repositories");
    write(
        &plan.plans,
        &format!(
            "feature/issue-12 {OTHER_URL} issue-12 13:40\nfeature/issue-12 {URL} issue-12 13:45\n"
        ),
    );
    assert_eq!(plan.ask(PUSH).behavior, "allow");
}

#[test]
fn an_approved_push_that_leaves_the_sandbox_takes_the_grant_route() {
    let plan = Plan::new("wpp-unsandboxed");
    for flag in [json!(true), json!("true")] {
        let payload = json!({
            "cwd": &plan.repo,
            "tool_name": "Bash",
            "tool_input": {"command": PUSH, "dangerouslyDisableSandbox": flag},
        });
        let mut hook = plan.lead.hook(WORKER);
        hook.env("AGENT_APPROVED_PLANS", &plan.plans);
        let verdict = verdict(&run(&mut hook, &payload.to_string()));
        assert!(
            !verdict.id().is_empty(),
            "flag: {flag}: {}",
            verdict.message
        );
    }
}

#[test]
fn with_no_plans_file_a_push_takes_the_grant_route() {
    let plan = Plan::new("wpp-no-plans");
    std::fs::remove_file(&plan.plans).expect("remove the plans file");
    plan.assert_grant_route("no approved-plans file");
}

#[test]
fn a_directory_whose_path_json_escapes_still_gets_its_approved_push() {
    let plan = Plan::new("wpp-escaped-path");
    let clone = plan.lead.tmp.path().join("plan \u{e9}\\q");
    let source = plan.repo.to_str().expect("a UTF-8 path");
    git(
        plan.lead.tmp.path(),
        &["clone", "-q", source, "plan \u{e9}\\q"],
    );
    git(&clone, &["remote", "set-url", "origin", URL]);
    assert_eq!(plan.ask_from(&clone, PUSH).behavior, "allow");
}

#[test]
fn a_listed_branch_that_is_not_checked_out_has_no_grant_route() {
    let plan = Plan::new("wpp-not-checked-out");
    let verdict = plan.ask("git push origin feature/issue-5");
    assert_eq!(verdict.behavior, "deny");
    assert!(
        verdict.message.contains("not the checked-out branch"),
        "{}",
        verdict.message
    );
    assert_eq!(plan.lead.requests(), 0);
}

#[test]
fn a_listed_branch_in_an_unbindable_shape_has_no_grant_route() {
    let plan = Plan::new("wpp-listed-unbound");
    for command in [
        "git push --force origin feature/issue-12",
        "git push origin +feature/issue-12",
        "git push origin HEAD:feature/issue-12",
        "git add -A && git push origin feature/issue-12",
    ] {
        let verdict = plan.ask(command);
        assert_eq!(verdict.behavior, "deny", "command: {command:?}");
        assert!(
            verdict.message.contains("whole call"),
            "command: {command:?}"
        );
    }
    assert_eq!(plan.lead.requests(), 0);
}

#[test]
fn a_listed_branch_pushed_to_another_remote_takes_the_grant_route() {
    let plan = Plan::new("wpp-other-remote");
    git(&plan.repo, &["remote", "add", "upstream", URL]);
    let verdict = plan.ask("git push upstream feature/issue-12");
    assert!(!verdict.id().is_empty(), "{}", verdict.message);
}

#[test]
fn a_branch_the_plans_do_not_list_for_this_url_takes_the_grant_route() {
    let plan = Plan::new("wpp-unlisted");
    for branch in ["feature/issue-3", "feature/issue-4"] {
        git(&plan.repo, &["checkout", "-q", "-B", branch]);
        let verdict = plan.ask(&format!("git push origin {branch}"));
        assert!(
            !verdict.id().is_empty(),
            "branch: {branch}: {}",
            verdict.message
        );
    }
}

#[test]
fn a_listed_branch_of_another_issue_takes_the_grant_route() {
    let plan = Plan::new("wpp-other-issue");
    let branches = [
        "feature/issue-5",
        "feature/issue-120",
        "feature/issue-12-more",
    ];
    let lines = branches.map(|branch| format!("{branch} {URL} issue 13:40\n"));
    write(&plan.plans, &lines.concat());
    for branch in branches {
        git(&plan.repo, &["checkout", "-q", "-B", branch]);
        let verdict = plan.ask(&format!("git push origin {branch}"));
        assert!(
            !verdict.id().is_empty(),
            "branch: {branch}: {}",
            verdict.message
        );
    }
}

#[test]
fn a_listed_branch_outside_the_issue_pattern_takes_the_grant_route() {
    let plan = Plan::new("wpp-not-an-issue-branch");
    for branch in ["dev", "main", "feature/other"] {
        git(&plan.repo, &["checkout", "-q", "-B", branch]);
        let verdict = plan.ask(&format!("git push origin {branch}"));
        assert!(
            !verdict.id().is_empty(),
            "branch: {branch}: {}",
            verdict.message
        );
    }
}

#[test]
fn the_branch_prefix_comes_from_the_project_config() {
    let plan = Plan::new("wpp-prefix");
    git(&plan.repo, &["checkout", "-q", "-B", "work/issue-12"]);
    write(
        &plan.repo.join(".claude/project.env"),
        "WORKTREE_BRANCH_PREFIX=work\n",
    );
    write(
        &plan.plans,
        &format!("work/issue-12 {URL} issue-12 13:40\n"),
    );
    let payload = json!({
        "cwd": &plan.repo,
        "tool_name": "Bash",
        "tool_input": {"command": "git push origin work/issue-12"},
    });
    let mut hook = plan.lead.hook(WORKER);
    hook.env("AGENT_APPROVED_PLANS", &plan.plans)
        .env("CLAUDE_PROJECT_DIR", &plan.repo);
    assert_eq!(
        verdict(&run(&mut hook, &payload.to_string())).behavior,
        "allow"
    );
}

#[test]
fn an_origin_that_pushes_to_another_url_takes_the_grant_route() {
    let plan = Plan::new("wpp-pushurl");
    git(
        &plan.repo,
        &["remote", "set-url", "--push", "origin", OTHER_URL],
    );
    plan.assert_grant_route("remote.origin.pushurl");
}

#[test]
fn an_origin_with_a_second_push_url_takes_the_grant_route() {
    let plan = Plan::new("wpp-two-pushurls");
    git(&plan.repo, &["remote", "set-url", "--push", "origin", URL]);
    git(
        &plan.repo,
        &["remote", "set-url", "--add", "--push", "origin", OTHER_URL],
    );
    plan.assert_grant_route("two push URLs");
}

#[test]
fn a_push_url_rewritten_by_push_instead_of_takes_the_grant_route() {
    let plan = Plan::new("wpp-instead-of");
    plan.config(
        "url.git@github.com:o/evil-.pushInsteadOf",
        "git@github.com:o/",
    );
    plan.assert_grant_route("pushInsteadOf");
}

#[test]
fn a_listed_push_url_that_runs_a_transport_helper_takes_the_grant_route() {
    let plan = Plan::new("wpp-helper");
    let helper = format!("ext::{URL}");
    git(&plan.repo, &["remote", "set-url", "origin", &helper]);
    write(
        &plan.plans,
        &format!("feature/issue-12 {helper} issue-12 13:40\n"),
    );
    plan.assert_grant_route("a transport helper URL");
}

#[test]
fn a_remote_origin_push_mapping_takes_the_grant_route() {
    let plan = Plan::new("wpp-push-mapping");
    plan.config(
        "remote.origin.push",
        "refs/heads/feature/issue-12:refs/heads/dev",
    );
    plan.assert_grant_route("remote.origin.push");
}

#[test]
fn a_mirror_origin_takes_the_grant_route() {
    let plan = Plan::new("wpp-mirror");
    plan.config("remote.origin.mirror", "true");
    plan.assert_grant_route("remote.origin.mirror");
}

#[test]
fn push_default_upstream_or_tracking_takes_the_grant_route() {
    let plan = Plan::new("wpp-push-default");
    plan.config("branch.feature/issue-12.merge", "refs/heads/dev");
    for value in ["upstream", "tracking"] {
        plan.config("push.default", value);
        plan.assert_grant_route(value);
    }
}

#[test]
fn push_default_simple_keeps_the_approved_push() {
    let plan = Plan::new("wpp-push-default-simple");
    plan.config("branch.feature/issue-12.merge", "refs/heads/dev");
    plan.config("push.default", "simple");
    assert_eq!(plan.ask(PUSH).behavior, "allow");
}

#[test]
fn a_multi_line_config_value_cannot_pass_for_push_default() {
    let plan = Plan::new("wpp-push-default-forged");
    plan.config("push.default", "upstream");
    plan.config("test.x", "a\npush.default=simple");
    plan.assert_grant_route("a later value holding a push.default line");
}

#[test]
fn a_setting_that_sends_tags_or_submodule_commits_along_takes_the_grant_route() {
    let plan = Plan::new("wpp-sends-more");
    for (key, value) in [
        ("push.followTags", "true"),
        ("push.followTags", "yes"),
        ("push.followTags", "1"),
        ("push.recurseSubmodules", "on-demand"),
        ("push.recurseSubmodules", "only"),
        ("submodule.recurse", "true"),
    ] {
        plan.config(key, value);
        plan.assert_grant_route(&format!("{key} {value}"));
        git(&plan.repo, &["config", "--unset", key]);
    }
}

#[test]
fn a_follow_tags_key_with_no_value_takes_the_grant_route() {
    let plan = Plan::new("wpp-valueless");
    let config = plan.repo.join(".git/config");
    let text = std::fs::read_to_string(&config).expect("read the config");
    write(&config, &format!("{text}[push]\n\tfollowTags\n"));
    plan.assert_grant_route("a valueless push.followTags");
}

#[test]
fn those_settings_switched_off_keep_the_approved_push() {
    let plan = Plan::new("wpp-sends-no-more");
    for (key, value) in [
        ("push.followTags", "false"),
        ("push.followTags", "No"),
        ("push.followTags", "off"),
        ("push.followTags", "0"),
        ("push.recurseSubmodules", "no"),
        ("push.recurseSubmodules", "check"),
        ("submodule.recurse", "false"),
    ] {
        plan.config(key, value);
        assert_eq!(plan.ask(PUSH).behavior, "allow", "setting: {key} {value}");
        git(&plan.repo, &["config", "--unset", key]);
    }
}

#[test]
fn the_last_value_of_a_setting_decides_as_in_git() {
    let plan = Plan::new("wpp-last-value");
    plan.config("push.followTags", "true");
    git(&plan.repo, &["config", "--add", "push.followTags", "false"]);
    // The [submodule] section lands after [push], and git reads the two keys into one variable.
    plan.config("push.recurseSubmodules", "on-demand");
    plan.config("submodule.recurse", "false");
    assert_eq!(plan.ask(PUSH).behavior, "allow");
    git(&plan.repo, &["config", "--unset", "submodule.recurse"]);
    plan.assert_grant_route("push.recurseSubmodules alone");
}

#[test]
fn a_legacy_remotes_or_branches_file_takes_the_grant_route() {
    for kind in ["remotes", "branches"] {
        let plan = Plan::new(&format!("wpp-legacy-{kind}"));
        write(
            &plan.repo.join(".git").join(kind).join("origin"),
            &format!("URL: {URL}\nPush: refs/heads/feature/issue-12:refs/heads/dev\n"),
        );
        plan.assert_grant_route(kind);
    }
}
