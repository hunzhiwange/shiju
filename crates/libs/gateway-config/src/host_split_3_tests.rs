//! 目录选择与补丁形状属于配置领域；宿主端口仅执行传入的真实领域操作。
use crate::handlers::{config_patch, has_explicit_directory};
use crate::host::{ConfigHost, HostFuture, InstanceQuery};
use agent_config::runtime::Config;
use axum::{Json, extract::{Extension, Query}, http::{HeaderMap, HeaderValue, StatusCode}};
use gateway_types::ApiError;
use serde_json::Value;
use tempfile::TempDir;
use vw_api_transport::id::TenantScope;

#[derive(Clone)]
struct TestHost;

impl ConfigHost for TestHost {
    type State = ();
    type HookCandidate = ();

    fn config(_: &()) -> &parking_lot::Mutex<Config> { panic!("state is unused") }
    fn pairing(_: &()) -> &gateway_pairing::pairing::PairingGuard { panic!("pairing is unused") }
    fn require_auth(_: &(), _: &HeaderMap) -> Result<(), (StatusCode, Json<Value>)> {
        panic!("authentication is unused")
    }
    fn mask_sensitive_fields(_: &Config) -> Config { panic!("rejected patches have no response config") }
    fn hydrate_config_for_save(_: Config, _: &Config) -> Config { panic!("hydration is unused") }
    fn resolve_directory(query: &InstanceQuery, _: &HeaderMap) -> String {
        query.directory.clone().expect("explicit test directory")
    }
    fn with_instance<T: Send + 'static>(
        _: String,
        action: impl FnOnce() -> HostFuture<Result<T, ApiError>> + Send + 'static,
    ) -> HostFuture<Result<T, ApiError>> { action() }
    fn list_providers() -> HostFuture<Result<std::collections::HashMap<String, agent_provider::provider::Info>, agent_provider::provider::CatalogError>> {
        panic!("providers are unused")
    }
    fn build_hook_candidate(_: &(), _: &Config) -> Result<Option<()>, String> { panic!("hooks are unused") }
    fn publish_hook_candidate(_: &(), _: Option<()>) { panic!("hooks are unused") }
    async fn invalidate_session_query_engines(_: &()) { panic!("sessions are unused") }
}

#[test]
fn has_explicit_directory_detects_non_empty_query_or_header_values() {
    let mut headers = HeaderMap::new();
    let empty_query = InstanceQuery { directory: None };
    assert!(!has_explicit_directory(&empty_query, &headers));

    let blank_query = InstanceQuery { directory: Some("  ".to_string()) };
    assert!(!has_explicit_directory(&blank_query, &headers));

    let query = InstanceQuery { directory: Some("/tmp/project".to_string()) };
    assert!(has_explicit_directory(&query, &headers));

    headers.insert("x-directory", HeaderValue::from_static("  "));
    assert!(!has_explicit_directory(&empty_query, &headers));

    headers.insert("x-directory", HeaderValue::from_static("/tmp/project"));
    assert!(has_explicit_directory(&empty_query, &headers));
}

#[test]
fn has_explicit_directory_ignores_non_utf8_header_value() {
    let query = InstanceQuery { directory: None };
    let mut headers = HeaderMap::new();
    let invalid = HeaderValue::from_bytes(&[0xff]).expect("non-utf8 header bytes");
    headers.insert("x-directory", invalid);

    assert!(!has_explicit_directory(&query, &headers));
}

#[tokio::test]
async fn config_patch_rejects_non_object_patch() {
    let temp = TempDir::new().expect("temp dir");

    let err = config_patch::<TestHost>(
        Extension(TenantScope::default()),
        Query(InstanceQuery { directory: Some(temp.path().to_string_lossy().into_owned()) }),
        HeaderMap::new(),
        Json(serde_json::json!("invalid-patch")),
    )
    .await
    .expect_err("non-object patch should be rejected");

    assert_eq!(err.status, StatusCode::BAD_REQUEST);
    assert!(err.message.contains("config patch must be a JSON object"));
}

