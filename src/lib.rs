mod diff;

use diff::{diff_vec_to_js, yaml_diff};
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

/// Compute a complete diff of two YAML strings in one call.
/// Handles all top-level types: mappings, sequences, scalars, and mixed.
#[wasm_bindgen]
pub fn compute_diff(yone: &str, ytwo: &str) -> Result<JsValue, JsError> {
    let (one, two) = validate_and_parse(yone, ytwo)?;
    let diffs =
        yaml_diff(&one, &two, 0).map_err(|e| JsError::new(&format!("Diff error: {e:?}")))?;
    diff_vec_to_js(&diffs).map_err(|e| JsError::new(&format!("Serialization error: {e:?}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wrap_in_root(value: serde_yml::Value) -> serde_yml::Value {
        let mut map = serde_yml::Mapping::new();
        map.insert(serde_yml::Value::String("__root__".to_string()), value);
        serde_yml::Value::Mapping(map)
    }

    #[test]
    fn wrap_in_root_scalar() {
        let value = serde_yml::from_str::<serde_yml::Value>("hello").unwrap();
        let wrapped = wrap_in_root(value);
        insta::assert_snapshot!(serde_yml::to_string(&wrapped).unwrap());
    }

    #[test]
    fn wrap_in_root_sequence() {
        let value = serde_yml::from_str::<serde_yml::Value>("- a\n- b").unwrap();
        let wrapped = wrap_in_root(value);
        insta::assert_snapshot!(serde_yml::to_string(&wrapped).unwrap());
    }

    #[test]
    fn wrap_in_root_mapping() {
        let value = serde_yml::from_str::<serde_yml::Value>("a: 1").unwrap();
        let wrapped = wrap_in_root(value);
        insta::assert_snapshot!(serde_yml::to_string(&wrapped).unwrap());
    }

    #[test]
    fn read_yaml_valid() {
        let result = read_yaml("a: 1\nb: 2").unwrap();
        insta::assert_snapshot!(serde_yml::to_string(&result).unwrap());
    }

    fn compute_diff_test(yone: &str, ytwo: &str) -> Vec<diff::YamlDiff> {
        let (one, two) = validate_and_parse(yone, ytwo).unwrap();
        diff::yaml_diff(&one, &two, 0).unwrap()
    }

    #[test]
    fn e2e_scalar_scalar() {
        let diffs = compute_diff_test("hello", "world");
        insta::assert_yaml_snapshot!(diffs);
    }

    #[test]
    fn e2e_scalar_map() {
        let diffs = compute_diff_test("hello", "a: 1");
        insta::assert_yaml_snapshot!(diffs);
    }

    #[test]
    fn e2e_scalar_array() {
        let diffs = compute_diff_test("hello", "- a\n- b");
        insta::assert_yaml_snapshot!(diffs);
    }

    #[test]
    fn e2e_map_map() {
        let diffs = compute_diff_test("a: 1\nb: old", "a: 1\nb: new");
        insta::assert_yaml_snapshot!(diffs);
    }

    #[test]
    fn e2e_map_array() {
        let diffs = compute_diff_test("a: 1", "- a\n- b");
        insta::assert_yaml_snapshot!(diffs);
    }

    #[test]
    fn e2e_array_array() {
        let diffs = compute_diff_test("- a\n- b", "- a\n- c");
        insta::assert_yaml_snapshot!(diffs);
    }

    #[test]
    fn e2e_scalar_int_keyed_array() {
        let diffs = compute_diff_test("1", "hi:\n  - a\n  - b\n  - c\n");
        insta::assert_yaml_snapshot!(diffs);
    }
}
