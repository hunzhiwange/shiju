//! Read-only execution-host editor projection. Secrets and unredacted baselines stay in this call.
use super::projection::{self, Error, Projected};
use serde_json::{Map, Value, json};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};
use vw_api_types::{
    agent_apps::AppId,
    app_providers::{paths::ConfigPathState, preview::*},
};
/// Resolved from the executor's transaction state and tenant store, never from the request.
#[derive(Default)]
pub struct PreviewOwner {
    pub settings: Option<Value>,
    pub exclusive: Option<Map<String, Value>>,
    pub unavailable: Option<PreviewUnavailable>,
}
pub fn read(
    home: &Path,
    app: AppId,
    request: &ProviderPreviewRequest,
    owner: &PreviewOwner,
) -> Result<ProviderPreview, &'static str> {
    let mut response = ProviderPreview {
        settings: Value::Null,
        changes: vec![],
        inactive: vec![],
        unavailable: None,
        parse_error: None,
    };
    if let Some(reason) = &owner.unavailable {
        response.unavailable = Some(reason.clone());
        return Ok(response);
    }
    if !matches!(app, AppId::Claude | AppId::Codex | AppId::Gemini | AppId::GrokBuild) {
        response.unavailable = Some(PreviewUnavailable::UnsupportedApplication);
        return Ok(response);
    }
    if !request.settings.is_object()
        || serde_json::to_vec(&request.settings).map_err(|_| "invalid draft")?.len() > 128 * 1024
    {
        return Err("invalid preview draft");
    }
    let paths =
        super::paths::read(home, app, request.directory_override.as_deref(), &BTreeMap::new())?;
    if paths.files.iter().any(|f| f.state == ConfigPathState::Inaccessible) {
        return Err("configuration unavailable");
    }
    let load = |index: usize| -> Result<Option<Vec<u8>>, &'static str> {
        let path = PathBuf::from(&paths.files[index].path);
        let relative = path.strip_prefix(home).map_err(|_| "unauthorized configuration")?;
        let components: Vec<_> = relative
            .iter()
            .map(|p| p.to_str().ok_or("invalid configuration path"))
            .collect::<Result<_, _>>()?;
        super::catalog::read_file(home, &components).map_err(|_| "configuration unavailable")
    };
    let official = request.category.as_deref() == Some("official");
    let settings = &request.settings;
    let result = (|| -> Result<Projected, Error> {
        match app {
            AppId::Claude => projection::claude(
                parse_json(load(0).map_err(|_| Error::Shape)?.as_deref(), "live.settings")?,
                settings,
                owner.settings.as_ref(),
                owner.exclusive.as_ref(),
            ),
            AppId::Gemini => {
                let bytes = load(0).map_err(|_| Error::Shape)?;
                let text = std::str::from_utf8(bytes.as_deref().unwrap_or_default())
                    .map_err(|_| Error::Shape)?;
                // Share the existing dotenv parser (last duplicate wins; no expansion).
                let env = super::live::patch_dotenv::entries(text)
                    .into_iter()
                    .map(|(k, v)| (k, Value::String(v)))
                    .collect();
                projection::gemini(
                    env,
                    parse_json(load(1).map_err(|_| Error::Shape)?.as_deref(), "live.settings")?,
                    settings,
                    official,
                )
            }
            AppId::Codex | AppId::GrokBuild => {
                let bytes = load(0).map_err(|_| Error::Shape)?;
                let text = std::str::from_utf8(bytes.as_deref().unwrap_or_default())
                    .map_err(|_| Error::Shape)?;
                let doc = projection::toml(text, "live.config")?;
                if app == AppId::Codex {
                    if doc
                        .get("cli_auth_credentials_store")
                        .and_then(toml_edit::Item::as_str)
                        .is_some_and(|s| s != "file")
                    {
                        return Err(Error::Unavailable(PreviewUnavailable::OfficialLogin));
                    }
                    if settings
                        .pointer("/modelCatalog/models")
                        .and_then(Value::as_array)
                        .is_some_and(|models| {
                            models.iter().any(|m| {
                                m.get("model")
                                    .and_then(Value::as_str)
                                    .is_some_and(|s| !s.trim().is_empty())
                            })
                        })
                    {
                        return Err(Error::Unavailable(PreviewUnavailable::ModelCatalog));
                    }

                    if request
                        .meta
                        .as_ref()
                        .or_else(|| settings.get("meta"))
                        .is_some_and(|v| v.as_object().is_none_or(|m| !m.is_empty()))
                    {
                        return Err(Error::Unavailable(PreviewUnavailable::ManagedProvider));
                    }
                    let auth =
                        parse_json(load(1).map_err(|_| Error::Shape)?.as_deref(), "live.auth")?;
                    // The native preservation/account policy is not a preview-owned setting.
                    // Do not guess whether an existing login would survive a later switch.
                    if login_material(&auth) {
                        return Err(Error::Unavailable(PreviewUnavailable::OfficialLogin));
                    }
                    let login = false;
                    projection::codex(
                        doc,
                        settings,
                        owner.settings.as_ref(),
                        owner.exclusive.as_ref(),
                        login,
                        official,
                    )
                } else {
                    projection::grok(doc, settings, owner.settings.as_ref(), official)
                }
            }
            _ => unreachable!(),
        }
    })();
    match result {
        Ok(projected) => {
            let (before, after) = projected
                .difference
                .as_ref()
                .map(|(a, b)| (a, b))
                .unwrap_or((&projected.before, &projected.settings));
            diff(before, after, &mut vec![], &mut response.changes);
            response.settings = redact(projected.settings);
            response.inactive = projected.inactive;
        }
        Err(Error::Parse(location)) => response.parse_error = Some(location),
        Err(Error::Unavailable(reason)) => response.unavailable = Some(reason),
        Err(Error::Draft(draft)) => {
            response.settings = redact(draft);
            response.unavailable = Some(PreviewUnavailable::InvalidDraft);
        }
        Err(Error::Shape | Error::Keyless) => {
            return Err("invalid or unavailable editor projection");
        }
    }
    Ok(response)
}
fn parse_json(bytes: Option<&[u8]>, source: &str) -> Result<Value, Error> {
    let value: Value = serde_json::from_slice(bytes.unwrap_or(b"{}")).map_err(|e| {
        Error::Parse(PreviewParseError {
            source: source.into(),
            line: e.line(),
            column: e.column(),
        })
    })?;
    if !value.is_object() {
        return Err(Error::Shape);
    }
    Ok(value)
}
fn redact(value: Value) -> Value {
    match value {
        Value::String(_) => json!("[redacted]"),
        Value::Array(values) => values.into_iter().map(redact).collect(),
        Value::Object(values) => values
            .into_iter()
            .map(|(k, v)| {
                let name = k.to_ascii_lowercase();
                let sensitive = ["key", "token", "secret", "password", "credential"]
                    .iter()
                    .any(|word| name.contains(word));
                (k, if sensitive { json!("[redacted]") } else { redact(v) })
            })
            .collect(),
        other => other,
    }
}
fn diff(before: &Value, after: &Value, path: &mut Vec<String>, result: &mut Vec<PreviewChange>) {
    if let (Some(old), Some(new)) = (before.as_object(), after.as_object()) {
        for key in old.keys().chain(new.keys().filter(|k| !old.contains_key(*k))) {
            path.push(key.clone());
            match (old.get(key), new.get(key)) {
                (Some(old), Some(new)) => diff(old, new, path, result),
                (Some(_), None) => result
                    .push(PreviewChange { path: path.clone(), kind: PreviewChangeKind::Removed }),
                (None, Some(_)) => result
                    .push(PreviewChange { path: path.clone(), kind: PreviewChangeKind::Added }),
                _ => {}
            }
            path.pop();
        }
    } else if before != after {
        result.push(PreviewChange { path: path.clone(), kind: PreviewChangeKind::Changed });
    }
}

fn login_material(auth: &Value) -> bool {
    let present = |v: &Value| match v {
        Value::Null => false,
        Value::String(s) => !s.trim().is_empty(),
        Value::Array(a) => !a.is_empty(),
        Value::Object(m) => !m.is_empty(),
        _ => true,
    };
    ["personal_access_token", "agent_identity", "bedrock_api_key"]
        .iter()
        .any(|k| auth.get(k).is_some_and(present))
        || auth.get("tokens").is_some_and(|tokens| {
            ["id_token", "access_token", "refresh_token"]
                .iter()
                .any(|k| tokens.get(k).is_some_and(present))
        })
}

/// Existing cards use the source classification, independent of client category labels.
pub fn editor_category(
    app: AppId,
    id: &str,
    record: &Value,
    requested: Option<&str>,
) -> Option<String> {
    let official = record.get("category").and_then(Value::as_str) == Some("official")
        || match app {
            AppId::Gemini => {
                let partner = record
                    .pointer("/meta/partnerPromotionKey")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                let name = record
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_ascii_lowercase();
                partner.eq_ignore_ascii_case("google-official")
                    || !partner.eq_ignore_ascii_case("packycode")
                        && (name == "google" || name.starts_with("google "))
            }
            AppId::Codex => {
                let settings = &record["settingsConfig"];
                let doc = settings
                    .get("config")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .parse::<toml_edit::DocumentMut>()
                    .ok();
                let upstream = doc.as_ref().is_some_and(|d| {
                    d.get("openai_base_url").is_some()
                        || d.get("base_url").is_some()
                        || d.get("model_provider")
                            .and_then(toml_edit::Item::as_str)
                            .is_some_and(|id| id != "openai")
                });
                id == "codex-official"
                    && settings.get("auth").is_some_and(Value::is_object)
                    && !upstream
                    && !settings
                        .pointer("/auth/OPENAI_API_KEY")
                        .and_then(Value::as_str)
                        .is_some_and(|key| !key.trim().is_empty())
            }
            _ => false,
        };
    if official { Some("official".into()) } else { requested.map(str::to_owned) }
}
