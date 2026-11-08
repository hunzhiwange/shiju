//! Convert authorized, saved provider records into fixed native discovery inputs.
use crate::{app_providers::AppProviderStore, global_database::GlobalDatabase};
use axum::http::StatusCode;
use serde_json::Value;
use std::collections::BTreeMap;
use tokio_util::sync::CancellationToken;
use vw_agent_apps::{
    apps::discovery::read::LocalReadEnvironment,
    models::{
        discovery_agy,
        discovery_http::{self, HttpDiscoveryRequest},
        discovery_opencode, edits,
    },
    providers::{self, codex_catalog},
};
use vw_api_types::{
    agent_apps::AppId,
    app_providers::{
        ProviderProtocol,
        agy::AgyProfile,
        claude::{ClaudeConfig, ClaudeProfile},
        endpoints::AppProviderEndpointKind,
        gemini::{GeminiConfig, GeminiProfile},
    },
    id::{ModelId, TenantScope},
    model_catalog::{
        ModelCatalogEntry, ModelCatalogIdentity, ModelDiscoveryFailure as Failure,
        ModelDiscoveryMethod, ModelDiscoveryRequest, ScopedModelCatalogIdentity,
    },
};

pub(super) enum Work {
    Http(Http),
    OpenCode(LocalReadEnvironment),
    Agy(LocalReadEnvironment, AgyProfile),
}

pub(super) struct Http {
    base_url: String,
    endpoint_kind: AppProviderEndpointKind,
    protocol: ProviderProtocol,
    api_key: String,
    models_url: Option<String>,
    user_agent: Option<String>,
    headers: BTreeMap<String, String>,
}

pub(super) fn prepare(
    database: &GlobalDatabase,
    tenant: &TenantScope,
    host: LocalReadEnvironment,
    input: &ModelDiscoveryRequest,
) -> Result<Work, StatusCode> {
    let deployment = &input.deployment;
    let app = deployment.app_id;
    // The native CLI uses the authorized host login or a saved key. It cannot
    // select another login by ID without changing the host's live configuration.
    if input.account_id.is_some()
        || matches!(input.method, ModelDiscoveryMethod::OpenCode) && app != AppId::OpenCode
        || matches!(input.method, ModelDiscoveryMethod::Agy) && app != AppId::Agy
    {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }
    if input.method == ModelDiscoveryMethod::Agy {
        let profile = AppProviderStore::new(database.clone())
            .and_then(|store| store.agy_profile(tenant, &deployment.provider_id))
            .map_err(record_error)?;
        if let Some(endpoint) = &profile.base_url {
            require_endpoint(deployment, endpoint)?;
        }
        return Ok(Work::Agy(host, profile));
    }
    let row = AppProviderStore::preview_record(database, tenant, app, &deployment.provider_id)
        .map_err(record_error)?;
    if input.method == ModelDiscoveryMethod::OpenCode {
        if let Some(endpoint) =
            row.pointer("/opencode/declaration/options/baseURL").and_then(Value::as_str)
        {
            require_endpoint(deployment, endpoint)?;
        }
        return Ok(Work::OpenCode(host));
    }

    if row.pointer("/meta/authBinding").is_some_and(|value| !value.is_null())
        || row.pointer("/meta/liveConfigManaged").is_some_and(|value| value == true)
    {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }
    let (base_url, api_key, protocol) = http_credentials(deployment, &row)?;
    require_endpoint(deployment, &base_url)?;
    let options = row
        .get("requestOptions")
        .map(|value| serde_json::from_value(value.clone()))
        .transpose()
        .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?
        .unwrap_or_default();
    providers::validation::validate(app, &options).map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
    Ok(Work::Http(Http {
        base_url,
        endpoint_kind: if row.pointer("/meta/isFullUrl").is_some_and(|value| value == true) {
            AppProviderEndpointKind::FullUrl
        } else {
            AppProviderEndpointKind::BaseUrl
        },
        protocol,
        api_key,
        models_url: row.pointer("/meta/modelsUrl").and_then(Value::as_str).map(str::to_owned),
        user_agent: row.pointer("/meta/customUserAgent").and_then(Value::as_str).map(str::to_owned),
        headers: options.request_headers.unwrap_or_default(),
    }))
}

fn record_error(error: String) -> StatusCode {
    match error.as_str() {
        "application provider not found" => StatusCode::NOT_FOUND,
        "application provider storage unavailable" | "provider storage unavailable" => {
            StatusCode::SERVICE_UNAVAILABLE
        }
        _ => StatusCode::UNPROCESSABLE_ENTITY,
    }
}

fn require_endpoint(deployment: &ModelCatalogIdentity, endpoint: &str) -> Result<(), StatusCode> {
    if deployment.endpoint == endpoint { Ok(()) } else { Err(StatusCode::FORBIDDEN) }
}

fn http_credentials(
    deployment: &ModelCatalogIdentity,
    row: &Value,
) -> Result<(String, String, ProviderProtocol), StatusCode> {
    use ProviderProtocol::{Anthropic, Gemini, OpenaiResponses};
    let unsupported = StatusCode::UNPROCESSABLE_ENTITY;
    match deployment.app_id {
        AppId::Codex if row.get("codex").is_some() => {
            let profile = codex_catalog::form::editable_profile(&deployment.provider_id, row)
                .ok_or(unsupported)?;
            let identity = codex_catalog::identity(
                &deployment.target_host_id,
                &deployment.provider_id,
                &profile,
            )
            .map_err(|_| unsupported)?;
            Ok((identity.endpoint, profile.api_key.ok_or(unsupported)?, OpenaiResponses))
        }
        AppId::Claude if let Some(native) = row.get("claude") => {
            let profile: ClaudeProfile =
                serde_json::from_value(native.clone()).map_err(|_| unsupported)?;
            let ClaudeConfig::ThirdParty { base_url, api_key, .. } = profile.config else {
                return Err(unsupported);
            };
            Ok((base_url, api_key, Anthropic))
        }
        AppId::Gemini if let Some(native) = row.get("gemini") => {
            let profile: GeminiProfile =
                serde_json::from_value(native.clone()).map_err(|_| unsupported)?;
            let GeminiConfig::ThirdParty { base_url, api_key } = profile.config else {
                return Err(unsupported);
            };
            Ok((base_url, api_key, Gemini))
        }
        app => {
            let profile = providers::update::profile(row).map_err(|_| unsupported)?;
            providers::validate_profile(app, &profile).map_err(|_| unsupported)?;
            Ok((profile.base_url, profile.api_key, profile.protocol))
        }
    }
}

impl Work {
    pub(super) async fn discover(
        self,
        cancellation: &CancellationToken,
    ) -> Result<Vec<String>, Failure> {
        let cancelled = cancellation.cancelled();
        match self {
            Self::Http(input) => discovery_http::discover(
                HttpDiscoveryRequest {
                    base_url: &input.base_url,
                    endpoint_kind: input.endpoint_kind,
                    protocol: input.protocol,
                    models_url: input.models_url.as_deref(),
                    authorized_overrides: &[],
                    api_key: &input.api_key,
                    user_agent: input.user_agent.as_deref(),
                    request_headers: &input.headers,
                },
                cancelled,
            )
            .await
            .map(|models| models.into_iter().map(|row| row.id).collect()),
            Self::OpenCode(host) => {
                discovery_opencode::discover(&host, None, cancelled).await.map(|models| {
                    models
                        .into_iter()
                        .map(|row| format!("{}/{}", row.provider_id, row.model_id))
                        .collect()
                })
            }
            Self::Agy(host, profile) => {
                discovery_agy::discover(&host, Some(&profile), cancelled).await
            }
        }
        .map_err(|category| match category {
            "unauthorized" | "invalid_credential" => Failure::Unauthorized,
            "timeout" | "timed_out" => Failure::TimedOut,
            "invalid_response" | "response_too_large" => Failure::InvalidResponse,
            _ => Failure::Unavailable,
        })
    }
}

pub(super) fn entries(
    tenant: &TenantScope,
    deployment: &ModelCatalogIdentity,
    ids: Vec<String>,
) -> Result<Vec<ModelCatalogEntry>, Failure> {
    // The scoped wire shape repeats deployment fields per row. Bound the
    // expanded response independently of the raw HTTP/CLI payload limit.
    if ids.len() > 1024 {
        return Err(Failure::InvalidResponse);
    }
    Ok(ids
        .into_iter()
        .map(ModelId::from)
        .filter(|id| edits::validate_model_id(id).is_ok())
        .map(|model_id| ModelCatalogEntry {
            identity: ScopedModelCatalogIdentity {
                scope: tenant.clone(),
                deployment: deployment.clone(),
                model_id,
            },
            display_alias: None,
            metadata: Default::default(),
        })
        .collect())
}
