mod diff;

use diff::{yaml_diff, YamlDiff};
use wasm_bindgen::prelude::*;

fn read_yaml(data: &str) -> Result<serde_yml::Value, serde_yml::Error> {
    let parsed_data: serde_yml::Value = serde_yml::from_str(data)?;
    Ok(parsed_data)
}

fn format_parse_error(label: &str, e: &serde_yml::Error) -> String {
    match e.location() {
        Some(loc) => format!("[{label}] Error: {e} at line: {}", loc.line()),
        None => format!("[{label}] Error: {e}"),
    }
}

#[wasm_bindgen]
pub fn diff(yone: &str, ytwo: &str) -> Result<Vec<YamlDiff>, JsError> {
    let yone_trimmed = yone.trim();
    let ytwo_trimmed = ytwo.trim();

    if yone_trimmed.is_empty() && ytwo_trimmed.is_empty() {
        return Err(JsError::new(
            "[YAML ONE] Error: empty input\n[YAML TWO] Error: empty input",
        ));
    }
    if yone_trimmed.is_empty() {
        return Err(JsError::new("[YAML ONE] Error: empty input"));
    }
    if ytwo_trimmed.is_empty() {
        return Err(JsError::new("[YAML TWO] Error: empty input"));
    }

    let parsed_one = read_yaml(yone);
    let parsed_two = read_yaml(ytwo);

    match (parsed_one, parsed_two) {
        (Ok(one), Ok(two)) => {
            yaml_diff(&one, &two).map_err(|e| JsError::new(&format!("Diff error: {e:?}")))
        }
        (Err(e1), Err(e2)) => {
            let msg = format!(
                "{}\n{}",
                format_parse_error("YAML ONE", &e1),
                format_parse_error("YAML TWO", &e2)
            );
            Err(JsError::new(&msg))
        }
        (Err(e), Ok(_)) => Err(JsError::new(&format_parse_error("YAML ONE", &e))),
        (Ok(_), Err(e)) => Err(JsError::new(&format_parse_error("YAML TWO", &e))),
    }
}
