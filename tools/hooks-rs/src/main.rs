#![deny(unsafe_code)]
mod common;
mod frozen_paths;
mod git_safety;
mod protect_generated;
mod tool_result_size;
mod validate_file_naming;

fn main() {
    let name = std::env::args().nth(1).unwrap_or_default();
    match name.as_str() {
        "frozen-paths" => frozen_paths::run(),
        "git-safety" => git_safety::run(),
        "protect-generated" => protect_generated::run(),
        "tool-result-size" => tool_result_size::run(),
        "validate-file-naming" => validate_file_naming::run(),
        // Exit 2 so a typo in the hook wiring blocks instead of failing open.
        _ => {
            eprintln!("claude-hooks: unknown command '{name}'");
            std::process::exit(2)
        }
    }
}
