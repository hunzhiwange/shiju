//! Authorized usage settings and isolated script trials.

use std::{path::PathBuf, sync::Arc, time::Duration};

use axum::{
    Extension, Json, Router,
    extract::{
        DefaultBodyLimit, FromRequestParts, Query,
        rejection::{JsonRejection, QueryRejection},
    },
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use reqwest::Url;
use serde::{Deserialize, Serialize};
use vw_agent_apps::{
    apps::lifecycle::operations::Operation,
    providers::{update::profile, validate_identity, validate_profile},
    usage::scripts::{
        config::{PreparedUsage, ProviderDefaults, ScriptParameters, UsageTemplate},
        extract::ExtractError,
        network::NetworkError,
        run::RunError,
        runtime::RuntimeError,
    },
};
use vw_api_types::{
    agent_apps_auth::AuthErrorCode,
    apps_usage::{UsageQueryError, UsageQueryErrorCode, UsageSnapshotQuery},
    id::{ProjectScope, TenantScope},
};

use super::{
    access::{Authorization, UsageScriptAction, authorize_usage_script},
    usage_read::{ReadError, usage_account_host},
    usage_refresh::UsageRefresh,
};
use crate::{
    agent_apps::{
        auth_accounts::AuthAccountScope,
        usage_settings::{
            UsageCredentialId, UsageCredentialRefs, UsageSettings, UsageSettingsError,
            UsageSettingsScope,
        },
    },
    app_providers::AppProviderStore,
    gateway::api::handlers::agent_apps_auth_read::require_provider_account,
};

const AUTHORIZATION_INTERVAL: Duration = Duration::from_secs(1);
const MAX_SOURCE_BYTES: usize = 64 * 1024;

#[derive(Clone)]
struct PolicyFile(Option<PathBuf>);

pub(super) fn router_with_policy(path: Option<PathBuf>) -> Router {
    Router::new()
        .route("/apps/usage/templates", get(templates))
        .route("/apps/usage/settings", get(read_settings).put(save_settings))
        .route("/apps/usage/script-test", post(script_test))
        .layer(DefaultBodyLimit::max(2 * MAX_SOURCE_BYTES))
        .layer(Extension(PolicyFile(path)))
}

#[derive(FromRequestParts)]
struct RequestContext {
    policy: Extension<PolicyFile>,
    tenant: Extension<TenantScope>,
    headers: HeaderMap,
    service: Option<Extension<Arc<UsageRefresh>>>,
}

#[derive(Clone)]
struct Selection {
    policy: PolicyFile,
    headers: HeaderMap,
    project: ProjectScope,
    query: UsageSnapshotQuery,
    service: Option<Arc<UsageRefresh>>,
}

impl RequestContext {
    fn select(self, query: UsageSnapshotQuery) -> Result<Selection, ReadError> {
        let project = ProjectScope::new(self.tenant.0, query.project_id.clone())
            .map_err(|_| AuthErrorCode::InvalidInput)?;
        for id in [
            Some(query.target_host_id.as_str()),
            Some(query.provider_id.as_str()),
            query.account_id.as_deref(),
        ]
        .into_iter()
        .flatten()
        {
            validate_identity(id).map_err(|_| AuthErrorCode::InvalidInput)?;
        }
        if query.kind.is_some() {
            return Err(AuthErrorCode::InvalidInput.into());
        }
        Ok(Selection {
            policy: self.policy.0,
            headers: self.headers,
            project,
            query,
            service: self.service.map(|Extension(service)| service),
        })
    }
}

impl Selection {
    fn service(&self) -> Result<&UsageRefresh, ReadError> {
        self.service.as_deref().ok_or_else(|| AuthErrorCode::StorageUnavailable.into())
    }

    fn authorize(&self, action: UsageScriptAction) -> Result<Authorization, ReadError> {
        let authorization = authorize_usage_script(
            self.policy.0.as_deref(),
            &self.headers,
            self.project.tenant_scope(),
            &self.query,
            action,
        )
        .map_err(ReadError::access)?;
        let service = self.service()?;
        if service
            .projects
            .get(&self.project)
            .map_err(|_| AuthErrorCode::StorageUnavailable)?
            .is_none()
        {
            return Err(AuthErrorCode::NotFound.into());
        }
        Ok(authorization)
    }

    fn scope<'a>(&'a self, authorization: &'a Authorization) -> UsageSettingsScope<'a> {
        UsageSettingsScope {
            project: &self.project,
            principal: &authorization.principal,
            target_host: &authorization.host.id,
            app: self.query.app_id,
            provider: &self.query.provider_id,
            account: self.query.account_id.as_deref(),
        }
    }

    fn require_resource(&self, authorization: &Authorization) -> Result<(), ReadError> {
        if let Some(account) = self.query.account_id.as_deref() {
            let (host, service) =
                usage_account_host(self.query.app_id, authorization, self.policy.0.is_some())?;
            require_provider_account(
                self.service()?.database.clone(),
                &host,
                AuthAccountScope {
                    tenant: self.project.tenant_scope(),
                    principal: &host.principal,
                    host: &host.id,
                    service,
                },
                self.query.app_id,
                &self.query.provider_id,
                account,
            )?;
        } else {
            self.provider_record()?;
        }
        Ok(())
    }

    fn provider_record(&self) -> Result<serde_json::Value, ReadError> {
        Ok(AppProviderStore::usage_record(
            &self.service()?.database,
            self.project.tenant_scope(),
            self.query.app_id,
            &self.query.provider_id,
        )?)
    }

    async fn trial_inputs(&self, key: Option<UsageCredentialId>) -> Result<TrialInputs, ReadError> {
        let selection = self.clone();
        tokio::task::spawn_blocking(move || {
            let authorization = selection.authorize(UsageScriptAction::Test(key))?;
            if selection.query.account_id.is_some() {
                selection.require_resource(&authorization)?;
                return Err(AuthErrorCode::Unsupported.into());
            }
            let record = selection.provider_record()?;
            let profile = profile(&record).map_err(|_| AuthErrorCode::Unsupported)?;
            validate_profile(selection.query.app_id, &profile)
                .map_err(|_| AuthErrorCode::InvalidInput)?;
            Ok(TrialInputs {
                principal: authorization.principal,
                origins: authorization.usage_script_origins,
                base_url: profile.base_url,
                api_key: key.map(|_| profile.api_key),
            })
        })
        .await
        .map_err(|_| AuthErrorCode::StorageUnavailable)?
    }
}

#[derive(Serialize)]
struct TemplateMetadata {
    template: UsageTemplate,
    source: Option<&'static str>,
}

async fn templates(
    context: RequestContext,
    query: Result<Query<UsageSnapshotQuery>, QueryRejection>,
) -> Result<impl IntoResponse, ReadError> {
    let Query(query) = query.map_err(|_| AuthErrorCode::InvalidInput)?;
    let selection = context.select(query)?;
    tokio::task::spawn_blocking(move || {
        let authorization = selection.authorize(UsageScriptAction::Templates)?;
        selection.require_resource(&authorization)?;
        Ok::<_, ReadError>(())
    })
    .await
    .map_err(|_| AuthErrorCode::StorageUnavailable)??;
    use UsageTemplate::*;
    let templates = [
        General,
        Custom,
        NewApi,
        Sub2Api,
        GitHubCopilot,
        CodingPlan,
        Balance,
        OfficialSubscription,
    ];
    Ok(no_store(
        templates.map(|template| TemplateMetadata { template, source: template.preset_source() }),
    ))
}

async fn read_settings(
    context: RequestContext,
    query: Result<Query<UsageSnapshotQuery>, QueryRejection>,
) -> Result<impl IntoResponse, ReadError> {
    let Query(query) = query.map_err(|_| AuthErrorCode::InvalidInput)?;
    let selection = context.select(query)?;
    let settings = tokio::task::spawn_blocking(move || {
        let authorization = selection.authorize(UsageScriptAction::SettingsRead)?;
        selection.require_resource(&authorization)?;
        selection.service()?.settings.get(selection.scope(&authorization)).map_err(settings_error)
    })
    .await
    .map_err(|_| AuthErrorCode::StorageUnavailable)??;
    Ok(no_store(settings))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SaveRequest {
    selection: UsageSnapshotQuery,
    settings: UsageSettings,
}

async fn save_settings(
    context: RequestContext,
    request: Result<Json<SaveRequest>, JsonRejection>,
) -> Result<StatusCode, ReadError> {
    let Json(request) = request.map_err(|_| AuthErrorCode::InvalidInput)?;
    let selection = context.select(request.selection)?;
    tokio::task::spawn_blocking(move || {
        let key = provider_key_ref(&request.settings.credentials)?;
        let authorization = selection.authorize(UsageScriptAction::SettingsWrite(key))?;
        if key.is_some()
            && (selection.query.account_id.is_some()
                || !matches!(
                    request.settings.template,
                    UsageTemplate::General | UsageTemplate::Sub2Api
                ))
        {
            return Err(AuthErrorCode::InvalidInput.into());
        }
        selection.require_resource(&authorization)?;
        selection
            .service()?
            .settings
            .save(selection.scope(&authorization), &request.settings)
            .map_err(settings_error)?;
        Ok(StatusCode::NO_CONTENT)
    })
    .await
    .map_err(|_| AuthErrorCode::StorageUnavailable)?
}

fn settings_error(error: UsageSettingsError) -> ReadError {
    match error {
        UsageSettingsError::StorageUnavailable => AuthErrorCode::StorageUnavailable,
        UsageSettingsError::InvalidScope
        | UsageSettingsError::InvalidCredentialRef
        | UsageSettingsError::InvalidRefreshInterval => AuthErrorCode::InvalidInput,
    }
    .into()
}

fn provider_key_ref(refs: &UsageCredentialRefs) -> Result<Option<UsageCredentialId>, ReadError> {
    if refs.access_token_ref.is_some() || refs.signing.is_some() {
        return Err(AuthErrorCode::Forbidden.into());
    }
    Ok(refs.api_key_ref)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct TrialRequest {
    selection: UsageSnapshotQuery,
    template: UsageTemplate,
    source: Option<String>,
    timeout_seconds: Option<u64>,
    #[serde(default)]
    credentials: UsageCredentialRefs,
}

#[derive(PartialEq, Eq)]
struct TrialInputs {
    principal: String,
    origins: Vec<Url>,
    base_url: String,
    api_key: Option<String>,
}

/// Cancel work when the HTTP subscriber drops. The runtime also polls this
/// operation from blocking QuickJS workers.
struct TrialOperation(Operation);

impl Drop for TrialOperation {
    fn drop(&mut self) {
        self.0.cancel();
    }
}

async fn script_test(
    context: RequestContext,
    request: Result<Json<TrialRequest>, JsonRejection>,
) -> Result<Response, ReadError> {
    let Json(request) = request.map_err(|_| AuthErrorCode::InvalidInput)?;
    let selection = context.select(request.selection.clone())?;
    let operation = TrialOperation(Operation::new());
    tokio::spawn(run_trial(selection, request, operation.0.clone()))
        .await
        .map_err(|_| AuthErrorCode::StorageUnavailable)?
}

async fn run_trial(
    selection: Selection,
    request: TrialRequest,
    operation: Operation,
) -> Result<Response, ReadError> {
    let key = provider_key_ref(&request.credentials)?;
    let initial = selection.trial_inputs(key).await?;
    let source = request.source.as_deref();
    match (request.template, source, key) {
        (UsageTemplate::Custom, Some(source), None)
            if !source.trim().is_empty() && source.len() <= MAX_SOURCE_BYTES => {}
        (UsageTemplate::General | UsageTemplate::Sub2Api, None, Some(_)) => {}
        (UsageTemplate::Custom | UsageTemplate::General | UsageTemplate::Sub2Api, _, _) => {
            return Err(AuthErrorCode::InvalidInput.into());
        }
        _ => return Err(AuthErrorCode::Unsupported.into()),
    }

    let slots = &selection.service()?.query_slots;
    let _slot = tokio::select! {
        biased;
        () = operation.cancelled() => return Err(AuthErrorCode::Conflict.into()),
        slot = slots.acquire() => slot.map_err(|_| AuthErrorCode::StorageUnavailable)?,
    };
    if initial != selection.trial_inputs(key).await? {
        return Err(AuthErrorCode::Conflict.into());
    }

    let PreparedUsage::Script(script) = request
        .template
        .prepare(
            source,
            ScriptParameters::default(),
            ProviderDefaults {
                base_url: Some(&initial.base_url),
                api_key: initial.api_key.as_deref(),
            },
        )
        .map_err(|_| AuthErrorCode::InvalidInput)?
    else {
        return Err(AuthErrorCode::Unsupported.into());
    };
    let response = script.run(request.timeout_seconds.unwrap_or(10), &initial.origins, &operation);
    tokio::pin!(response);
    let mut authorization_checks = tokio::time::interval_at(
        tokio::time::Instant::now() + AUTHORIZATION_INTERVAL,
        AUTHORIZATION_INTERVAL,
    );
    let result = loop {
        tokio::select! {
            biased;
            result = &mut response => break result,
            _ = authorization_checks.tick() => {
                let checked = selection.trial_inputs(key).await;
                match checked {
                    Ok(current) if current == initial => {}
                    revoked => {
                        operation.cancel();
                        let _ = response.await;
                        return Err(revoked.err().unwrap_or_else(|| AuthErrorCode::Conflict.into()));
                    }
                }
            }
        }
    };
    if initial != selection.trial_inputs(key).await? {
        return Err(AuthErrorCode::Conflict.into());
    }

    match result {
        Ok(packages) => Ok(no_store(packages).into_response()),
        Err(error) => trial_error(error),
    }
}

fn trial_error(error: RunError) -> Result<Response, ReadError> {
    use UsageQueryErrorCode::*;
    let code = match error {
        RunError::Network(NetworkError::DestinationDenied | NetworkError::Redirect) => {
            return Err(AuthErrorCode::Forbidden.into());
        }
        RunError::Network(NetworkError::Runtime(RuntimeError::TimedOut))
        | RunError::Extract(ExtractError::Runtime(RuntimeError::TimedOut)) => Timeout,
        RunError::Network(NetworkError::Runtime(RuntimeError::Unavailable))
        | RunError::Extract(ExtractError::Runtime(RuntimeError::Unavailable)) => {
            return Err(AuthErrorCode::ExecutionHostUnavailable.into());
        }
        RunError::Network(NetworkError::Runtime(_) | NetworkError::InvalidRequest) => {
            return Err(AuthErrorCode::InvalidInput.into());
        }
        RunError::Network(NetworkError::Upstream(code)) => code,
        RunError::Extract(_) => InvalidResponse,
        RunError::Cancelled => return Err(AuthErrorCode::Conflict.into()),
    };
    let status =
        if code == Timeout { StatusCode::GATEWAY_TIMEOUT } else { StatusCode::BAD_GATEWAY };
    Ok((
        status,
        no_store(UsageQueryError {
            code,
            message: "Usage script trial failed".into(),
            correlation_id: uuid::Uuid::new_v4().to_string(),
        }),
    )
        .into_response())
}

fn no_store<T>(body: T) -> ([(header::HeaderName, &'static str); 1], Json<T>) {
    ([(header::CACHE_CONTROL, "no-store")], Json(body))
}
