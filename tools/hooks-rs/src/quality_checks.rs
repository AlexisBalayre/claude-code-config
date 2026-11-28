use crate::check_agent_config::{captured_output, spawn_captured};
use crate::common::{git, project_dir, Config};
use crate::session::{cwd, git_lines, source_regex};
use std::io::Write;
use std::path::Path;
use std::process::{exit, Command};

const COMMON_DIR: [&str; 3] = ["rev-parse", "--path-format=absolute", "--git-common-dir"];

/// Run `cmd` on the files as `xargs sh -c '<cmd> "$@"' _` does. Returns whether every run
/// succeeded, and their stdout and stderr in the order written.
pub fn xargs(cmd: &str, files: &[String]) -> (bool, Vec<u8>) {
    let mut ok = true;
    let mut output = Vec::new();
    let mut rest = files;
    while !rest.is_empty() {
        // The stricter of the two xargs defaults: 5000 arguments (BSD), 128 KiB (GNU).
        let mut bytes = 0;
        let fits = |file: &&String| {
            bytes += file.len() + 1;
            bytes <= 128 * 1024
        };
        let (batch, tail) = rest.split_at(rest.iter().take(5000).take_while(fits).count().max(1));
        rest = tail;
        let mut run = Command::new("sh");
        run.args(["-c", &format!("{cmd} \"$@\""), "_"]).args(batch);
        let Ok((mut child, file)) = spawn_captured(&mut run) else {
            return (false, output);
        };
        let status = child.wait();
        output.extend(captured_output(file));
        match status.ok().and_then(|status| status.code()) {
            Some(0) => {}
            Some(255) => {
                output.extend(b"xargs: sh: exited with status 255; aborting\n");
                return (false, output);
            }
            _ => ok = false,
        }
    }
    (ok, output)
}

pub fn run() {
    // The session's tree is its cwd when that is a worktree of this repository; any other
    // cwd falls back to the project dir. git before 2.31 prints --path-format back
    // instead of a path, hence the shape test.
    let cwd = cwd();
    let project = project_dir();
    let top = git(&project, &COMMON_DIR)
        .filter(|common| !cwd.is_empty() && common.starts_with('/') && !common.contains('\n'))
        .filter(|common| git(&cwd, &COMMON_DIR).as_ref() == Some(common))
        .and_then(|_| git(&cwd, &["rev-parse", "--show-toplevel"]))
        .filter(|top| !top.is_empty());
    if std::env::set_current_dir(top.unwrap_or(project)).is_err() {
        exit(1)
    }

    let config = Config::load(".");
    let (format, lint) = (config.get("FORMAT_FIX_CMD"), config.get("LINT_CMD"));
    if format.is_empty() && lint.is_empty() {
        return;
    }
    let Some(source) = source_regex(&config) else {
        return;
    };
    // Untracked files too: a file Write just created is not staged yet.
    let dirty: Vec<String> = git_lines(&[
        &["diff", "--name-only"],
        &["diff", "--cached", "--name-only"],
        &["ls-files", "--others", "--exclude-standard"],
    ])
    .into_iter()
    .filter(|file| source.is_match(file) && Path::new(file).is_file())
    .collect();
    if dirty.is_empty() {
        return;
    }

    if !format.is_empty() {
        xargs(&format, &dirty);
    }
    if lint.is_empty() {
        return;
    }
    let (ok, output) = xargs(&lint, &dirty);
    if !ok {
        let end = output
            .iter()
            .rposition(|byte| *byte != b'\n')
            .map_or(0, |last| last + 1);
        let lines: Vec<&[u8]> = output[..end].split(|byte| *byte == b'\n').collect();
        // The tail only: this output lands in the model's context on every failing turn.
        let mut report = lines[lines.len().saturating_sub(40)..].join(&b'\n');
        report.extend(
            format!("\nLint failed on the files this session touched ({lint}). Fix the remaining issues above.\n")
                .bytes(),
        );
        let _ = std::io::stderr().write_all(&report);
        exit(2)
    }
}
