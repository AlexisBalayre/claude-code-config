#![deny(unsafe_code)]
mod added_lines;
mod check_agent_config;
mod check_dashes;
mod comment_pruner;
mod common;
mod convention_spot_check;
mod frozen_paths;
mod gate_lock;
mod git_safety;
mod grant;
mod post_compact_state;
mod pre_commit;
mod pre_push;
mod protect_generated;
mod push_gate;
mod quality_checks;
mod session;
mod tool_result_size;
mod validate_file_naming;
mod verify;
mod worker_permission;
mod worktree_create;
mod worktree_remove;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let name = args.first().map(String::as_str).unwrap_or("");
    let rest = args.get(1..).unwrap_or(&[]);
    match name {
        "added-lines" => added_lines::run(rest),
        "check-agent-config" => check_agent_config::run(rest),
        "check-dashes" => check_dashes::run(rest),
        "gate-lock" => gate_lock::run(rest),
        "verify" => verify::run(rest),
        "pre-commit" => pre_commit::run(rest),
        "pre-push" => pre_push::run(rest),
        "worktree-create" => worktree_create::run(rest),
        "worktree-remove" => worktree_remove::run(rest),
        "grant" => grant::run(rest),
        "comment-pruner" => comment_pruner::run(rest),
        "convention-spot-check" => convention_spot_check::run(),
        "post-compact-state" => post_compact_state::run(),
        "quality-checks" => quality_checks::run(),
        "frozen-paths" => frozen_paths::run(),
        "git-safety" => git_safety::run(),
        "protect-generated" => protect_generated::run(),
        "push-gate" => push_gate::run(),
        "tool-result-size" => tool_result_size::run(),
        "validate-file-naming" => validate_file_naming::run(),
        "worker-permission" => worker_permission::run(),
        // Exit 2 so a typo in the hook wiring blocks instead of failing open.
        _ => {
            eprintln!("claude-hooks: unknown command '{name}'");
            std::process::exit(2)
        }
    }
}
