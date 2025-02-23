use std::collections::BTreeMap;
use wasm_bindgen::prelude::*;
use serde_yml;

// Reads and parses a YAML file into a BTreeMap
fn read_yaml(data: &str) -> Result<BTreeMap<String, serde_yml::Value>, serde_yml::Error> {
    let parsed_data: BTreeMap<String, serde_yml::Value> = serde_yml::from_str(&data)?;
    Ok(parsed_data)
}

fn yaml_diff(one: BTreeMap<String, serde_yml::Value>, two: BTreeMap<String, serde_yml::Value>) -> String {
    let mut diff = String::new();
    for (key, value_one) in one.iter() {
        match two.get(key) {
            Some(value_two) => {
                if value_one != value_two {
                    diff.push_str(&format!("~ {}: {:?} != {:?}\n", key, value_one, value_two));
                }
            }
            None => {
                diff.push_str(&format!("+ {}: {:?}\n", key, value_one));
            }
        }
    }
    for (key, value_two) in two.iter() {
        if !one.contains_key(key) {
            diff.push_str(&format!("- {}: {:?}\n", key, value_two));
        }
    }
    diff
}

#[wasm_bindgen]
pub fn diff(yone: &str, ytwo: &str) -> Result<String, JsError> {
    let parsed_one: Result<BTreeMap<String, serde_yml::Value>, JsError> = match read_yaml(yone) {
        Ok(one) => Ok(one),
        Err(e) => {
            let error_message = match e.location(){
                Some(location) => format!("[YAML ONE] Error at line: {}", location.line()),
                None => format!("[YAML ONE] Error {}", e.to_string()),
            };
            return Err(JsError::new(&error_message));
        },
    };
    let parsed_two: Result<BTreeMap<String, serde_yml::Value>, JsError> = match read_yaml(ytwo) {
        Ok(two) => Ok(two),
        Err(e) => {
            let error_message = match e.location(){
                Some(location) => format!("[YAML TWO] Error at line: {}", location.line()),
                None => format!("[YAML TWO] Error {}", e.to_string()),
            };
            return Err(JsError::new(&error_message));
        },
    };
    return match (parsed_one, parsed_two) {
        (Ok(one), Ok(two)) => {
            Ok(yaml_diff(one, two))
        }
        (Err(e), _) => {
            Err(e)
        }
        (_, Err(e)) => {
            Err(e)
        }
    };
}
