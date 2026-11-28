use crate::common::{agent_work, project_dir};
use crate::worker_permission::{ascii, fresh};
use serde_json::Value;
use std::io::Write;
use std::process::{exit, Command};

const STALE: &str = "past the 30 minute window; when the worker retries the call, the hook records it again under the same id";

fn refuse(message: &str) -> ! {
    eprintln!("grant: {message}");
    exit(1)
}

pub fn run(args: &[String]) {
    let (verb, id) = match args {
        [verb, id]
            if matches!(verb.as_str(), "show" | "allow")
                && id.len() == 64
                && id.bytes().all(|c| matches!(c, b'0'..=b'9' | b'a'..=b'f')) =>
        {
            (verb.as_str(), id)
        }
        _ => {
            eprintln!("usage: claude-hooks grant show <id> | allow <id>; <id> is the 64-hex request id worker-permission printed");
            exit(2)
        }
    };
    let Some(work) = agent_work(project_dir()) else {
        eprintln!("grant: no state directory: AGENT_WORK is unset and this is no git checkout");
        exit(2)
    };
    let requests = work.join("requests");
    let request = requests.join(format!("{id}.json"));
    if !request.is_file() {
        refuse(&format!(
            "no request {id}: never recorded, already granted, or pruned {STALE}"
        ));
    }
    // A request past the window is one the user may have declined, or whose worker stopped. The
    // prune is the age check, so no request crosses the window between two reads of the clock.
    for entry in std::fs::read_dir(&requests).into_iter().flatten().flatten() {
        let file = entry.path();
        if file.extension().is_some_and(|kind| kind == "json") && !fresh(&file) {
            let _ = std::fs::remove_file(file);
        }
    }
    if !request.is_file() {
        refuse(&format!("request {id} is {STALE}"));
    }
    let record: Value = std::fs::read_to_string(&request)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default();
    let Some(worker) = record["worker"].as_str() else {
        refuse(&format!("request {id} is unreadable"))
    };
    // What the lead puts to the user is this record, never the worker's own account of the call.
    if verb == "show" {
        for line in format!("{record:#}").lines() {
            println!("{}", ascii(line));
        }
        return;
    }

    let date = Command::new("date")
        .arg("+%Y-%m-%d %H:%M:%S")
        .output()
        .map(|out| out.stdout)
        .unwrap_or_default();
    let line = format!(
        "{} {}\n",
        String::from_utf8_lossy(&date).trim_end(),
        ascii(&record.to_string())
    );
    let grants = work.join("grants");
    // The log is the user's audit, so it is written first: a grant that cannot be logged is
    // not given.
    let granted = std::fs::File::options()
        .append(true)
        .create(true)
        .open(work.join("grants.log"))
        .and_then(|mut log| log.write_all(line.as_bytes()))
        .and_then(|()| std::fs::create_dir_all(&grants))
        .and_then(|()| std::fs::write(grants.join(id), format!("{worker}\n")));
    if let Err(error) = granted {
        refuse(&format!("{}: {error}", work.display()));
    }
    let _ = std::fs::remove_file(request);
    println!("grant: granted {id} to {worker} for 30 minutes");
}
