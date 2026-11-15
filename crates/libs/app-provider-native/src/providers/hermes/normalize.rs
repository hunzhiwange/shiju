use super::*;

/// Normalize form aliases and model arrays without reading environment credentials.
pub fn normalize_declaration(v: &Value) -> Result<Value, WriteError> {
    let mut obj = v.as_object().cloned().ok_or(WriteError::InvalidResource)?;
    for (old, new) in [
        ("baseUrl", "base_url"),
        ("apiKey", "api_key"),
        ("apiMode", "api_mode"),
        ("maxTokens", "max_tokens"),
        ("contextLength", "context_length"),
    ] {
        if let Some(v) = obj.remove(old) {
            obj.entry(new).or_insert(v);
        }
    }
    for key in ["api", "_cc_source", "provider_key", "name"] {
        obj.remove(key);
    }
    for key in ["base_url", "api_key", "api_mode", "key_env", "model"] {
        if obj.get(key).is_some_and(|v| !v.is_string()) {
            return invalid();
        }
    }
    if obj.get("key_env").and_then(Value::as_str).is_some_and(|s| {
        s.is_empty()
            || !s
                .bytes()
                .enumerate()
                .all(|(i, c)| c.is_ascii_alphabetic() || c == b'_' || (i > 0 && c.is_ascii_digit()))
    }) {
        return invalid();
    }
    if obj.get("api_key").and_then(Value::as_str).is_some_and(|s| {
        matches!(s, "[redacted]" | "<redacted>")
            || (!s.is_empty() && s.chars().all(|c| matches!(c, '*' | '•')))
    }) {
        return invalid();
    }
    if let Some(models) = obj.remove("models") {
        let mut map = serde_json::Map::new();
        let mut first = None;
        match models {
            Value::Array(models) => {
                for model in models {
                    let mut m = model.as_object().cloned().ok_or(WriteError::InvalidResource)?;
                    let id = m
                        .remove("id")
                        .and_then(|v| v.as_str().map(str::to_string))
                        .ok_or(WriteError::InvalidResource)?;
                    if !label(&id) || map.contains_key(&id) {
                        return invalid();
                    }
                    first.get_or_insert(id.clone());
                    map.insert(id, Value::Object(m));
                }
            }
            Value::Object(models) => {
                for (id, v) in models {
                    if !label(&id) || !(v.is_object() || v.is_null()) {
                        return invalid();
                    }
                    first.get_or_insert(id.clone());
                    map.insert(id, if v.is_null() { json!({}) } else { v });
                }
            }
            _ => return invalid(),
        }
        if let Some(id) = first {
            if !obj.contains_key("model") {
                obj.insert("model".into(), json!(id));
            }
        } else {
            obj.remove("model");
        }
        obj.insert("models".into(), Value::Object(map));
    }
    Ok(Value::Object(obj))
}
