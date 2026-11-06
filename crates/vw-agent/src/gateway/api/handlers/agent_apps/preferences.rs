//! Preferences share the gateway settings database, never execution-host files.
use crate::{gateway::settings_store::GatewaySettingsStore, global_database::GlobalDatabase};
use axum::{
    Extension, Json, Router,
    extract::{DefaultBodyLimit, Query},
    http::{HeaderMap, StatusCode},
    routing::get,
};
use serde::Deserialize;
use std::path::PathBuf;
use vw_agent_apps::apps::preferences::{
    HostPlatform, default_user_preferences, normalize_preferences,
};
use vw_api_types::{
    agent_apps::{AppHostPreferences, AppPreferencesPatch as Patch, AppPreferencesResult},
    id::TenantScope,
};

#[derive(Clone)]
struct PolicyFile(Option<PathBuf>);
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Target {
    target_host_id: String,
}
pub(super) fn router() -> Router {
    router_with_policy(std::env::var_os("SHIJU_AGENT_APPS_ACCESS_FILE").map(PathBuf::from))
}
pub(super) fn router_with_policy(path: Option<PathBuf>) -> Router {
    Router::new()
        .route("/agent-apps/preferences", get(read).patch(patch))
        .layer(DefaultBodyLimit::max(16384))
        .layer(Extension(PolicyFile(path)))
}
async fn read(
    Extension(policy): Extension<PolicyFile>,
    Extension(tenant): Extension<TenantScope>,
    database: Option<Extension<GlobalDatabase>>,
    headers: HeaderMap,
    Query(target): Query<Target>,
) -> Result<Json<AppPreferencesResult>, StatusCode> {
    run(policy, tenant, database, headers, target, None).await
}
async fn patch(
    Extension(policy): Extension<PolicyFile>,
    Extension(tenant): Extension<TenantScope>,
    database: Option<Extension<GlobalDatabase>>,
    headers: HeaderMap,
    Query(target): Query<Target>,
    Json(patch): Json<Patch>,
) -> Result<Json<AppPreferencesResult>, StatusCode> {
    run(policy, tenant, database, headers, target, Some(patch)).await
}
async fn run(
    policy: PolicyFile,
    tenant: TenantScope,
    database: Option<Extension<GlobalDatabase>>,
    headers: HeaderMap,
    target: Target,
    patch: Option<Patch>,
) -> Result<Json<AppPreferencesResult>, StatusCode> {
    tokio::task::spawn_blocking(move || {
        let (host, principal) = super::access::authorize_preferences(
            policy.0.as_deref(),
            &headers,
            &tenant,
            &target.target_host_id,
            None,
            matches!(patch, Some(Patch::Host(_))),
        )?;
        let database = match database {
            Some(Extension(db)) => db,
            None => GlobalDatabase::for_home(&crate::global::paths().home)
                .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?,
        };
        let store =
            GatewaySettingsStore::new(database).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
        let user = load(&store, &tenant, "agent_apps_user", &principal)?
            .unwrap_or_else(default_user_preferences);
        let previous: AppHostPreferences =
            load(&store, &tenant, "agent_apps_host", &host.id)?.unwrap_or_default();
        let (user, next) = match &patch {
            Some(Patch::User(value)) => (value.clone(), previous.clone()),
            Some(Patch::Host(value)) => (user, value.clone()),
            None => (user, previous.clone()),
        };
        let platform = if cfg!(target_os = "macos") {
            HostPlatform::MacOs
        } else if cfg!(windows) {
            HostPlatform::Windows
        } else {
            HostPlatform::Linux
        };
        let result = normalize_preferences(user, next, &previous, platform)
            .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
        if let Some(patch) = patch {
            let (namespace, key, value) = match patch {
                Patch::User(_) => {
                    ("agent_apps_user", principal.as_str(), serde_json::to_string(&result.user))
                }
                Patch::Host(_) => {
                    ("agent_apps_host", host.id.as_str(), serde_json::to_string(&result.host))
                }
            };
            store
                .upsert(
                    &tenant,
                    namespace,
                    key,
                    &value.map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?,
                )
                .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
        }
        Ok(Json(result))
    })
    .await
    .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
}
fn load<T: serde::de::DeserializeOwned>(
    store: &GatewaySettingsStore,
    tenant: &TenantScope,
    namespace: &str,
    key: &str,
) -> Result<Option<T>, StatusCode> {
    store
        .get(tenant, namespace, key)
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
        .map(|s| serde_json::from_str(&s).map_err(|_| StatusCode::SERVICE_UNAVAILABLE))
        .transpose()
}
