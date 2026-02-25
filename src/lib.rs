mod diff;

use std::cell::RefCell;

use diff::{
    diff_node_to_js, diff_vec_to_js, to_js, yaml_diff, yaml_key_to_string, DiffType, DiffValue,
    YamlDiff,
};
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

fn validate_and_parse(
    yone: &str,
    ytwo: &str,
) -> Result<(serde_yml::Value, serde_yml::Value), JsError> {
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
        (Ok(one), Ok(two)) => Ok((one, two)),
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

// ── Chunked diff API ────────────────────────────────

struct DiffState {
    left: serde_yml::Value,
    right: serde_yml::Value,
}

thread_local! {
    static DIFF_STATE: RefCell<Option<DiffState>> = const { RefCell::new(None) };
}

/// Find a value in a mapping by its string-converted key.
/// Handles non-string YAML keys (numbers, bools, null) that were
/// converted to strings by `yaml_key_to_string` during `diff_init`.
fn find_value_by_key<'a>(
    map: &'a serde_yml::Mapping,
    key_str: &str,
) -> Option<&'a serde_yml::Value> {
    for (k, v) in map.iter() {
        if yaml_key_to_string(k) == key_str {
            return Some(v);
        }
    }
    None
}

/// Parse both YAML strings and store for chunked diffing.
/// Returns top-level keys if both are mappings, empty vec otherwise.
#[wasm_bindgen]
pub fn diff_init(yone: &str, ytwo: &str) -> Result<Vec<String>, JsError> {
    let (one, two) = validate_and_parse(yone, ytwo)?;

    let keys = match (&one, &two) {
        (serde_yml::Value::Mapping(m1), serde_yml::Value::Mapping(m2)) => {
            let mut keys: Vec<String> = m1.keys().map(yaml_key_to_string).collect();
            for key in m2.keys() {
                let ks = yaml_key_to_string(key);
                if !keys.contains(&ks) {
                    keys.push(ks);
                }
            }
            keys
        }
        _ => Vec::new(),
    };

    DIFF_STATE.with(|state| {
        let mut state = state
            .try_borrow_mut()
            .map_err(|e| JsError::new(&format!("State borrow error: {e}")))?;
        *state = Some(DiffState {
            left: one,
            right: two,
        });
        Ok(keys)
    })
}

/// Diff a single top-level key from stored state.
/// Returns a plain JS object (no wasm-bindgen struct) to avoid cloning overhead.
#[wasm_bindgen]
pub fn diff_key(key: &str) -> Result<JsValue, JsError> {
    DIFF_STATE.with(|state| {
        let state = state
            .try_borrow()
            .map_err(|e| JsError::new(&format!("State borrow error: {e}")))?;
        let state = state
            .as_ref()
            .ok_or_else(|| JsError::new("diff_init not called"))?;

        let left_map = state
            .left
            .as_mapping()
            .ok_or_else(|| JsError::new("Left value is not a mapping"))?;
        let right_map = state
            .right
            .as_mapping()
            .ok_or_else(|| JsError::new("Right value is not a mapping"))?;

        let left_val = find_value_by_key(left_map, key);
        let right_val = find_value_by_key(right_map, key);

        let node = match (left_val, right_val) {
            (Some(lv), Some(rv)) => {
                let children = yaml_diff(lv, rv, 0)
                    .map_err(|e| JsError::new(&format!("Diff error: {e:?}")))?;
                let any_child_has_diff = children.iter().any(|c| c.has_diff);
                YamlDiff::new(
                    Some(key.to_string()),
                    DiffValue::new(
                        to_js(lv).map_err(|e| JsError::new(&format!("{e:?}")))?,
                        to_js(rv).map_err(|e| JsError::new(&format!("{e:?}")))?,
                    ),
                    if any_child_has_diff {
                        DiffType::Modified
                    } else {
                        DiffType::Unchanged
                    },
                    any_child_has_diff,
                    children,
                )
            }
            (Some(lv), None) => YamlDiff::new(
                Some(key.to_string()),
                DiffValue::new(
                    to_js(lv).map_err(|e| JsError::new(&format!("{e:?}")))?,
                    JsValue::NULL,
                ),
                DiffType::Deletions,
                true,
                Vec::new(),
            ),
            (None, Some(rv)) => YamlDiff::new(
                Some(key.to_string()),
                DiffValue::new(
                    JsValue::NULL,
                    to_js(rv).map_err(|e| JsError::new(&format!("{e:?}")))?,
                ),
                DiffType::Additions,
                true,
                Vec::new(),
            ),
            (None, None) => return Err(JsError::new(&format!("Key not found: {key}"))),
        };

        diff_node_to_js(&node).map_err(|e| JsError::new(&format!("Serialization error: {e:?}")))
    })
}

/// Diff stored state as a single operation (fallback for non-mapping top-levels).
/// Returns a JS array of plain objects.
#[wasm_bindgen]
pub fn diff_stored() -> Result<JsValue, JsError> {
    DIFF_STATE.with(|state| {
        let state = state
            .try_borrow()
            .map_err(|e| JsError::new(&format!("State borrow error: {e}")))?;
        let state = state
            .as_ref()
            .ok_or_else(|| JsError::new("diff_init not called"))?;
        let diffs = yaml_diff(&state.left, &state.right, 0)
            .map_err(|e| JsError::new(&format!("Diff error: {e:?}")))?;
        diff_vec_to_js(&diffs).map_err(|e| JsError::new(&format!("Serialization error: {e:?}")))
    })
}

/// Free stored diff state.
#[wasm_bindgen]
pub fn diff_cleanup() -> Result<(), JsError> {
    DIFF_STATE.with(|state| {
        let mut state = state
            .try_borrow_mut()
            .map_err(|e| JsError::new(&format!("State borrow error: {e}")))?;
        *state = None;
        Ok(())
    })
}
