use wasm_bindgen::prelude::*;

#[wasm_bindgen]
#[derive(Clone, Debug)]
pub enum DiffType {
    Additions,
    Deletions,
    Conflicts,
}

#[wasm_bindgen]
#[derive(Clone, Debug)]
pub struct DiffValue {
    left_value: JsValue,
    right_value: JsValue,
}

#[wasm_bindgen]
impl DiffValue {
    #[wasm_bindgen(getter)]
    pub fn left_value(&self) -> JsValue {
        self.left_value.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn right_value(&self) -> JsValue {
        self.right_value.clone()
    }
}

#[wasm_bindgen]
#[derive(Clone, Debug)]
pub struct YamlDiff {
    key: Option<String>,
    diff: DiffValue,
    has_diff: bool,
    diff_type: DiffType,
    children: Vec<YamlDiff>,
}

#[wasm_bindgen]
impl YamlDiff {
    #[wasm_bindgen(getter)]
    pub fn key(&self) -> Option<String> {
        self.key.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn diff(&self) -> DiffValue {
        self.diff.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn has_diff(&self) -> bool {
        self.has_diff
    }

    #[wasm_bindgen(getter)]
    pub fn diff_type(&self) -> DiffType {
        self.diff_type.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn children(&self) -> Vec<YamlDiff> {
        self.children.clone()
    }
}

fn map_diff(left: serde_yml::Mapping, right: serde_yml::Mapping) -> Vec<YamlDiff> {
    let mut diffs: Vec<YamlDiff> = Vec::new();

    for (key, value_one) in left.iter() {
        match right.get(key) {
            Some(value_two) => {
                let child_diffs = yaml_diff(value_one.clone(), value_two.clone());
                let mut child = YamlDiff {
                    key: Some(key.as_str().unwrap_or("").to_string()),
                    diff: DiffValue {
                        left_value: serde_wasm_bindgen::to_value(value_one).unwrap(),
                        right_value: serde_wasm_bindgen::to_value(value_two).unwrap(),
                    },
                    diff_type: DiffType::Additions,
                    has_diff: true,
                    children: child_diffs,
                };
                diffs.push(child);
            }
            None => {
                let child = YamlDiff {
                    key: Some(key.as_str().unwrap_or("").to_string()),
                    diff: DiffValue {
                        left_value: serde_wasm_bindgen::to_value(value_one).unwrap(),
                        right_value: JsValue::NULL,
                    },
                    diff_type: DiffType::Additions,
                    has_diff: true,
                    children: Vec::new(),
                };
                diffs.push(child);
            }
        }
    }
    for (key, value_two) in right.iter() {
        if !left.contains_key(key) {
            let child = YamlDiff {
                key: Some(key.as_str().unwrap_or("").to_string()),
                diff: DiffValue {
                    left_value: JsValue::NULL,
                    right_value: serde_wasm_bindgen::to_value(value_two).unwrap(),
                },
                diff_type: DiffType::Additions,
                    has_diff: true,
                children: Vec::new(),
            };
            diffs.push(child);
        }
    }
    diffs
}

fn seq_diff(left: serde_yml::Sequence, right: serde_yml::Sequence) -> Vec<YamlDiff> {
    let mut diffs: Vec<YamlDiff> = Vec::new();

    let max_len = std::cmp::max(left.len(), right.len());
    for i in 0..max_len {
        match (left.get(i), right.get(i)) {
            (
                Some(serde_yml::Value::Mapping(left_value)),
                Some(serde_yml::Value::Mapping(right_value)),
            ) => {
                let child_diffs = map_diff(left_value.clone(), right_value.clone());
                let mut child = YamlDiff {
                    key: Some(i.to_string()),
                    diff: DiffValue {
                        left_value: serde_wasm_bindgen::to_value(left_value).unwrap(),
                        right_value: serde_wasm_bindgen::to_value(right_value).unwrap(),
                    },
                    diff_type: DiffType::Additions,
                    has_diff: true,
                    children: child_diffs,
                };
                diffs.push(child);
            }
            (
                Some(serde_yml::Value::Sequence(left_value)),
                Some(serde_yml::Value::Sequence(right_value)),
            ) => {
                let child_diffs = seq_diff(left_value.clone(), right_value.clone());
                let mut child = YamlDiff {
                    key: Some(i.to_string()),
                    diff: DiffValue {
                        left_value: serde_wasm_bindgen::to_value(left_value).unwrap(),
                        right_value: serde_wasm_bindgen::to_value(right_value).unwrap(),
                    },
                    diff_type: DiffType::Additions,
                    has_diff: true,
                    children: child_diffs,
                };
                diffs.push(child);
            }
            (Some(left_value), Some(right_value)) => {
                let child = YamlDiff {
                    key: Some(i.to_string()),
                    diff: DiffValue {
                        left_value: serde_wasm_bindgen::to_value(left_value).unwrap(),
                        right_value: serde_wasm_bindgen::to_value(right_value).unwrap(),
                    },
                    diff_type: DiffType::Additions,
                    has_diff: true,
                    children: Vec::new(),
                };
                diffs.push(child);
            }
            (Some(left_value), None) => {
                let child = YamlDiff {
                    key: Some(i.to_string()),
                    diff: DiffValue {
                        left_value: serde_wasm_bindgen::to_value(left_value).unwrap(),
                        right_value: JsValue::NULL,
                    },
                    diff_type: DiffType::Additions,
                    has_diff: true,
                    children: Vec::new(),
                };
                diffs.push(child);
            }
            (None, Some(right_value)) => {
                let child = YamlDiff {
                    key: Some(i.to_string()),
                    diff: DiffValue {
                        left_value: JsValue::NULL,
                        right_value: serde_wasm_bindgen::to_value(right_value).unwrap(),
                    },
                    diff_type: DiffType::Additions,
                    has_diff: true,
                    children: Vec::new(),
                };
                diffs.push(child);
            }
            (None, None) => {}
        }
    }
    diffs
}

fn val_diff(left: serde_yml::Value, right: serde_yml::Value) -> Vec<YamlDiff> {
    let mut diffs: Vec<YamlDiff> = Vec::new();
    let child = YamlDiff {
        key: None,
        diff: DiffValue {
            left_value: serde_wasm_bindgen::to_value(&left).unwrap(),
            right_value: serde_wasm_bindgen::to_value(&right).unwrap(),
        },
        has_diff: left != right,
        diff_type: match (left, right) {
            (serde_yml::Value::Null, _) => DiffType::Additions,
            (_, serde_yml::Value::Null) => DiffType::Deletions,
            _ => DiffType::Conflicts,
        },
        children: Vec::new(),
    };
    diffs.push(child);
    diffs
}

pub fn yaml_diff(left: serde_yml::Value, right: serde_yml::Value) -> Vec<YamlDiff> {
    match (left, right) {
        (serde_yml::Value::Mapping(map_one), serde_yml::Value::Mapping(map_two)) => {
            map_diff(map_one.clone(), map_two.clone())
        }
        (serde_yml::Value::Sequence(seq_one), serde_yml::Value::Sequence(seq_two)) => {
            seq_diff(seq_one.clone(), seq_two.clone())
        }
        (one, two) => val_diff(one, two),
    }
}
