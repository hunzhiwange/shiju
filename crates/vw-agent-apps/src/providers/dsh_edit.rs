//! Native member decoding and removal, restricted to the shared Dsh profile set.
use super::*;

pub fn member(pre: &Snapshot, id: &str) -> Result<Option<DshProfile>, WriteError> {
    let mut found = None;
    for (key, bytes) in pre {
        if matches!(key.as_str(), "credentials" | "home") {
            continue;
        }
        if let Some(node) = providers(bytes.as_deref())?.get(id) {
            if found.as_ref().is_some_and(|old| old != node) {
                return Err(WriteError::Conflict);
            }
            found = Some(node.clone());
        }
    }
    let Some(node) = found else {
        return Ok(None);
    };
    let original: Value = serde_json::to_value(node).map_err(|_| WriteError::InvalidResource)?;
    let mut declaration = original.clone();
    let obj = declaration.as_object_mut().ok_or(WriteError::InvalidResource)?;
    let name = obj
        .remove("displayName")
        .and_then(|v| v.as_str().map(str::to_owned))
        .ok_or(WriteError::InvalidResource)?;
    let mut options = json!({"baseURL": obj.remove("baseURL").ok_or(WriteError::InvalidResource)?});
    if let Some(headers) = obj.remove("headers") {
        options["headers"] = headers;
    }
    obj.insert("options".into(), options);
    let models = obj.remove("models").ok_or(WriteError::InvalidResource)?;
    let mut mapped = serde_json::Map::new();
    for model in models.as_array().ok_or(WriteError::InvalidResource)? {
        let mut model = model.clone();
        let fields = model.as_object_mut().ok_or(WriteError::InvalidResource)?;
        let key = fields
            .remove("id")
            .and_then(|v| v.as_str().map(str::to_owned))
            .ok_or(WriteError::InvalidResource)?;
        for (native, catalog) in [("contextWindow", "context"), ("maxTokens", "output")] {
            if let Some(value) = fields.remove(native) {
                fields
                    .entry("limit")
                    .or_insert_with(|| json!({}))
                    .as_object_mut()
                    .ok_or(WriteError::InvalidResource)?
                    .insert(catalog.into(), value);
            }
        }
        if mapped.insert(key, model).is_some() {
            return invalid();
        }
    }
    obj.insert("models".into(), Value::Object(mapped));
    let profile = DshProfile { name, declaration };
    validate(id, &profile)?;
    // Unsupported native layouts must never be silently normalized during editing.
    if native(id, &profile) != original || !credential_binding_valid(pre, &profile) {
        return invalid();
    }
    Ok(Some(profile))
}

pub fn remove(pre: &Snapshot, id: &str) -> Result<Snapshot, WriteError> {
    let Some(_) = member(pre, id)? else {
        return Ok(pre.clone());
    };
    if entries(pre.get("home").and_then(|v| v.as_deref()))?
        .iter()
        .any(|v| v["id"].as_str() == Some("llm-pi-ai") && !v["config"].is_null())
    {
        return invalid();
    }
    let mut next = pre.clone();
    for (key, bytes) in pre {
        if matches!(key.as_str(), "credentials" | "home") {
            continue;
        }
        let mut parsed = entries(bytes.as_deref())?;
        for plugin in &mut parsed {
            if plugin["id"].as_str() == Some("llm-pi-ai")
                && let Some(nodes) = plugin
                    .get_mut("config")
                    .and_then(|v| v.get_mut("providers"))
                    .and_then(Yaml::as_mapping_mut)
            {
                nodes.remove(Yaml::from(id));
                if nodes.values().any(|node| referenced(node, id, false)) {
                    return invalid();
                }
                // Model IDs in sibling declarations are local, not provider references.
                nodes.clear();
            }
        }
        // Keep default selections and plugin references intact: reject a referenced member.
        if referenced(
            &serde_yaml::to_value(&parsed).map_err(|_| WriteError::InvalidResource)?,
            id,
            true,
        ) {
            return invalid();
        }
        if providers(bytes.as_deref())?.get(id).is_some() {
            next.insert(key.clone(), Some(patch_profile(bytes.as_deref(), id, None)?));
        }
    }
    // Shared refs and account/device records are native-owned, including unused refs.
    Ok(next)
}
fn referenced(value: &Yaml, id: &str, exact: bool) -> bool {
    match value {
        Yaml::String(s) => {
            (exact && s == id)
                || s.starts_with(&format!("{id}/"))
                || s.starts_with(&format!("{id}:"))
        }
        Yaml::Sequence(values) => values.iter().any(|v| referenced(v, id, exact)),
        Yaml::Mapping(values) => {
            values.iter().any(|(k, v)| referenced(k, id, exact) || referenced(v, id, exact))
        }
        Yaml::Tagged(_) => true,
        _ => false,
    }
}
