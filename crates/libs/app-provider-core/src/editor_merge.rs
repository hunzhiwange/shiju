//! Claude editor changes. Baselines are executor-only; shared projection owns field classification.
use app_config_patch::patch_json::JsonPatch;
use serde_json::Value;
use vw_api_app_provider_edit::editor_merge::*;
#[derive(Debug, PartialEq, Eq)]
pub enum MergeError {
    Invalid,
    Conflict(Vec<Vec<String>>),
}
pub fn merge(
    base: &Value,
    current: &[u8],
    edits: &[EditorEdit],
    choice: EditorChoice,
    global: bool,
) -> Result<Vec<u8>, MergeError> {
    let now: Value = serde_json::from_slice(current).map_err(|_| MergeError::Invalid)?;
    if !base.is_object() || !now.is_object() || edits.len() > 128 {
        return Err(MergeError::Invalid);
    }
    let mut patch = JsonPatch::default();
    let mut conflicts = vec![];
    for (index, edit) in edits.iter().enumerate() {
        let path = &edit.path;
        if path.is_empty()
            || path.len() > 64
            || path.iter().any(|k| k.is_empty() || k.len() > 256 || k.chars().any(char::is_control))
            || edits[..index].iter().any(|e| e.path.starts_with(path) || path.starts_with(&e.path))
        {
            return Err(MergeError::Invalid);
        }
        if global {
            use app_provider_projection::projection::fields;
            let root = path[0].as_str();
            if fields::claude_floor_top(root)
                || ["api_format", "apiFormat", "openrouter_compat_mode", "openrouterCompatMode"]
                    .contains(&root)
                || root == "env"
                    && (path.len() < 2
                        || fields::claude_floor_env(&path[1])
                        || fields::claude_exclusive_env(&path[1]))
            {
                return Err(MergeError::Invalid);
            }
        }
        let after = match &edit.value {
            EditorValue::Set { value } => {
                if placeholder(value) {
                    return Err(MergeError::Invalid);
                }
                Some(value)
            }
            EditorValue::Remove => None,
        };
        let before = at(base, path);
        // Sending an unchanged display value cannot revert an external edit.
        if before == after {
            continue;
        }
        let current_value = at(&now, path);
        if current_value != before && current_value != after {
            conflicts.push(path.clone());
            if choice == EditorChoice::KeepTheirs {
                continue;
            }
        }
        match after {
            Some(value) => patch.set.push((path.clone(), value.clone())),
            None => patch.remove.push(path.clone()),
        }
    }
    if !conflicts.is_empty() && choice == EditorChoice::Refuse {
        return Err(MergeError::Conflict(conflicts));
    }
    let result = patch.apply(Some(current)).map_err(|_| MergeError::Invalid)?;
    let value: Value = serde_json::from_slice(&result).map_err(|_| MergeError::Invalid)?;
    if value.get("env").is_some_and(|env| !env.is_object()) {
        return Err(MergeError::Invalid);
    }
    Ok(result)
}

fn at<'a>(value: &'a Value, path: &[String]) -> Option<&'a Value> {
    let mut current = value;
    for key in path {
        current = current.as_object()?.get(key)?;
    }
    Some(current)
}
fn placeholder(value: &Value) -> bool {
    match value {
        Value::String(s) => matches!(s.trim(), "[redacted]" | "<redacted>"),
        Value::Array(a) => a.iter().any(placeholder),
        Value::Object(m) => m.values().any(placeholder),
        _ => false,
    }
}
