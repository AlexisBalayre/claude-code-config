use crate::common::Config;
use regex_lite::Regex;
use std::process::{exit, Command, Stdio};

const SKIPPED: &str = r"(^|/)migrations/|(^|/)[^/]*(\.lock|-lock\.(json|yaml))$|(^|/)(tests?|__tests__|__mocks__|__snapshots__)/|(^|/)([^/]*-)?fixtures?/|\.(test|spec|mock)\.[a-z]+$|_test\.[a-z]+$|(^|/)test_[^/]*$";

/// Run git in the current directory with its stderr shown; Err holds its exit code.
pub fn git_loud(args: &[&str]) -> Result<String, i32> {
    let out = Command::new("git")
        .args(args)
        .stderr(Stdio::inherit())
        .output()
        .map_err(|_| 127)?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout)
            .trim_end_matches('\n')
            .to_owned())
    } else {
        Err(out.status.code().unwrap_or(1))
    }
}

pub struct Count {
    pub level: &'static str,
    pub added: u64,
    pub fail_at: u64,
    /// `added<TAB>path` for each production file changed.
    pub rows: Vec<String>,
}

// A rename prints as `old => new` or `dir/{old => new}/rest`: the count goes to the new
// path. --no-renames would count a moved production file in full.
fn new_path(path: &str, braces: &Regex) -> String {
    let after_arrow = |text: &str| {
        text.rsplit_once(" => ")
            .map_or(text, |(_, new)| new)
            .to_owned()
    };
    let Some(found) = braces.find(path) else {
        return after_arrow(path);
    };
    let mid = after_arrow(&path[found.start() + 1..found.end() - 1]);
    let mut joined = format!("{}{mid}{}", &path[..found.start()], &path[found.end()..]);
    while joined.contains("//") {
        joined = joined.replace("//", "/");
    }
    joined
        .strip_prefix('/')
        .map(str::to_owned)
        .unwrap_or(joined)
}

pub fn count(base: &str, head: &str) -> Result<Count, i32> {
    let generated =
        Config::load(git_loud(&["rev-parse", "--show-toplevel"])?).get("GENERATED_PATHS_REGEX");
    let skipped = if generated.is_empty() {
        SKIPPED.to_owned()
    } else {
        format!("{SKIPPED}|{generated}")
    };
    let skipped = Regex::new(&skipped).map_err(|_| 2)?;
    let braces = Regex::new(r"\{[^}]* => [^}]*\}").map_err(|_| 2)?;

    let diff = ["-c", "core.quotePath=false", "diff"];
    let numstat = git_loud(&[&diff[..], &["--numstat", base, head]].concat())?;
    let mut added = 0;
    let mut rows = Vec::new();
    for line in numstat.lines() {
        let mut fields = line.split('\t');
        let lines = fields.next().unwrap_or("");
        let path = new_path(fields.nth(1).unwrap_or(""), &braces);
        if lines != "-" && !skipped.is_match(&path) {
            added += lines.parse::<u64>().unwrap_or(0);
            rows.push(format!("{lines}\t{path}"));
        }
    }

    let names = git_loud(&[&diff[..], &["--name-only", base, head]].concat())?;
    // 1,001, not 1,000: the cap fails from this number, and 1,000 passes.
    let fail_at = if names.lines().all(|name| name.ends_with(".md")) {
        1001
    } else {
        800
    };
    let level = match added {
        _ if added >= fail_at => "fail",
        400.. => "warn",
        _ => "pass",
    };
    Ok(Count {
        level,
        added,
        fail_at,
        rows,
    })
}

pub fn run(args: &[String]) {
    let [base, head, ..] = args else {
        eprintln!("Usage: added-lines <base> <head>");
        exit(1)
    };
    let count = count(base, head).unwrap_or_else(|code| exit(code));
    println!("{} {} {}", count.level, count.added, count.fail_at);
    for row in count.rows {
        println!("{row}");
    }
}
