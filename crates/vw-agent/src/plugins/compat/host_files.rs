//! Preserve the Codex freeform patch payload while using host file authorization.
use serde_json::{Value, json};

pub(crate) fn patch_input(input: Value) -> Value {
    match input {
        Value::String(patch) => json!({"patchText": patch}),
        other => other,
    }
}
