//! Scope-bound management of production observations and request policy.
use super::{ControlError, ReadError, RoutingControl};
use crate::{
    app_providers::AppProviderStore,
    gateway::{
        agent_apps_failover_runtime,
        agent_apps_high_availability_store::NAMESPACE,
        settings_store::GatewaySettingsStore,
    },
    global_database::GlobalDatabase,
};
use axum::{
    Extension, Json, Router,
    extract::{Query, rejection::{JsonRejection, QueryRejection}},
    http::HeaderMap,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde::Deserialize;
use vw_agent_apps::{
    high_availability::health::{HealthAccount, HealthKey, HealthSnapshot},
    providers::{failover::ProviderAuth, validate_identity},
    routing::supports_app,
};
use vw_api_types::{
    agent_apps::AppId,
    agent_apps_high_availability::{
        FailoverHealthResponse, FailoverHealthState, FailoverPolicy, FailoverQueueScope,
    },
    id::{ProjectId, ProjectScope, ProviderId, TenantScope},
};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ScopeQuery {
    project_id: ProjectId,
    target_host_id: String,
    app_id: AppId,
    // Required for health reads/resets and forbidden for policy/route operations.
    provider_id: Option<ProviderId>,
}

enum Operation {
    Health,
    Reset,
    Route,
    Policy,
    SavePolicy(FailoverPolicy),
}

pub(super) fn router() -> Router {
    Router::new()
        .route("/agent-apps/high-availability/health", get(health))
        .route("/agent-apps/high-availability/health/reset", post(reset))
        .route("/agent-apps/high-availability/route", get(route))
        .route("/agent-apps/high-availability/policy", get(policy).put(save_policy))
}

async fn health(
    control: Option<Extension<RoutingControl>>,
    Extension(tenant): Extension<TenantScope>,
    database: Option<Extension<GlobalDatabase>>,
    headers: HeaderMap,
    query: Result<Query<ScopeQuery>, QueryRejection>,
) -> Result<Response, ReadError> {
    execute(control, tenant, database, headers, query, Operation::Health).await
}

async fn reset(
    control: Option<Extension<RoutingControl>>,
    Extension(tenant): Extension<TenantScope>,
    database: Option<Extension<GlobalDatabase>>,
    headers: HeaderMap,
    query: Result<Query<ScopeQuery>, QueryRejection>,
) -> Result<Response, ReadError> {
    execute(control, tenant, database, headers, query, Operation::Reset).await
}

async fn route(
    control: Option<Extension<RoutingControl>>,
    Extension(tenant): Extension<TenantScope>,
    database: Option<Extension<GlobalDatabase>>,
    headers: HeaderMap,
    query: Result<Query<ScopeQuery>, QueryRejection>,
) -> Result<Response, ReadError> {
    execute(control, tenant, database, headers, query, Operation::Route).await
}

async fn policy(
    control: Option<Extension<RoutingControl>>,
    Extension(tenant): Extension<TenantScope>,
    database: Option<Extension<GlobalDatabase>>,
    headers: HeaderMap,
    query: Result<Query<ScopeQuery>, QueryRejection>,
) -> Result<Response, ReadError> {
    execute(control, tenant, database, headers, query, Operation::Policy).await
}

async fn save_policy(
    control: Option<Extension<RoutingControl>>,
    Extension(tenant): Extension<TenantScope>,
    database: Option<Extension<GlobalDatabase>>,
    headers: HeaderMap,
    query: Result<Query<ScopeQuery>, QueryRejection>,
    body: Result<Json<FailoverPolicy>, JsonRejection>,
) -> Result<Response, ReadError> {
    let Json(policy) = body.map_err(|_| ReadError::InvalidInput)?;
    policy.validate().map_err(|_| ReadError::InvalidInput)?;
    execute(control, tenant, database, headers, query, Operation::SavePolicy(policy)).await
}

async fn execute(
    control: Option<Extension<RoutingControl>>,
    tenant: TenantScope,
    database: Option<Extension<GlobalDatabase>>,
    headers: HeaderMap,
    query: Result<Query<ScopeQuery>, QueryRejection>,
    operation: Operation,
) -> Result<Response, ReadError> {
    let Query(query) = query.map_err(|_| ReadError::InvalidInput)?;
    validate_identity(&query.target_host_id).map_err(|_| ReadError::InvalidInput)?;
    let needs_provider = matches!(operation, Operation::Health | Operation::Reset);
    if needs_provider != query.provider_id.is_some() {
        return Err(ReadError::InvalidInput);
    }
    if let Some(id) = &query.provider_id {
        validate_identity(id.as_ref()).map_err(|_| ReadError::InvalidInput)?;
    }
    let project = ProjectScope::new(tenant.clone(), query.project_id.clone())
        .map_err(|_| ReadError::InvalidInput)?;
    let manage = matches!(operation, Operation::Reset | Operation::SavePolicy(_));
    let (control, _) = RoutingControl::authorize(
        control, headers, tenant, query.target_host_id.clone(),
        Some((query.project_id, query.app_id)), manage,
    )
    .await
    .map_err(|error| match error {
        ControlError::Unauthenticated => ReadError::Unauthenticated,
        ControlError::Forbidden => ReadError::Forbidden,
        ControlError::NotFound => ReadError::NotFound,
        _ => ReadError::Unavailable,
    })?;
    if !supports_app(query.app_id) {
        return Err(ReadError::UnsupportedMode);
    }
    let scope = FailoverQueueScope {
        project, target_host_id: query.target_host_id, app_id: query.app_id,
    };
    tokio::task::spawn_blocking(move || {
        let database = match database {
            Some(Extension(database)) => database,
            None => GlobalDatabase::for_home(&crate::global::paths().home)
                .map_err(|_| ReadError::Unavailable)?,
        };
        match operation {
            Operation::Route => control.routing.failover_route(&scope)
                .map(|route| Json(route).into_response()).map_err(ReadError::from_status),
            Operation::Policy => agent_apps_failover_runtime::policy(database, &scope)
                .map(|policy| Json(policy).into_response()).map_err(|_| ReadError::Unavailable),
            Operation::SavePolicy(policy) => {
                let key = agent_apps_failover_runtime::policy_key(&scope)
                    .map_err(|_| ReadError::InvalidInput)?;
                let payload = serde_json::to_string(&policy).map_err(|_| ReadError::InvalidInput)?;
                GatewaySettingsStore::new(database).and_then(|store| {
                    store.upsert(scope.project.tenant_scope(), NAMESPACE, &key, &payload)
                }).map_err(|_| ReadError::Unavailable)?;
                Ok(Json(policy).into_response())
            }
            Operation::Health | Operation::Reset => {
                let id = query.provider_id.ok_or(ReadError::InvalidInput)?;
                let providers = AppProviderStore::new(database.clone())
                    .and_then(|store| store.failover_providers(&scope.project, scope.app_id))
                    .map_err(|_| ReadError::Unavailable)?;
                let provider = providers.iter().find(|entry| entry.provider.id == id.as_ref())
                    .ok_or(ReadError::NotFound)?;
                // The production failover executor uses only third-party, unbound
                // credentials. Never fold official or unresolved account health into it.
                let snapshot = if provider.auth == ProviderAuth::ThirdParty {
                    let policy = agent_apps_failover_runtime::policy(database, &scope)
                        .map_err(|_| ReadError::Unavailable)?;
                    let key = HealthKey {
                        project: scope.project.clone(), target_host_id: scope.target_host_id.clone(),
                        app_id: scope.app_id, provider_id: id.clone(), account: HealthAccount::Unbound,
                    };
                    control.routing.failover_health(&key, &policy.circuit_breaker,
                        matches!(operation, Operation::Reset))
                } else {
                    if matches!(operation, Operation::Reset) {
                        return Err(ReadError::IneligibleProvider);
                    }
                    HealthSnapshot { state: FailoverHealthState::Unknown, retry_after_ms: None }
                };
                Ok(Json(FailoverHealthResponse {
                    scope, provider_id: id, state: snapshot.state, retry_after_ms: snapshot.retry_after_ms,
                }).into_response())
            }
        }
    }).await.map_err(|_| ReadError::Unavailable)?
}
