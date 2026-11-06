//! Shared Codex TOML projection, with a separate editor display step. No file publication.
use super::codex_route::{CodexProjection, Route, RowInput, requires_openai_auth};
use super::{Error, Projected, fields, patch, toml, value};
use crate::providers::live::patch_toml::TomlPatch;
use serde_json::{Map, Value};
use toml_edit::{DocumentMut, Item};
use vw_api_types::app_providers::preview::PreviewUnavailable;
fn path(parts: &[&str]) -> Vec<String> {
    parts.iter().map(|s| (*s).into()).collect()
}
fn parse(row: &Value, official: bool) -> Result<CodexProjection, Error> {
    match CodexProjection::of(&RowInput { settings: row, official, proxy_injected_oauth: false }) {
        Err(Error::Keyless) => {
            let mut draft = row.clone();
            draft["auth"] = serde_json::json!({"OPENAI_API_KEY":"CC_SWITCH_PENDING_KEY"});
            CodexProjection::of(&RowInput {
                settings: &draft,
                official,
                proxy_injected_oauth: false,
            })
        }
        other => other,
    }
}
pub fn project(
    live: DocumentMut,
    row: &Value,
    previous: Option<&Value>,
    exclusive: Option<&Map<String, Value>>,
    login: bool,
    official: bool,
) -> Result<Projected, Error> {
    // Predicting official login and generated catalog requires the account/catalog owners.
    if official {
        return Err(Error::Unavailable(PreviewUnavailable::OfficialLogin));
    }
    let text = row.get("config").and_then(Value::as_str).unwrap_or("");
    let row_doc = toml(text, "draft.config")?;
    let projection = parse(row, false)?;
    let before = value(&live)?;
    let selector = row_doc.get("model_provider").and_then(Item::as_str);
    let mut doc = apply(live, previous, exclusive, login, projection)?;
    let projected_config = value(&doc)?;
    if let Some(table) = doc
        .get_mut("model_providers")
        .and_then(|p| p.get_mut("custom"))
        .and_then(Item::as_table_like_mut)
    {
        table.remove("experimental_bearer_token");
    }
    let shown = value(&doc)?;
    let inactive = super::inactive(&value(&row_doc)?, &shown, |p| {
        if p.len() == 2 {
            fields::CODEX_FLOOR_TOP.contains(&p[1].as_str())
                || fields::CODEX_EXCLUSIVE_TOP.contains(&p[1].as_str())
        } else {
            p[1] == "model_providers"
                && (p[2] == "custom"
                    || p[2] == "cc-switch-official"
                    || Some(p[2].as_str()) == selector)
                || fields::CODEX_FLOOR_NESTED
                    .iter()
                    .any(|k| p[1..].iter().map(String::as_str).eq(k.iter().copied()))
        }
    });
    let mut settings = row.as_object().cloned().ok_or(Error::Shape)?;
    settings.insert("config".into(), shown);
    Ok(Projected {
        before: serde_json::json!({"config":before,"auth":row.get("auth").cloned().unwrap_or_default()}),
        settings: settings.into(),
        difference: Some((
            serde_json::json!({"config":before}),
            serde_json::json!({"config":projected_config}),
        )),
        inactive,
    })
}
fn value_from(v: &toml_edit::Value) -> Option<Value> {
    let mut doc = DocumentMut::new();
    doc.insert("v", Item::Value(v.clone()));
    value(&doc).ok()?.get("v").cloned()
}
fn profile_references(before: &Value, id: &str) -> bool {
    before
        .get("profiles")
        .and_then(Value::as_object)
        .is_some_and(|p| p.values().any(|v| v["model_provider"] == id))
}

/// Strict native path: no draft placeholder or display credential removal.
pub(crate) fn native(
    live: DocumentMut,
    row: &Value,
    previous: Option<&Value>,
    login: bool,
    exclusive: Option<&Map<String, Value>>,
) -> Result<DocumentMut, Error> {
    let projection = CodexProjection::of(&RowInput {
        settings: row,
        official: false,
        proxy_injected_oauth: false,
    })?;
    apply(live, previous, exclusive, login, projection)
}

fn apply(
    mut live: DocumentMut,
    previous: Option<&Value>,
    exclusive: Option<&Map<String, Value>>,
    login: bool,
    projection: CodexProjection,
) -> Result<DocumentMut, Error> {
    match &projection.route {
        Route::Custom { table, .. } | Route::BuiltIn { table: Some(table), .. } => {
            crate::providers::validation::native_request(
                table.get("base_url"),
                table.get("http_headers"),
            )
            .map_err(|_| Error::Shape)?;
        }
        _ => {}
    }
    let before = value(&live)?;
    let mut edits = TomlPatch::default();
    for key in fields::CODEX_FLOOR_TOP {
        edits.remove.push(path(&[key]));
    }
    for p in fields::CODEX_FLOOR_NESTED {
        edits.remove.push(path(p));
    }
    let mut outgoing = vec![];
    if let Some(previous) = previous {
        let doc = toml(
            previous.get("config").and_then(Value::as_str).unwrap_or_default(),
            "owner.config",
        )?;
        for key in fields::CODEX_EXCLUSIVE_TOP {
            if let Some(value) = doc.get(key).and_then(Item::as_value) {
                outgoing.push(((*key).to_string(), value.clone()));
            }
        }
    }
    if let Some(exclusive) = exclusive {
        outgoing = exclusive
            .iter()
            .filter_map(|(key, v)| {
                let encoded = ::toml::to_string(&serde_json::json!({"v":v})).ok()?;
                Some((key.clone(), toml(&encoded, "owner").ok()?.get("v")?.as_value()?.clone()))
            })
            .collect();
    }
    for (key, v) in outgoing {
        if before.get(&key) == value_from(&v).as_ref() {
            edits.remove.push(path(&[&key]));
        }
    }
    for (key, v) in projection.top.iter().chain(&projection.exclusive) {
        edits.set.push((path(&[key]), Item::Value(v.clone())));
    }
    for (p, v) in &projection.nested {
        edits.set.push((p.clone(), Item::Value(v.clone())));
    }
    let mut targets = vec!["custom".to_string(), "cc-switch-official".to_string()];
    if let Some(previous) = previous {
        let previous =
            toml(previous.get("config").and_then(Value::as_str).unwrap_or(""), "owner.config")?;
        if let Some(id) = previous.get("model_provider").and_then(Item::as_str)
            && let Some(url) = previous
                .get("model_providers")
                .and_then(|p| p.get(id))
                .and_then(|p| p.get("base_url"))
                .and_then(Item::as_str)
            && before
                .get("model_providers")
                .and_then(|p| p.get(id))
                .and_then(|p| p.get("base_url"))
                .and_then(Value::as_str)
                == Some(url)
        {
            targets.push(id.into());
        }
    }
    if let Some(providers) = before.get("model_providers").and_then(Value::as_object) {
        for (id, table) in providers {
            if table["experimental_bearer_token"] == "PROXY_MANAGED" {
                targets.push(id.clone());
            }
        }
    }
    // Reserved provider tables must be renamed; unproven user tables/profile references survive.
    if let Some(container) = live.get_mut("model_providers") {
        let providers = container.as_table_like_mut().ok_or(Error::Shape)?;
        for key in ["openai", "ollama", "lmstudio"] {
            if let Some(item) = providers.remove(key) {
                if targets.iter().any(|id| id == key)
                    || item.get("experimental_bearer_token").and_then(Item::as_str)
                        == Some("PROXY_MANAGED")
                {
                    continue;
                }
                let mut name = "cc-switch".to_string();
                let mut n = 2;
                while providers.contains_key(&name) {
                    name = format!("cc-switch-{n}");
                    n += 1;
                }
                providers.insert(&name, item);
            }
        }
    }
    for id in targets {
        if id != "custom" && !profile_references(&before, &id) {
            edits.remove.push(path(&["model_providers", &id]));
        }
    }
    match projection.route {
        Route::Custom { mut table, auth } => {
            table.insert(
                "requires_openai_auth",
                toml_edit::value(requires_openai_auth(auth, login)),
            );
            edits.set.push((path(&["model_provider"]), toml_edit::value("custom")));
            edits.set.push((path(&["model_providers", "custom"]), Item::Table(table)));
        }
        Route::BuiltIn { id, table } => {
            edits.set.push((path(&["model_provider"]), toml_edit::value(id.clone())));
            if let Some(table) = table {
                edits.set.push((path(&["model_providers", &id]), Item::Table(table)));
            }
            edits.remove.push(path(&["model_providers", "custom", "experimental_bearer_token"]));
        }
        Route::Default => {
            edits.remove.push(path(&["model_providers", "custom", "experimental_bearer_token"]));
        }
        Route::Official => return Err(Error::Unavailable(PreviewUnavailable::OfficialLogin)),
    }
    let doc = patch(&live, edits)?;
    let output = value(&doc)?;
    if let Some(profile) = output
        .get("profile")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .and_then(|name| output.get("profiles").and_then(|v| v.get(name)))
    {
        // With no top-level selector, Codex uses built-in openai. A profile that
        // explicitly selects the same route does not redirect requests.
        let selector = output.get("model_provider").and_then(Value::as_str).unwrap_or("openai");
        let overridden = ["model_provider", "openai_base_url", "experimental_bearer_token"]
            .into_iter()
            .any(|key| {
                let value = profile
                    .get(key)
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|text| !text.is_empty());
                match (key, value) {
                    (_, None) => false,
                    ("model_provider", Some(id)) => id != selector,
                    _ => true,
                }
            });
        if overridden {
            return Err(Error::Unavailable(PreviewUnavailable::Ownership));
        }
    }
    Ok(doc)
}
