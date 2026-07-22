use super::*;
use crate::app::agent::gateway::instance::InstanceQuery;
use crate::app::agent::project;
use crate::app::agent::storage;
use axum::http::StatusCode;
use axum::http::{HeaderMap, HeaderValue};
use std::sync::Once;
use tempfile::TempDir;
use vw_api_types::session::GatewaySessionCreateBody;

static TEST_HOME: Once = Once::new();
static SHARED_TEST_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

fn ensure_test_home() {
    TEST_HOME.call_once(|| {
        let dir =
            std::env::temp_dir().join(format!("vw-session-shared-tests-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("test home should be created");
        unsafe {
            std::env::set_var("VIBEWINDOW_TEST_HOME", dir);
        }
    });
}

#[test]
fn session_api_error_maps_storage_not_found_to_not_found() {
    let error = session_api_error(agent_session::session::Error::Storage(
        storage::Error::NotFound(storage::NotFoundError { message: "missing session".to_string() }),
    ));

    assert_eq!(error.status, StatusCode::NOT_FOUND);
    assert_eq!(error.to_string(), "missing session");
}

#[test]
fn session_api_error_maps_other_errors_to_bad_request() {
    let error = session_api_error(agent_session::session::Error::NoProjectContext);

    assert_eq!(error.status, StatusCode::BAD_REQUEST);
    assert_eq!(error.to_string(), "no active project context");
}

#[test]
fn session_api_error_maps_non_not_found_storage_errors_to_bad_request() {
    let storage_error = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "storage denied");
    let error = session_api_error(agent_session::session::Error::Storage(storage::Error::Io(
        storage_error,
    )));

    assert_eq!(error.status, StatusCode::BAD_REQUEST);
    assert_eq!(error.to_string(), "storage denied");
}

#[test]
fn has_explicit_directory_checks_query_before_headers() {
    let query = InstanceQuery { directory: Some("/tmp/project".to_string()) };
    let headers = HeaderMap::new();

    assert!(has_explicit_directory(&query, &headers));
}

#[test]
fn has_explicit_directory_accepts_directory_header() {
    let query = InstanceQuery { directory: None };
    let mut headers = HeaderMap::new();
    headers.insert("x-vibewindow-directory", HeaderValue::from_static("/tmp/project"));

    assert!(has_explicit_directory(&query, &headers));
}

#[test]
fn has_explicit_directory_falls_back_to_header_when_query_is_blank() {
    let query = InstanceQuery { directory: Some("  ".to_string()) };
    let mut headers = HeaderMap::new();
    headers.insert("x-vibewindow-directory", HeaderValue::from_static(" /tmp/project "));

    assert!(has_explicit_directory(&query, &headers));
}

#[test]
fn has_explicit_directory_rejects_blank_query_and_header() {
    let query = InstanceQuery { directory: Some("  ".to_string()) };
    let mut headers = HeaderMap::new();
    headers.insert("x-vibewindow-directory", HeaderValue::from_static("  "));

    assert!(!has_explicit_directory(&query, &headers));
}

#[test]
fn has_explicit_directory_rejects_absent_query_and_header() {
    let query = InstanceQuery { directory: None };
    let headers = HeaderMap::new();

    assert!(!has_explicit_directory(&query, &headers));
}

#[test]
fn has_explicit_directory_ignores_invalid_header_value() {
    let query = InstanceQuery { directory: None };
    let mut headers = HeaderMap::new();
    let value = HeaderValue::from_bytes(b"\xff").expect("header bytes");
    headers.insert("x-vibewindow-directory", value);

    assert!(!has_explicit_directory(&query, &headers));
}

#[test]
fn ui_session_list_query_deserializes_defaults() {
    let query: UiSessionListQuery = serde_json::from_value(serde_json::json!({
        "roots": true,
        "limit": 20
    }))
    .expect("valid query");

    assert_eq!(query.directory, None);
    assert_eq!(query.roots, Some(true));
    assert_eq!(query.start, None);
    assert_eq!(query.search, None);
    assert_eq!(query.limit, Some(20));
}

#[test]
fn ui_session_list_query_deserializes_empty_filters() {
    let query: UiSessionListQuery =
        serde_json::from_value(serde_json::json!({})).expect("valid empty query");

    assert_eq!(query.directory, None);
    assert_eq!(query.roots, None);
    assert_eq!(query.start, None);
    assert_eq!(query.search, None);
    assert_eq!(query.limit, None);
}

#[test]
fn ui_session_list_query_rejects_non_numeric_limit() {
    let result = serde_json::from_value::<UiSessionListQuery>(serde_json::json!({
        "limit": "ten"
    }));

    assert!(result.is_err());
}

#[test]
fn ui_session_list_query_deserializes_all_filters() {
    let query: UiSessionListQuery = serde_json::from_value(serde_json::json!({
        "directory": "/tmp/project",
        "roots": false,
        "start": 42,
        "search": "topic",
        "limit": 10
    }))
    .expect("valid query");

    assert_eq!(query.directory.as_deref(), Some("/tmp/project"));
    assert_eq!(query.roots, Some(false));
    assert_eq!(query.start, Some(42));
    assert_eq!(query.search.as_deref(), Some("topic"));
    assert_eq!(query.limit, Some(10));
}

#[test]
fn ui_session_create_body_deserializes_flattened_session_and_permission() {
    let body: UiSessionCreateBody = serde_json::from_value(serde_json::json!({
        "parentID": "parent-1",
        "title": "Child",
        "permission": [
            {
                "permission": "shell.exec",
                "pattern": "cargo test",
                "action": "allow"
            }
        ]
    }))
    .expect("valid create body");

    assert_eq!(
        body.session,
        GatewaySessionCreateBody {
            parent_id: Some("parent-1".to_string()),
            title: Some("Child".to_string()),
        }
    );
    let permission = body.permission.expect("permission rules");
    assert_eq!(permission.len(), 1);
    assert_eq!(permission[0].permission, "shell.exec");
}

#[test]
fn ui_session_create_body_allows_permission_to_be_absent() {
    let body: UiSessionCreateBody = serde_json::from_value(serde_json::json!({
        "title": "Standalone"
    }))
    .expect("valid create body");

    assert_eq!(
        body.session,
        GatewaySessionCreateBody { parent_id: None, title: Some("Standalone".to_string()) }
    );
    assert!(body.permission.is_none());
}

#[test]
fn ui_session_create_body_deserializes_empty_request() {
    let body: UiSessionCreateBody =
        serde_json::from_value(serde_json::json!({})).expect("valid empty create body");

    assert_eq!(body.session, GatewaySessionCreateBody::default());
    assert!(body.permission.is_none());
}

#[test]
fn ui_session_create_body_rejects_unknown_permission_action() {
    let result = serde_json::from_value::<UiSessionCreateBody>(serde_json::json!({
        "permission": [
            {
                "permission": "shell.exec",
                "pattern": "*",
                "action": "sometimes"
            }
        ]
    }));

    assert!(result.is_err());
}

#[test]
fn resolve_scope_from_query_returns_scope_for_directory_query() {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let query = InstanceQuery { directory: Some(temp_dir.path().to_string_lossy().to_string()) };
    let headers = HeaderMap::new();

    assert!(
        resolve_scope_from_query(&vw_api_types::id::TenantScope::default(), &query, &headers)
            .is_some()
    );
}

#[test]
fn resolve_scope_from_query_returns_scope_for_directory_header() {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let query = InstanceQuery { directory: None };
    let mut headers = HeaderMap::new();
    let path = temp_dir.path().to_string_lossy();
    let value = HeaderValue::from_str(path.as_ref()).expect("header value");
    headers.insert("x-vibewindow-directory", value);

    assert!(
        resolve_scope_from_query(&vw_api_types::id::TenantScope::default(), &query, &headers)
            .is_some()
    );
}

#[tokio::test]
async fn with_tenant_instance_establishes_directory_context_and_returns_value() {
    let _guard = SHARED_TEST_LOCK.lock().await;
    ensure_test_home();
    project::instance::dispose_all().await;

    let project_dir = TempDir::new().expect("project dir");
    let expected = project_dir.path().to_string_lossy().to_string();
    let actual = with_tenant_instance(TenantScope::default(), expected.clone(), || {
        Box::pin(async { Ok(project::instance::directory()) })
    })
    .await
    .expect("tenant instance should execute the callback");

    assert_eq!(actual, expected);
    project::instance::dispose_all().await;
}
