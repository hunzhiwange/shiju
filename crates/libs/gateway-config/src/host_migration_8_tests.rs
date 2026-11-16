//! Configuration domain scenarios migrated from the host; external lifecycle ports are excluded.
use crate::host::{ConfigHost, HostFuture, InstanceQuery};
use agent_config::runtime as config;
use agent_config::runtime_schema::validate_config;
use agent_database::GlobalDatabase;
use axum::{Json, extract::{Extension, Query, State}, http::{HeaderMap, StatusCode, HeaderValue}, response::IntoResponse, body::to_bytes};
use gateway_pairing::pairing::PairingGuard;
use gateway_types::ApiError;
use parking_lot::Mutex;
use serde_json::Value;
use std::{ffi::{OsStr, OsString}, sync::Arc};
use tempfile::TempDir;
use vw_api_transport::id::TenantScope;

fn global_async_env_lock() -> &'static tokio::sync::Mutex<()> {
    static LOCK: std::sync::OnceLock<tokio::sync::Mutex<()>> = std::sync::OnceLock::new();
    LOCK.get_or_init(|| tokio::sync::Mutex::new(()))
}

#[derive(Clone)]
struct TestState { config: Arc<Mutex<config::Config>>, pairing: Arc<PairingGuard> }
fn state(config: config::Config, require_pairing: bool, tokens: &[String]) -> TestState {
    TestState { config: Arc::new(Mutex::new(config)), pairing: Arc::new(PairingGuard::new(require_pairing, tokens)) }
}
#[derive(Clone)]
struct TestHost;
impl ConfigHost for TestHost {
    type State = TestState;
    type HookCandidate = ();
    fn config(state: &TestState) -> &Mutex<config::Config> { &state.config }
    fn pairing(state: &TestState) -> &PairingGuard { &state.pairing }
    fn require_auth(state: &TestState, headers: &HeaderMap) -> Result<(), (StatusCode, Json<Value>)> {
        agent_gateway::api::auth::require_pairing_auth(&state.pairing, headers)
    }
    fn mask_sensitive_fields(config: &config::Config) -> config::Config { agent_gateway::api::secrets::mask_sensitive_fields(config) }
    fn hydrate_config_for_save(incoming: config::Config, current: &config::Config) -> config::Config { agent_gateway::api::secrets::hydrate_config_for_save(incoming, current) }
    fn resolve_directory(query: &InstanceQuery, headers: &HeaderMap) -> String {
        agent_gateway::instance::resolve_directory(&agent_gateway::instance::InstanceQuery { directory: query.directory.clone() }, headers)
    }
    // These domain handlers already pass explicit tenant and project root to real storage.
    // The host tests retain actual instance bootstrap and disposal coverage.
    fn with_instance<T: Send + 'static>(_: String, action: impl FnOnce() -> HostFuture<Result<T, ApiError>> + Send + 'static) -> HostFuture<Result<T, ApiError>> { action() }
    fn list_providers() -> HostFuture<Result<std::collections::HashMap<String, agent_provider::provider::Info>, agent_provider::provider::CatalogError>> { panic!("provider lifecycle remains in the host test") }
    fn build_hook_candidate(_: &TestState, _: &config::Config) -> Result<Option<()>, String> { Ok(None) }
    fn publish_hook_candidate(_: &TestState, _: Option<()>) {}
    async fn invalidate_session_query_engines(_: &TestState) {}
}

struct EnvGuard {
    key: &'static str,
    previous: Option<OsString>,
}

impl EnvGuard {
    fn set(key: &'static str, value: &OsStr) -> Self {
        let previous = std::env::var_os(key);
        unsafe {
            std::env::set_var(key, value);
        }
        Self { key, previous }
    }

    fn unset(key: &'static str) -> Self {
        let previous = std::env::var_os(key);
        unsafe {
            std::env::remove_var(key);
        }
        Self { key, previous }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        unsafe {
            if let Some(previous) = &self.previous {
                std::env::set_var(self.key, previous);
            } else {
                std::env::remove_var(self.key);
            }
        }
    }
}

fn instance_query(temp: &TempDir) -> Query<InstanceQuery> {
    Query(InstanceQuery { directory: Some(temp.path().to_string_lossy().to_string()) })
}

fn directory_query(directory: &std::path::Path) -> Query<InstanceQuery> {
    Query(InstanceQuery { directory: Some(directory.to_string_lossy().to_string()) })
}

fn local_store(home: &std::path::Path) -> config::SystemConfigStore {
    config::SystemConfigStore::new(GlobalDatabase::for_home(home).expect("global database"))
        .expect("system config store")
}

async fn response_json(response: axum::response::Response) -> Value {
    let bytes = to_bytes(response.into_body(), usize::MAX).await.expect("body should be readable");
    serde_json::from_slice(&bytes).expect("response should be json")
}


mod read;
mod patch;
mod profiles;
mod runtime;
mod auth;
