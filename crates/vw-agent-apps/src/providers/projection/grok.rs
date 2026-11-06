//! Grok whole-model-table projection, including retired tables and summary following.
use super::{Error, Projected, patch, toml, value};
use crate::providers::live::patch_toml::TomlPatch;
use serde_json::Value;
use toml_edit::{DocumentMut, Item, Table, TableLike};
fn path(p: &[&str]) -> Vec<String> {
    p.iter().map(|s| (*s).into()).collect()
}
fn selected(doc: &DocumentMut) -> Option<String> {
    let tables = doc.get("model")?.as_table_like()?;
    let default =
        doc.get("models").and_then(|p| p.get("default")).and_then(Item::as_str).map(str::trim);
    if let Some(id) = default.filter(|id| tables.get(id).is_some_and(Item::is_table_like)) {
        return Some(id.into());
    }
    let names: Vec<_> = tables.iter().filter(|(_, t)| t.is_table_like()).map(|(n, _)| n).collect();
    match names.as_slice() {
        [one] => Some((*one).into()),
        _ => None,
    }
}
fn valid(table: &dyn TableLike) -> bool {
    let nonempty =
        |k: &str| table.get(k).and_then(Item::as_str).is_some_and(|v| !v.trim().is_empty());
    ["model", "base_url", "name", "api_backend"].into_iter().all(nonempty)
        && (nonempty("api_key") || nonempty("env_key"))
        && table.get("context_window").and_then(Item::as_integer).is_some_and(|v| v > 0)
}
pub fn project(
    live: DocumentMut,
    row: &Value,
    previous: Option<&Value>,
    official: bool,
) -> Result<Projected, Error> {
    let row_doc = toml(row.get("config").and_then(Value::as_str).unwrap_or(""), "draft.config")?;
    let before = value(&live)?;
    let target = if official { None } else { selected(&row_doc) };
    // Source displays invalid/ambiguous stored rows for repair instead of fabricating a live projection.
    let invalid = !official
        && target.as_ref().is_none_or(|id| {
            !row_doc
                .get("model")
                .and_then(|p| p.get(id))
                .and_then(Item::as_table_like)
                .is_some_and(valid)
        });
    if invalid {
        let mut shown = row.clone();
        shown["config"] = value(&row_doc)?;
        return Err(Error::Draft(shown));
    }
    if let Some(table) = target.as_ref().and_then(|id| row_doc.get("model").and_then(|m| m.get(id)))
    {
        crate::providers::validation::native_request(
            table.get("base_url"),
            table.get("extra_headers"),
        )
        .map_err(|_| Error::Shape)?;
    }
    let mut retired = vec![];
    if let Some(previous) = previous {
        let doc =
            toml(previous.get("config").and_then(Value::as_str).unwrap_or(""), "owner.config")?;
        if let Some(id) = selected(&doc) {
            retired.push(id);
        }
    }
    if let Some(tables) = before.get("model").and_then(Value::as_object) {
        for (id, t) in tables {
            if t["api_key"] == "PROXY_MANAGED" {
                retired.push(id.clone());
            }
        }
    }
    let mut edits = TomlPatch::default();
    for id in &retired {
        if Some(id) != target.as_ref() {
            edits.remove.push(path(&["model", id]));
        }
    }
    match &target {
        Some(id) => {
            edits.set.push((path(&["models", "default"]), toml_edit::value(id.clone())));
            // Copy every key, including env_key, headers and vendor extension options.
            let source = row_doc
                .get("model")
                .and_then(|p| p.get(id))
                .and_then(Item::as_table_like)
                .ok_or(Error::Shape)?;
            let mut table = Table::new();
            for (k, v) in source.iter() {
                table.insert(k, v.clone());
            }
            edits.set.push((path(&["model", id]), Item::Table(table)));
        }
        None => edits.remove.push(path(&["models", "default"])),
    }
    let summary = before.pointer("/models/session_summary").and_then(Value::as_str);
    if summary
        .is_none_or(|name| Some(name) == target.as_deref() || retired.iter().any(|r| r == name))
    {
        match &target {
            Some(id) => {
                edits.set.push((path(&["models", "session_summary"]), toml_edit::value(id.clone())))
            }
            None => edits.remove.push(path(&["models", "session_summary"])),
        }
    }
    let mut doc = patch(&live, edits)?;
    for k in ["models", "model"] {
        if doc.get(k).and_then(Item::as_table_like).is_some_and(|t| t.is_empty()) {
            doc.remove(k);
        }
    }
    let shown = value(&doc)?;
    let inactive = super::inactive(&value(&row_doc)?, &shown, |p| {
        p.len() == 3
            && (p[1] == "models" && p[2] == "default"
                || p[1] == "model" && Some(p[2].as_str()) == target.as_deref())
    });
    let mut settings = row.as_object().cloned().ok_or(Error::Shape)?;
    settings.insert("config".into(), shown);
    Ok(Projected {
        before: serde_json::json!({"config":before}),
        settings: settings.into(),
        inactive,
        difference: None,
    })
}
