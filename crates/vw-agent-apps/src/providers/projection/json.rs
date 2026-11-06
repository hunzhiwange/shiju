//! Claude and Gemini pure editor projections, adapted from live/project and services/provider.
use super::{Error, Projected, fields};
use serde_json::{Map, Value, json};
fn object(value: &Value) -> Map<String, Value> {
    value.as_object().cloned().unwrap_or_default()
}
fn env(value: &Value) -> Map<String, Value> {
    object(&value["env"])
}

pub fn claude(
    live: Value,
    row: &Value,
    previous: Option<&Value>,
    exclusive: Option<&Map<String, Value>>,
) -> Result<Projected, Error> {
    if live.get("env").is_some_and(|v| !v.is_object()) {
        return Err(Error::Shape);
    }
    let mut shown = live.clone();
    let root = shown.as_object_mut().ok_or(Error::Shape)?;
    root.retain(|k, _| !fields::claude_floor_top(k));
    for (k, v) in object(row) {
        if fields::claude_floor_top(&k) {
            root.insert(k, v);
        }
    }
    let mut out = env(&live);
    out.retain(|k, _| !fields::claude_floor_env(k));
    let outgoing = exclusive.cloned().unwrap_or_else(|| previous.map(env).unwrap_or_default());
    for (k, v) in outgoing {
        if fields::claude_exclusive_env(&k) && out.get(&k) == Some(&v) {
            out.remove(&k);
        }
    }
    for (k, values) in [
        ("CLAUDE_CODE_MAX_CONTEXT_TOKENS", &["262144", "372000", "983616"][..]),
        ("CLAUDE_CODE_AUTO_COMPACT_WINDOW", &["262144", "372000", "1000000"][..]),
        ("CLAUDE_CODE_MAX_OUTPUT_TOKENS", &["131072"][..]),
    ] {
        if out.get(k).is_some_and(|v| {
            values.iter().any(|s| v == *s || v.as_u64().is_some_and(|n| n.to_string() == *s))
        }) {
            out.remove(k);
        }
    }
    for (k, v) in env(row) {
        if fields::claude_floor_env(&k) || fields::claude_exclusive_env(&k) {
            out.insert(k, v);
        }
    }
    let truthy = out.get("CLAUDE_CODE_USE_BEDROCK").is_some_and(|v| {
        v == true
            || v == 1
            || v.as_str().is_some_and(|s| matches!(s.to_ascii_lowercase().as_str(), "1" | "true"))
    });
    if truthy && let Some(key) = root.remove("apiKey") {
        out.entry("AWS_BEARER_TOKEN_BEDROCK").or_insert(key);
    }
    if !out.is_empty() || live.get("env").is_some() || row.get("env").is_some() {
        root.insert("env".into(), out.into());
    }
    let mut inactive = vec![];
    for (k, v) in object(row) {
        if k != "env"
            && !fields::claude_floor_top(&k)
            && !["api_format", "apiFormat", "openrouter_compat_mode", "openrouterCompatMode"]
                .contains(&k.as_str())
            && shown.get(&k) != Some(&v)
        {
            inactive.push(vec![k]);
        }
    }
    for (k, v) in env(row) {
        if !fields::claude_floor_env(&k)
            && !fields::claude_exclusive_env(&k)
            && shown["env"].get(&k) != Some(&v)
        {
            inactive.push(vec!["env".into(), k]);
        }
    }
    Ok(Projected { before: live, settings: shown, inactive, difference: None })
}

pub fn gemini(
    live_env: Map<String, Value>,
    live: Value,
    row: &Value,
    official: bool,
) -> Result<Projected, Error> {
    let mut out = live_env.clone();
    out.retain(|k, _| !fields::gemini_floor_env(k));
    for (k, v) in env(row) {
        if fields::gemini_floor_env(&k) && v.is_string() {
            out.insert(k, v);
        }
    }
    let mut config = live.clone();
    set(
        &mut config,
        fields::GEMINI_FLOOR_SETTINGS[0],
        Some(json!(if official { "oauth-personal" } else { "gemini-api-key" })),
    )?;
    set(
        &mut config,
        fields::GEMINI_FLOOR_SETTINGS[1],
        row.pointer("/config/model/name").filter(|v| !v.is_null()).cloned(),
    )?;
    let mut inactive = vec![];
    for (k, v) in env(row) {
        if !fields::gemini_floor_env(&k) && v.is_string() && out.get(&k) != Some(&v) {
            inactive.push(vec!["env".into(), k]);
        }
    }
    let strip = |mut v: Value| -> Result<Value, Error> {
        remove_pruning(&mut v, fields::GEMINI_FLOOR_SETTINGS[0]);
        remove_pruning(&mut v, fields::GEMINI_FLOOR_SETTINGS[1]);
        Ok(v)
    };
    let saved = strip(row.get("config").cloned().unwrap_or(json!({})))?;
    let display = strip(config.clone())?;
    for (k, v) in object(&saved) {
        if display.get(&k) != Some(&v) {
            inactive.push(vec!["config".into(), k]);
        }
    }
    let mut shown = object(row);
    shown.insert("env".into(), out.into());
    shown.insert("config".into(), config);
    Ok(Projected {
        before: json!({"env":live_env,"config":live}),
        settings: shown.into(),
        inactive,
        difference: None,
    })
}
fn remove_pruning(v: &mut Value, path: &[&str]) -> bool {
    let Some((key, rest)) = path.split_first() else {
        return false;
    };
    let Some(map) = v.as_object_mut() else {
        return false;
    };
    if rest.is_empty() {
        if map.remove(*key).is_none() {
            return false;
        }
    } else if let Some(child) = map.get_mut(*key) {
        if remove_pruning(child, rest) {
            map.remove(*key);
        } else {
            return false;
        }
    } else {
        return false;
    }
    map.is_empty()
}
fn set(value: &mut Value, path: &[&str], replacement: Option<Value>) -> Result<(), Error> {
    let (key, rest) = path.split_first().ok_or(Error::Shape)?;
    let map = value.as_object_mut().ok_or(Error::Shape)?;
    if rest.is_empty() {
        match replacement {
            Some(v) => {
                map.insert((*key).into(), v);
            }
            None => {
                map.remove(*key);
            }
        }
        return Ok(());
    }
    if !map.contains_key(*key) && replacement.is_none() {
        return Ok(());
    }
    set(map.entry((*key).to_string()).or_insert(json!({})), rest, replacement)
}
