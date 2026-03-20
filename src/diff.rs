use wasm_bindgen::prelude::*;

const MAX_DEPTH: usize = 256;

/// When two sequences have a product of lengths exceeding this limit,
/// fall back to positional comparison instead of Myers diff to avoid
/// excessive memory/time on pathological inputs.
const SEQ_DIFF_PRODUCT_LIMIT: usize = 10_000_000;

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(test, derive(serde::Serialize))]
pub(crate) enum DiffType {
    Unchanged,
    Additions,
    Deletions,
    Modified,
}

#[derive(Clone, Debug)]
#[cfg_attr(test, derive(serde::Serialize))]
pub(crate) struct DiffValue {
    pub(crate) left_value: serde_yml::Value,
    pub(crate) right_value: serde_yml::Value,
}

impl DiffValue {
    pub(crate) fn new(left_value: serde_yml::Value, right_value: serde_yml::Value) -> Self {
        Self {
            left_value,
            right_value,
        }
    }
}

#[derive(Clone, Debug)]
#[cfg_attr(test, derive(serde::Serialize))]
pub(crate) struct YamlDiff {
    pub(crate) key: Option<String>,
    pub(crate) diff: DiffValue,
    pub(crate) has_diff: bool,
    pub(crate) diff_type: DiffType,
    pub(crate) children: Vec<YamlDiff>,
}

impl YamlDiff {
    pub(crate) fn new(
        key: Option<String>,
        diff: DiffValue,
        diff_type: DiffType,
        has_diff: bool,
        children: Vec<YamlDiff>,
    ) -> Self {
        Self {
            key,
            diff,
            has_diff,
            diff_type,
            children,
        }
    }
}

/// Convert a YamlDiff node to a plain JS object. Crosses the WASM boundary
/// once instead of requiring repeated getter calls that clone on every access.
pub(crate) fn diff_node_to_js(node: &YamlDiff) -> Result<JsValue, JsValue> {
    let obj = js_sys::Object::new();

    let key_val = match &node.key {
        Some(k) => JsValue::from_str(k),
        None => JsValue::NULL,
    };
    js_sys::Reflect::set(&obj, &JsValue::from_str("key"), &key_val)?;
    js_sys::Reflect::set(
        &obj,
        &JsValue::from_str("has_diff"),
        &JsValue::from_bool(node.has_diff),
    )?;

    let dt: u32 = match node.diff_type {
        DiffType::Unchanged => 0,
        DiffType::Additions => 1,
        DiffType::Deletions => 2,
        DiffType::Modified => 3,
    };
    js_sys::Reflect::set(&obj, &JsValue::from_str("diff_type"), &JsValue::from(dt))?;

    let diff_obj = js_sys::Object::new();
    let lv = to_js(&node.diff.left_value)?;
    let rv = to_js(&node.diff.right_value)?;
    js_sys::Reflect::set(&diff_obj, &JsValue::from_str("left_value"), &lv)?;
    js_sys::Reflect::set(&diff_obj, &JsValue::from_str("right_value"), &rv)?;
    js_sys::Reflect::set(&obj, &JsValue::from_str("diff"), &diff_obj)?;

    let children_arr = js_sys::Array::new();
    for child in &node.children {
        children_arr.push(&diff_node_to_js(child)?);
    }
    js_sys::Reflect::set(&obj, &JsValue::from_str("children"), &children_arr)?;

    Ok(obj.into())
}

/// Convert a Vec<YamlDiff> to a JS array of plain objects.
pub(crate) fn diff_vec_to_js(diffs: &[YamlDiff]) -> Result<JsValue, JsValue> {
    let arr = js_sys::Array::new();
    for node in diffs {
        arr.push(&diff_node_to_js(node)?);
    }
    Ok(arr.into())
}

/// Strip `Value::Tagged` wrappers so the inner value is used for
/// comparison, serialization, and type dispatch. serde_yml may parse
/// certain scalars (e.g. version strings like `3.0.1`) as Tagged,
/// and `TaggedValue::serialize` emits a mapping `{tag: value}` which
/// confuses both the diff logic and JS rendering.
pub(crate) fn unwrap_tagged(value: &serde_yml::Value) -> &serde_yml::Value {
    match value {
        serde_yml::Value::Tagged(tagged) => unwrap_tagged(&tagged.value),
        other => other,
    }
}

pub(crate) fn to_js(value: &serde_yml::Value) -> Result<JsValue, JsValue> {
    serde_wasm_bindgen::to_value(unwrap_tagged(value))
        .map_err(|e| JsValue::from_str(&format!("Serialization error: {e}")))
}

pub(crate) fn yaml_key_to_string(key: &serde_yml::Value) -> String {
    match unwrap_tagged(key) {
        serde_yml::Value::String(s) => s.clone(),
        serde_yml::Value::Number(n) => n.to_string(),
        serde_yml::Value::Bool(b) => b.to_string(),
        serde_yml::Value::Null => "null".to_string(),
        other => format!("{:?}", other),
    }
}

/// Serialize a YAML value to a canonical string for use as a comparison key
/// in the Myers diff algorithm.
fn serialize_value(v: &serde_yml::Value) -> String {
    let v = unwrap_tagged(v);
    serde_yml::to_string(v).unwrap_or_else(|_| format!("{v:?}"))
}

/// Recursively decompose a YAML value into child diff nodes for
/// pure additions or deletions, so complex values render as
/// expandable trees instead of flat `{}` / `[...]`.
fn value_to_diff_children(
    value: &serde_yml::Value,
    diff_type: &DiffType,
    depth: usize,
) -> Vec<YamlDiff> {
    if depth > MAX_DEPTH {
        return Vec::new();
    }
    let make_diff = |v: &serde_yml::Value| -> DiffValue {
        let v = unwrap_tagged(v).clone();
        match diff_type {
            DiffType::Deletions => DiffValue::new(v, serde_yml::Value::Null),
            DiffType::Additions => DiffValue::new(serde_yml::Value::Null, v),
            _ => unreachable!(),
        }
    };
    let value = unwrap_tagged(value);
    match value {
        serde_yml::Value::Mapping(map) => {
            let mut children = Vec::new();
            for (key, val) in map.iter() {
                let sub = value_to_diff_children(val, diff_type, depth + 1);
                children.push(YamlDiff::new(
                    Some(yaml_key_to_string(key)),
                    make_diff(val),
                    diff_type.clone(),
                    true,
                    sub,
                ));
            }
            children
        }
        serde_yml::Value::Sequence(seq) => {
            let mut children = Vec::new();
            for (i, val) in seq.iter().enumerate() {
                let sub = value_to_diff_children(val, diff_type, depth + 1);
                children.push(YamlDiff::new(
                    Some(i.to_string()),
                    make_diff(val),
                    diff_type.clone(),
                    true,
                    sub,
                ));
            }
            children
        }
        _ => Vec::new(),
    }
}

fn map_diff(
    left: &serde_yml::Mapping,
    right: &serde_yml::Mapping,
    depth: usize,
) -> Result<Vec<YamlDiff>, JsValue> {
    let mut diffs: Vec<YamlDiff> = Vec::new();

    for (key, value_one) in left.iter() {
        let key_str = yaml_key_to_string(key);
        match right.get(key) {
            Some(value_two) => {
                let child_diffs = yaml_diff(value_one, value_two, depth + 1)?;
                let any_child_has_diff = child_diffs.iter().any(|c| c.has_diff);
                diffs.push(YamlDiff {
                    key: Some(key_str),
                    diff: DiffValue {
                        left_value: unwrap_tagged(value_one).clone(),
                        right_value: unwrap_tagged(value_two).clone(),
                    },
                    diff_type: if any_child_has_diff {
                        DiffType::Modified
                    } else {
                        DiffType::Unchanged
                    },
                    has_diff: any_child_has_diff,
                    children: child_diffs,
                });
            }
            None => {
                diffs.push(YamlDiff {
                    key: Some(key_str),
                    diff: DiffValue {
                        left_value: unwrap_tagged(value_one).clone(),
                        right_value: serde_yml::Value::Null,
                    },
                    diff_type: DiffType::Deletions,
                    has_diff: true,
                    children: value_to_diff_children(value_one, &DiffType::Deletions, depth + 1),
                });
            }
        }
    }

    for (key, value_two) in right.iter() {
        if !left.contains_key(key) {
            diffs.push(YamlDiff {
                key: Some(yaml_key_to_string(key)),
                diff: DiffValue {
                    left_value: serde_yml::Value::Null,
                    right_value: unwrap_tagged(value_two).clone(),
                },
                diff_type: DiffType::Additions,
                has_diff: true,
                children: value_to_diff_children(value_two, &DiffType::Additions, depth + 1),
            });
        }
    }

    Ok(diffs)
}

/// Positional element-by-element comparison for very large sequences where
/// Myers diff would be too expensive.
fn positional_seq_diff(
    left: &serde_yml::Sequence,
    right: &serde_yml::Sequence,
    depth: usize,
) -> Result<Vec<YamlDiff>, JsValue> {
    let mut diffs: Vec<YamlDiff> = Vec::new();
    let max_len = std::cmp::max(left.len(), right.len());

    for i in 0..max_len {
        match (left.get(i), right.get(i)) {
            (Some(lv), Some(rv)) => {
                let child_diffs = yaml_diff(lv, rv, depth + 1)?;
                let any_child_has_diff = child_diffs.iter().any(|c| c.has_diff);
                diffs.push(YamlDiff {
                    key: Some(i.to_string()),
                    diff: DiffValue {
                        left_value: unwrap_tagged(lv).clone(),
                        right_value: unwrap_tagged(rv).clone(),
                    },
                    diff_type: if any_child_has_diff {
                        DiffType::Modified
                    } else {
                        DiffType::Unchanged
                    },
                    has_diff: any_child_has_diff,
                    children: child_diffs,
                });
            }
            (Some(lv), None) => {
                diffs.push(YamlDiff {
                    key: Some(i.to_string()),
                    diff: DiffValue {
                        left_value: unwrap_tagged(lv).clone(),
                        right_value: serde_yml::Value::Null,
                    },
                    diff_type: DiffType::Deletions,
                    has_diff: true,
                    children: value_to_diff_children(lv, &DiffType::Deletions, depth + 1),
                });
            }
            (None, Some(rv)) => {
                diffs.push(YamlDiff {
                    key: Some(i.to_string()),
                    diff: DiffValue {
                        left_value: serde_yml::Value::Null,
                        right_value: unwrap_tagged(rv).clone(),
                    },
                    diff_type: DiffType::Additions,
                    has_diff: true,
                    children: value_to_diff_children(rv, &DiffType::Additions, depth + 1),
                });
            }
            (None, None) => unreachable!(),
        }
    }

    Ok(diffs)
}

fn seq_diff(
    left: &serde_yml::Sequence,
    right: &serde_yml::Sequence,
    depth: usize,
) -> Result<Vec<YamlDiff>, JsValue> {
    // Size guard: fall back to positional comparison for very large sequences
    if left.len().saturating_mul(right.len()) > SEQ_DIFF_PRODUCT_LIMIT {
        return positional_seq_diff(left, right, depth);
    }

    let left_strs: Vec<String> = left.iter().map(serialize_value).collect();
    let right_strs: Vec<String> = right.iter().map(serialize_value).collect();
    let ops = similar::capture_diff_slices(similar::Algorithm::Myers, &left_strs, &right_strs);

    let mut diffs: Vec<YamlDiff> = Vec::new();
    let mut pos: usize = 0;

    for op in ops {
        match op {
            similar::DiffOp::Equal {
                old_index,
                new_index,
                len,
            } => {
                for i in 0..len {
                    let li = old_index + i;
                    let ri = new_index + i;
                    let child_diffs = yaml_diff(&left[li], &right[ri], depth + 1)?;
                    let any_child_has_diff = child_diffs.iter().any(|c| c.has_diff);
                    diffs.push(YamlDiff {
                        key: Some(pos.to_string()),
                        diff: DiffValue {
                            left_value: unwrap_tagged(&left[li]).clone(),
                            right_value: unwrap_tagged(&right[ri]).clone(),
                        },
                        diff_type: if any_child_has_diff {
                            DiffType::Modified
                        } else {
                            DiffType::Unchanged
                        },
                        has_diff: any_child_has_diff,
                        children: child_diffs,
                    });
                    pos += 1;
                }
            }
            similar::DiffOp::Delete {
                old_index, old_len, ..
            } => {
                for i in 0..old_len {
                    let li = old_index + i;
                    diffs.push(YamlDiff {
                        key: Some(pos.to_string()),
                        diff: DiffValue {
                            left_value: unwrap_tagged(&left[li]).clone(),
                            right_value: serde_yml::Value::Null,
                        },
                        diff_type: DiffType::Deletions,
                        has_diff: true,
                        children: value_to_diff_children(
                            &left[li],
                            &DiffType::Deletions,
                            depth + 1,
                        ),
                    });
                    pos += 1;
                }
            }
            similar::DiffOp::Insert {
                new_index, new_len, ..
            } => {
                for i in 0..new_len {
                    let ri = new_index + i;
                    diffs.push(YamlDiff {
                        key: Some(pos.to_string()),
                        diff: DiffValue {
                            left_value: serde_yml::Value::Null,
                            right_value: unwrap_tagged(&right[ri]).clone(),
                        },
                        diff_type: DiffType::Additions,
                        has_diff: true,
                        children: value_to_diff_children(
                            &right[ri],
                            &DiffType::Additions,
                            depth + 1,
                        ),
                    });
                    pos += 1;
                }
            }
            similar::DiffOp::Replace {
                old_index,
                old_len,
                new_index,
                new_len,
            } => {
                // Pair elements positionally for internal diff visibility,
                // then emit remaining unpaired elements as additions/deletions.
                let common = std::cmp::min(old_len, new_len);
                for i in 0..common {
                    let li = old_index + i;
                    let ri = new_index + i;
                    let child_diffs = yaml_diff(&left[li], &right[ri], depth + 1)?;
                    let any_child_has_diff = child_diffs.iter().any(|c| c.has_diff);
                    diffs.push(YamlDiff {
                        key: Some(pos.to_string()),
                        diff: DiffValue {
                            left_value: unwrap_tagged(&left[li]).clone(),
                            right_value: unwrap_tagged(&right[ri]).clone(),
                        },
                        diff_type: if any_child_has_diff {
                            DiffType::Modified
                        } else {
                            DiffType::Unchanged
                        },
                        has_diff: any_child_has_diff,
                        children: child_diffs,
                    });
                    pos += 1;
                }
                for i in common..old_len {
                    let li = old_index + i;
                    diffs.push(YamlDiff {
                        key: Some(pos.to_string()),
                        diff: DiffValue {
                            left_value: unwrap_tagged(&left[li]).clone(),
                            right_value: serde_yml::Value::Null,
                        },
                        diff_type: DiffType::Deletions,
                        has_diff: true,
                        children: value_to_diff_children(
                            &left[li],
                            &DiffType::Deletions,
                            depth + 1,
                        ),
                    });
                    pos += 1;
                }
                for i in common..new_len {
                    let ri = new_index + i;
                    diffs.push(YamlDiff {
                        key: Some(pos.to_string()),
                        diff: DiffValue {
                            left_value: serde_yml::Value::Null,
                            right_value: unwrap_tagged(&right[ri]).clone(),
                        },
                        diff_type: DiffType::Additions,
                        has_diff: true,
                        children: value_to_diff_children(
                            &right[ri],
                            &DiffType::Additions,
                            depth + 1,
                        ),
                    });
                    pos += 1;
                }
            }
        }
    }

    Ok(diffs)
}

fn val_diff(left: &serde_yml::Value, right: &serde_yml::Value) -> Vec<YamlDiff> {
    let has_diff = left != right;
    let diff_type = if !has_diff {
        DiffType::Unchanged
    } else {
        match (left, right) {
            (serde_yml::Value::Null, _) => DiffType::Additions,
            (_, serde_yml::Value::Null) => DiffType::Deletions,
            _ => DiffType::Modified,
        }
    };

    vec![YamlDiff {
        key: None,
        diff: DiffValue {
            left_value: left.clone(),
            right_value: right.clone(),
        },
        has_diff,
        diff_type,
        children: Vec::new(),
    }]
}

pub fn yaml_diff(
    left: &serde_yml::Value,
    right: &serde_yml::Value,
    depth: usize,
) -> Result<Vec<YamlDiff>, JsValue> {
    if depth > MAX_DEPTH {
        return Err(JsValue::from_str(
            "Maximum nesting depth exceeded (256 levels)",
        ));
    }

    let left = unwrap_tagged(left);
    let right = unwrap_tagged(right);

    match (left, right) {
        (serde_yml::Value::Mapping(map_one), serde_yml::Value::Mapping(map_two)) => {
            map_diff(map_one, map_two, depth)
        }
        (serde_yml::Value::Sequence(seq_one), serde_yml::Value::Sequence(seq_two)) => {
            seq_diff(seq_one, seq_two, depth)
        }
        (one, two) => Ok(val_diff(one, two)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unwrap_tagged_strips_tag() {
        // serde_yml parses version-like strings (e.g. 3.0.1) as Tagged
        let value: serde_yml::Value = serde_yml::from_str("3.0.1").unwrap();
        let unwrapped = unwrap_tagged(&value);
        assert!(
            matches!(unwrapped, serde_yml::Value::String(s) if s == "3.0.1"),
            "expected String(\"3.0.1\"), got {unwrapped:?}",
        );
    }

    #[test]
    fn unwrap_tagged_passthrough() {
        let value: serde_yml::Value = serde_yml::from_str("hello").unwrap();
        let unwrapped = unwrap_tagged(&value);
        assert!(
            matches!(unwrapped, serde_yml::Value::String(s) if s == "hello"),
            "expected String(\"hello\"), got {unwrapped:?}",
        );
    }
}
