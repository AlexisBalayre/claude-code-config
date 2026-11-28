use crate::common::{block, payload, project_dir, Config};
use regex_lite::Regex;

pub fn run() {
    let generated = Config::load(project_dir()).get("GENERATED_PATHS_REGEX");
    if generated.is_empty() {
        return;
    }
    let payload = payload().unwrap_or_default();
    let file = payload["tool_input"]["file_path"].as_str().unwrap_or("");
    if Regex::new(&generated).is_ok_and(|re| file.lines().any(|line| re.is_match(line))) {
        block(&format!(
            "{file} is generated. Edit its source and rerun the generator instead."
        ));
    }
}
