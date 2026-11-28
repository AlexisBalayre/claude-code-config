use crate::added_lines::git_loud;
use std::process::exit;

pub fn run(args: &[String]) {
    let mut diff = vec!["diff", "-U0", "--no-color", "--no-ext-diff", "--no-prefix"];
    diff.extend(args.iter().map(String::as_str));
    let diff = git_loud(&diff).unwrap_or_else(|code| exit(code));

    // Only the `+++ ` that follows a `diff --git` line names a file: an added line
    // that starts with "++ " reads the same.
    let (mut file, mut header) = ("", false);
    let mut dashed = Vec::new();
    for line in diff.split('\n') {
        if line.starts_with("diff --git ") {
            header = true;
        } else if header && line.starts_with("+++ ") {
            file = &line[4..];
            header = false;
        } else if line.starts_with('+') && line.contains(['\u{2014}', '\u{2013}']) {
            dashed.push(format!("  {file}: {}", &line[1..]));
        }
    }
    if dashed.is_empty() {
        return;
    }
    eprintln!("ERROR: Added lines contain an em or en dash (U+2014, U+2013).");
    eprintln!("Use a comma, colon, parentheses or \"to\". A JS, TS or JSON string literal that needs the character can write \\u2014.");
    eprintln!("{}", dashed.join("\n"));
    exit(1)
}
