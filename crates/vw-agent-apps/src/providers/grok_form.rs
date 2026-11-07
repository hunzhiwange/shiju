//! Credential-free Grok model-table view and merge for the desktop editor.
use crate::providers::{basic, projection, validation};
use serde_json::Value;
use toml_edit::{DocumentMut, Item, Table, TableLike, value};
use vw_api_types::{
    agent_apps::AppId,
    app_providers::{create::ProviderSecretWrite, grok::*},
};

type Result<T> = std::result::Result<T, &'static str>;
const INVALID: &str = "invalid or unsupported Grok model table";
const EDITABLE: &[&str] =
    &["model", "name", "base_url", "api_backend", "context_window", "env_key"];

fn parse(text: &str) -> Result<DocumentMut> {
    if text.len() > 1024 * 1024 {
        return Err(INVALID);
    }
    text.parse().map_err(|_| INVALID)
}

fn section<'a>(doc: &'a mut DocumentMut, name: &str) -> Result<&'a mut dyn TableLike> {
    if !doc.contains_key(name) {
        doc.insert(name, Item::Table(Table::new()));
    }
    doc.get_mut(name).and_then(Item::as_table_like_mut).ok_or(INVALID)
}

fn editable(table: &dyn TableLike) -> Result<Table> {
    let mut fields = Table::new();
    for &key in EDITABLE {
        if let Some(item) = table.get(key) {
            let field = if key == "context_window" {
                value(item.as_integer().ok_or(INVALID)?)
            } else {
                let text = item
                    .as_str()
                    .filter(|text| text.len() <= 2048 && !text.chars().any(char::is_control))
                    .ok_or(INVALID)?;
                if key == "base_url" {
                    validation::endpoint(text).map_err(|_| INVALID)?;
                }
                value(text)
            };
            fields.insert(key, field);
        }
    }
    Ok(fields)
}

fn third_party(id: &str, row: &Value) -> Result<DocumentMut> {
    if id == "grokbuild-official"
        || crate::providers::preview::editor_category(AppId::GrokBuild, id, row, None).is_some()
    {
        return Err(INVALID);
    }
    let profile: GrokProfile =
        serde_json::from_value(row.get("grok").cloned().ok_or(INVALID)?).map_err(|_| INVALID)?;
    match profile.config {
        GrokConfig::ThirdParty { toml } => parse(&toml),
        GrokConfig::Official {} => Err(INVALID),
    }
}

fn model<'a>(doc: &'a DocumentMut, id: &str) -> Result<&'a dyn TableLike> {
    doc.get("model").and_then(|m| m.get(id)).and_then(Item::as_table_like).ok_or(INVALID)
}

fn put_model(doc: &mut DocumentMut, id: &str, table: Table) -> Result<()> {
    if id.is_empty() || id.len() > 128 || id.chars().any(char::is_control) {
        return Err(INVALID);
    }
    let tables = section(doc, "model")?;
    if tables.contains_key(id) {
        return Err(INVALID);
    }
    tables.insert(id, Item::Table(table));
    section(doc, "models")?.insert("default", value(id));
    Ok(())
}

pub fn view(id: &str, row: &Value, managed: bool) -> Result<GrokFormView> {
    let fields = basic::read(row).map_err(|_| INVALID)?;
    let config = (!managed).then(|| form_config(id, row)).and_then(Result::ok);
    Ok(GrokFormView { fields, config })
}

fn form_config(id: &str, row: &Value) -> Result<GrokFormConfig> {
    let doc = third_party(id, row)?;
    let id = projection::grok_selected(&doc).ok_or(INVALID)?;
    let table = model(&doc, &id)?;
    let mut shown = DocumentMut::new();
    put_model(&mut shown, &id, editable(table)?)?;
    let projected = projection::grok(
        DocumentMut::new(),
        &serde_json::json!({"config": doc.to_string()}),
        None,
        false,
    );
    let inactive = projected.map(|p| p.inactive).unwrap_or_else(|_| {
        // Repairable drafts still display the saved model fields when projection rejects them.
        Vec::new()
    });
    Ok(GrokFormConfig {
        toml: shown.to_string(),
        has_saved_key: table.contains_key("api_key"),
        inactive,
    })
}

pub fn prepare(id: &str, request: &GrokFormWrite, before: Option<&Value>) -> Result<GrokProfile> {
    if request.fields.category.as_deref().is_some_and(|c| c.trim().eq_ignore_ascii_case("official"))
        || id == "grokbuild-official"
    {
        return Err(INVALID);
    }
    let draft = parse(&request.toml)?;
    let id_new =
        draft.get("models").and_then(|m| m.get("default")).and_then(Item::as_str).ok_or(INVALID)?;
    let source = model(&draft, id_new)?;
    let mut table = editable(source)?;
    if draft.as_table().len() != 2
        || draft["models"].as_table_like().is_none_or(|m| m.len() != 1)
        || draft["model"].as_table_like().is_none_or(|m| m.len() != 1)
        || source.len() != table.len()
    {
        return Err(INVALID);
    }

    let mut merged = DocumentMut::new();
    if let Some(before) = before {
        merged = third_party(id, before)?;
        let old_id = projection::grok_selected(&merged).ok_or(INVALID)?;
        let old = model(&merged, &old_id)?;
        let same_endpoint = old.get("base_url").and_then(Item::as_str)
            == table.get("base_url").and_then(Item::as_str);
        if !same_endpoint
            && old
                .get("env_key")
                .and_then(Item::as_str)
                .is_some_and(|key| table.get("env_key").and_then(Item::as_str) == Some(key))
        {
            return Err(INVALID);
        }
        for (key, item) in old.iter() {
            if EDITABLE.contains(&key)
                || key == "api_key" && !matches!(request.api_key, ProviderSecretWrite::Keep)
            {
                continue;
            }
            // Opaque native options can contain credentials. Never move them to another endpoint.
            if !same_endpoint {
                return Err(INVALID);
            }
            table.insert(key, item.clone());
        }
        section(&mut merged, "model")?.remove(&old_id);
    }
    if let ProviderSecretWrite::Replace { value: secret } = &request.api_key {
        let secret = secret.trim();
        if secret.is_empty()
            || secret.len() > 8192
            || secret.chars().any(char::is_control)
            || secret.eq_ignore_ascii_case("[redacted]")
            || secret.eq_ignore_ascii_case("<redacted>")
            || secret.chars().all(|c| c == '*' || c == '•')
            || secret == "PROXY_MANAGED"
        {
            return Err(INVALID);
        }
        table.insert("api_key", value(secret));
    }
    put_model(&mut merged, id_new, table)?;
    Ok(GrokProfile {
        name: request.fields.name.clone(),
        config: GrokConfig::ThirdParty { toml: merged.to_string() },
    })
}
