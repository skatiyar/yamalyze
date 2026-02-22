mod diff;

use diff::{yaml_diff, YamlDiff};
use wasm_bindgen::prelude::*;

// Reads and parses a YAML file into a BTreeMap
fn read_yaml(data: &str) -> Result<serde_yml::Value, serde_yml::Error> {
    let parsed_data: serde_yml::Value = serde_yml::from_str(data)?;
    Ok(parsed_data)
}

#[wasm_bindgen]
pub fn diff(yone: &str, ytwo: &str) -> Result<Vec<YamlDiff>, JsError> {
    let parsed_one: Result<serde_yml::Value, JsError> = match read_yaml(yone) {
        Ok(one) => Ok(one),
        Err(e) => {
            let error_message = match e.location() {
                Some(location) => {
                    format!("[YAML ONE] Error: {e} at line: {}", location.line())
                }
                None => format!("[YAML ONE] Error {e}"),
            };
            return Err(JsError::new(&error_message));
        }
    };
    let parsed_two: Result<serde_yml::Value, JsError> = match read_yaml(ytwo) {
        Ok(two) => Ok(two),
        Err(e) => {
            let error_message = match e.location() {
                Some(location) => {
                    format!("[YAML TWO] Error: {e} at line: {}", location.line())
                }
                None => format!("[YAML TWO] Error {e}"),
            };
            return Err(JsError::new(&error_message));
        }
    };
    match (parsed_one, parsed_two) {
        (Ok(one), Ok(two)) => Ok(yaml_diff(one, two)),
        (Err(e), _) => Err(e),
        (_, Err(e)) => Err(e),
    }
}
