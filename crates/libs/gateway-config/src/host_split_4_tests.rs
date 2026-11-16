//! 配置写入前的领域校验；不装配宿主服务或访问持久化存储。
use crate::handlers::handle_api_config_put;
use crate::host::{ConfigHost, HostFuture, InstanceQuery};
use agent_config::runtime as config;
use axum::{Json, body::to_bytes, extract::{Extension, State}, http::{HeaderMap, StatusCode}, response::IntoResponse};
use gateway_types::ApiError;
use parking_lot::Mutex;
use serde_json::Value;
use std::sync::Arc;
use vw_api_transport::id::TenantScope;

#[derive(Clone)]
struct TestState {
    config: Arc<Mutex<config::Config>>,
}

fn state(config: config::Config) -> TestState {
    TestState { config: Arc::new(Mutex::new(config)) }
}

#[derive(Clone)]
struct TestHost;

impl ConfigHost for TestHost {
    type State = TestState;
    type HookCandidate = ();

    fn config(state: &TestState) -> &Mutex<config::Config> { &state.config }
    fn pairing(_: &TestState) -> &gateway_pairing::pairing::PairingGuard {
        panic!("rejected config must not update pairing")
    }
    fn require_auth(_: &TestState, _: &HeaderMap) -> Result<(), (StatusCode, Json<Value>)> {
        // Authentication itself remains covered by the real host tests.
        Ok(())
    }
    fn mask_sensitive_fields(_: &config::Config) -> config::Config {
        panic!("rejected config has no config response")
    }
    fn hydrate_config_for_save(incoming: config::Config, _: &config::Config) -> config::Config {
        // These inputs contain no masked secrets; secret hydration stays with the host.
        incoming
    }
    fn resolve_directory(_: &InstanceQuery, _: &HeaderMap) -> String {
        panic!("directory resolution is unused")
    }
    fn with_instance<T: Send + 'static>(
        _: String,
        _: impl FnOnce() -> HostFuture<Result<T, ApiError>> + Send + 'static,
    ) -> HostFuture<Result<T, ApiError>> { panic!("instance scope is unused") }
    fn list_providers() -> HostFuture<Result<std::collections::HashMap<String, agent_provider::provider::Info>, agent_provider::provider::CatalogError>> {
        panic!("providers are unused")
    }
    fn build_hook_candidate(_: &TestState, _: &config::Config) -> Result<Option<()>, String> {
        panic!("rejected config must not build hooks")
    }
    fn publish_hook_candidate(_: &TestState, _: Option<()>) {
        panic!("rejected config must not publish hooks")
    }
    async fn invalidate_session_query_engines(_: &TestState) {
        panic!("rejected config must not invalidate sessions")
    }
}

async fn response_json(response: axum::response::Response) -> Value {
    let bytes = to_bytes(response.into_body(), usize::MAX).await.expect("body should be readable");
    serde_json::from_slice(&bytes).expect("response should be json")
}

#[tokio::test]
async fn handle_api_config_put_rejects_invalid_toml() {
    let response = handle_api_config_put::<TestHost>(
        State(state(config::Config::default())),
        Extension(TenantScope::default()),
        HeaderMap::new(),
        "not = valid = toml".to_string(),
    )
    .await
    .into_response();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert!(response_json(response).await["error"].as_str().unwrap().contains("Invalid TOML"));
}

#[tokio::test]
async fn handle_api_config_put_rejects_type_mismatch_after_toml_parsing() {
    let current = config::Config::default();
    let mut incoming_toml: toml::Value =
        toml::from_str(&toml::to_string_pretty(&current).expect("default config should serialize"))
            .expect("default config should parse as toml value");
    incoming_toml
        .as_table_mut()
        .expect("config root table")
        .insert("default_temperature".to_string(), toml::Value::String("warm".to_string()));
    let body =
        toml::to_string_pretty(&incoming_toml).expect("invalid typed config should serialize");
    let state = state(current);

    let response = handle_api_config_put::<TestHost>(
        State(state),
        Extension(TenantScope::default()),
        HeaderMap::new(),
        body,
    )
    .await
    .into_response();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert!(response_json(response).await["error"].as_str().unwrap().contains("Invalid TOML"));
}

#[tokio::test]
async fn handle_api_config_put_rejects_invalid_config_without_mutating_runtime_state() {
    let current = config::Config::default();
    let expected_host = current.gateway.host.clone();
    let mut incoming = current.clone();
    incoming.gateway.host = " ".to_string();
    let body = toml::to_string_pretty(&incoming).expect("invalid config should serialize");
    let state = state(current);

    let response = handle_api_config_put::<TestHost>(
        State(state.clone()),
        Extension(TenantScope::default()),
        HeaderMap::new(),
        body,
    )
    .await
    .into_response();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert!(
        response_json(response).await["error"]
            .as_str()
            .unwrap()
            .contains("Invalid config: gateway.host must not be empty")
    );
    assert_eq!(state.config.lock().gateway.host, expected_host);
}
