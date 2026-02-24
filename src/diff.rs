use wasm_bindgen::prelude::*;

#[wasm_bindgen]
#[derive(Clone, Debug, PartialEq)]
pub enum DiffType {
    Unchanged,
    Additions,
    Deletions,
    Modified,
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

fn to_js(value: &serde_yml::Value) -> Result<JsValue, JsValue> {
    serde_wasm_bindgen::to_value(value)
        .map_err(|e| JsValue::from_str(&format!("Serialization error: {e}")))
}

fn yaml_key_to_string(key: &serde_yml::Value) -> String {
    match key {
        serde_yml::Value::String(s) => s.clone(),
        serde_yml::Value::Number(n) => n.to_string(),
        serde_yml::Value::Bool(b) => b.to_string(),
        serde_yml::Value::Null => "null".to_string(),
        other => format!("{:?}", other),
    }
}

/// Compute LCS (Longest Common Subsequence) indices between two sequences.
/// Returns pairs of (left_index, right_index) for matching elements.
fn lcs_indices(left: &serde_yml::Sequence, right: &serde_yml::Sequence) -> Vec<(usize, usize)> {
    let n = left.len();
    let m = right.len();
    let mut dp = vec![vec![0usize; m + 1]; n + 1];

    for i in 1..=n {
        for j in 1..=m {
            if left[i - 1] == right[j - 1] {
                dp[i][j] = dp[i - 1][j - 1] + 1;
            } else {
                dp[i][j] = std::cmp::max(dp[i - 1][j], dp[i][j - 1]);
            }
        }
    }

    let mut result = Vec::new();
    let (mut i, mut j) = (n, m);
    while i > 0 && j > 0 {
        if left[i - 1] == right[j - 1] {
            result.push((i - 1, j - 1));
            i -= 1;
            j -= 1;
        } else if dp[i - 1][j] > dp[i][j - 1] {
            i -= 1;
        } else {
            j -= 1;
        }
    }
    result.reverse();
    result
}

fn map_diff(
    left: &serde_yml::Mapping,
    right: &serde_yml::Mapping,
) -> Result<Vec<YamlDiff>, JsValue> {
    let mut diffs: Vec<YamlDiff> = Vec::new();

    for (key, value_one) in left.iter() {
        let key_str = yaml_key_to_string(key);
        match right.get(key) {
            Some(value_two) => {
                let child_diffs = yaml_diff(value_one, value_two)?;
                let any_child_has_diff = child_diffs.iter().any(|c| c.has_diff);
                diffs.push(YamlDiff {
                    key: Some(key_str),
                    diff: DiffValue {
                        left_value: to_js(value_one)?,
                        right_value: to_js(value_two)?,
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
                        left_value: to_js(value_one)?,
                        right_value: JsValue::NULL,
                    },
                    diff_type: DiffType::Deletions,
                    has_diff: true,
                    children: Vec::new(),
                });
            }
        }
    }

    for (key, value_two) in right.iter() {
        if !left.contains_key(key) {
            diffs.push(YamlDiff {
                key: Some(yaml_key_to_string(key)),
                diff: DiffValue {
                    left_value: JsValue::NULL,
                    right_value: to_js(value_two)?,
                },
                diff_type: DiffType::Additions,
                has_diff: true,
                children: Vec::new(),
            });
        }
    }

    Ok(diffs)
}

fn seq_diff(
    left: &serde_yml::Sequence,
    right: &serde_yml::Sequence,
) -> Result<Vec<YamlDiff>, JsValue> {
    let mut diffs: Vec<YamlDiff> = Vec::new();
    let lcs = lcs_indices(left, right);

    let mut li = 0;
    let mut ri = 0;
    let mut lcs_idx = 0;
    let mut pos = 0;

    while li < left.len() || ri < right.len() {
        let at_lcs = lcs_idx < lcs.len() && lcs[lcs_idx] == (li, ri);

        if at_lcs {
            // Matched pair — recurse for internal diffs
            let child_diffs = yaml_diff(&left[li], &right[ri])?;
            let any_child_has_diff = child_diffs.iter().any(|c| c.has_diff);
            diffs.push(YamlDiff {
                key: Some(pos.to_string()),
                diff: DiffValue {
                    left_value: to_js(&left[li])?,
                    right_value: to_js(&right[ri])?,
                },
                diff_type: if any_child_has_diff {
                    DiffType::Modified
                } else {
                    DiffType::Unchanged
                },
                has_diff: any_child_has_diff,
                children: child_diffs,
            });
            li += 1;
            ri += 1;
            lcs_idx += 1;
        } else {
            // Emit deletions from left until we reach the next LCS left index
            let next_lcs_li = if lcs_idx < lcs.len() {
                lcs[lcs_idx].0
            } else {
                left.len()
            };
            while li < next_lcs_li {
                diffs.push(YamlDiff {
                    key: Some(pos.to_string()),
                    diff: DiffValue {
                        left_value: to_js(&left[li])?,
                        right_value: JsValue::NULL,
                    },
                    diff_type: DiffType::Deletions,
                    has_diff: true,
                    children: Vec::new(),
                });
                li += 1;
                pos += 1;
            }

            // Emit additions from right until we reach the next LCS right index
            let next_lcs_ri = if lcs_idx < lcs.len() {
                lcs[lcs_idx].1
            } else {
                right.len()
            };
            while ri < next_lcs_ri {
                diffs.push(YamlDiff {
                    key: Some(pos.to_string()),
                    diff: DiffValue {
                        left_value: JsValue::NULL,
                        right_value: to_js(&right[ri])?,
                    },
                    diff_type: DiffType::Additions,
                    has_diff: true,
                    children: Vec::new(),
                });
                ri += 1;
                pos += 1;
            }

            continue;
        }
        pos += 1;
    }

    Ok(diffs)
}

fn val_diff(left: &serde_yml::Value, right: &serde_yml::Value) -> Result<Vec<YamlDiff>, JsValue> {
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

    Ok(vec![YamlDiff {
        key: None,
        diff: DiffValue {
            left_value: to_js(left)?,
            right_value: to_js(right)?,
        },
        has_diff,
        diff_type,
        children: Vec::new(),
    }])
}

pub fn yaml_diff(
    left: &serde_yml::Value,
    right: &serde_yml::Value,
) -> Result<Vec<YamlDiff>, JsValue> {
    match (left, right) {
        (serde_yml::Value::Mapping(map_one), serde_yml::Value::Mapping(map_two)) => {
            map_diff(map_one, map_two)
        }
        (serde_yml::Value::Sequence(seq_one), serde_yml::Value::Sequence(seq_two)) => {
            seq_diff(seq_one, seq_two)
        }
        (one, two) => val_diff(one, two),
    }
}
