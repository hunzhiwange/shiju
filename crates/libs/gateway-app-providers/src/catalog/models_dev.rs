//! App-specific projections of the gateway's public model catalog. No connection or credential reads.
use axum::{Extension, Json, extract::Path};
use model_protocols::{ModelProtocol, ProtocolAuthentication, ProtocolEndpoint};
use provider_model_cache::Provider;
use std::collections::BTreeMap;
use vw_api_app_identity::AppId;
use vw_api_app_provider_edit::catalog::{ModelDevPresetModel, ModelDevProviderPreset};
use vw_api_transport::id::TenantScope;

pub(super) async fn list(
    Extension(_tenant): Extension<TenantScope>,
    Path(app): Path<AppId>,
) -> Json<Vec<ModelDevProviderPreset>> {
    let snapshot = provider_model_cache::snapshot().await;
    Json(project(app, snapshot.providers.values()))
}

fn project<'a>(
    app: AppId,
    providers: impl IntoIterator<Item = &'a Provider>,
) -> Vec<ModelDevProviderPreset> {
    let mut rows = BTreeMap::new();
    for provider in providers {
        for model in provider.models.values() {
            if model.id.is_empty()
                || model.id.len() > 256
                || model.id.chars().any(char::is_control)
                || model.status.as_deref() == Some("deprecated")
            {
                continue;
            }
            let context_window = (model.limit.context > 0
                && i64::try_from(model.limit.context).is_ok())
            .then_some(model.limit.context);
            if app == AppId::GrokBuild && context_window.is_none() {
                continue;
            }
            for fact in model
                .protocols
                .as_ref()
                .or_else(|| model.provider.as_ref()?.protocols.as_ref())
                .or(provider.protocols.as_ref())
                .into_iter()
                .flatten()
                .filter(|fact| fact.models.contains(&model.id))
            {
                let Some((base_url, api_key_header)) = native_endpoint(app, fact, &model.id) else {
                    continue;
                };
                let row = rows
                    .entry((
                        provider.id.as_str(),
                        fact.deployment.as_str(),
                        base_url,
                        api_key_header,
                    ))
                    .or_insert_with(|| ModelDevProviderPreset {
                        provider_id: provider.id.clone(),
                        name: if provider.name.is_empty() {
                            provider.id.clone()
                        } else {
                            provider.name.clone()
                        },
                        deployment: fact.deployment.clone(),
                        base_url: base_url.into(),
                        api_key_header,
                        models: Vec::new(),
                    });
                row.models.push(ModelDevPresetModel {
                    id: model.id.clone(),
                    name: if model.name.is_empty() { model.id.clone() } else { model.name.clone() },
                    context_window,
                });
            }
        }
    }
    let mut rows: Vec<_> = rows.into_values().collect();
    for row in &mut rows {
        row.models.sort_by(|left, right| left.id.cmp(&right.id));
        row.models.dedup_by(|left, right| left.id == right.id);
    }
    rows.sort_by_cached_key(|row| row.name.to_lowercase());
    rows
}

fn native_endpoint<'a>(
    app: AppId,
    fact: &'a ProtocolEndpoint,
    model_id: &str,
) -> Option<(&'a str, bool)> {
    use ModelProtocol::*;
    use ProtocolAuthentication::{ApiKeyHeader, Bearer};

    let (base, api_key_header) = match (app, fact.protocol, &fact.authentication) {
        (AppId::Claude, Messages, Bearer {}) => (fact.url.strip_suffix("/v1/messages")?, false),
        (AppId::Claude, Messages, ApiKeyHeader { name })
            if name.eq_ignore_ascii_case("x-api-key") =>
        {
            (fact.url.strip_suffix("/v1/messages")?, true)
        }
        (AppId::ClaudeDesktop, Messages, Bearer {}) => {
            // The direct Desktop form has no model aliasing or custom auth header.
            let id = model_id.strip_prefix("anthropic/").unwrap_or(model_id);
            if !["claude-sonnet-", "claude-opus-", "claude-fable-", "claude-haiku-"]
                .iter()
                .any(|prefix| id.starts_with(*prefix))
            {
                return None;
            }
            (fact.url.strip_suffix("/v1/messages")?, false)
        }
        (AppId::Codex | AppId::GrokBuild, Responses, Bearer {}) => {
            (fact.url.strip_suffix("/responses")?, false)
        }
        (AppId::Gemini, Gemini, ApiKeyHeader { name })
            if name.eq_ignore_ascii_case("x-goog-api-key") =>
        {
            let (base, operation) = fact.url.rsplit_once("/v1beta/models/")?;
            if operation.strip_suffix(":generateContent") != Some(model_id) {
                return None;
            }
            (base, true)
        }
        _ => return None,
    };
    if base.contains(['{', '}', '<', '>'])
        || base.to_ascii_lowercase().contains("your_")
        || !base.starts_with("https://")
        || app_provider_validation::validation::endpoint(base).is_err()
    {
        return None;
    }
    Some((base, api_key_header))
}

#[cfg(test)]
#[path = "models_dev_tests.rs"]
mod tests;
