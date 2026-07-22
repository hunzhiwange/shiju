use super::*;
use axum::Json;
use axum::body::{Body, to_bytes};
use axum::extract::Path;
use axum::http::{Method, Request, StatusCode};
use std::path::{Path as FsPath, PathBuf};
use std::sync::OnceLock;
use tower::ServiceExt;

async fn env_lock() -> tokio::sync::MutexGuard<'static, ()> {
    static LOCK: OnceLock<tokio::sync::Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| tokio::sync::Mutex::new(())).lock().await
}

fn test_home() -> &'static PathBuf {
    static HOME: OnceLock<PathBuf> = OnceLock::new();
    HOME.get_or_init(|| {
        let home = std::env::temp_dir()
            .join(format!("vw-agent-auth-handler-tests-{}", std::process::id()));
        let auth_file = vw_config_types::paths::home_config_dir(&home).join("auth.json");
        std::fs::create_dir_all(auth_file.parent().expect("auth.json should have parent"))
            .expect("should create auth test directory");
        std::fs::write(&auth_file, "{}").expect("should seed auth file");
        unsafe {
            std::env::set_var("VIBEWINDOW_TEST_HOME", &home);
        }
        home
    })
}

fn auth_file() -> PathBuf {
    let _ = test_home();
    let paths = crate::app::agent::global::paths();
    vw_shared::auth::store::resolve_filepath(&paths.home, &paths.data)
}

fn reset_auth_store() {
    let auth_file = auth_file();
    if auth_file.is_dir() {
        std::fs::remove_dir_all(&auth_file).expect("should remove auth directory");
    }
    std::fs::create_dir_all(auth_file.parent().expect("auth.json should have parent"))
        .expect("should create auth parent directory");
    #[cfg(unix)]
    set_mode(&auth_file, 0o600);
    std::fs::write(auth_file, "{}").expect("should reset auth file");
}

fn api_info(key: &str) -> auth::Info {
    auth::Info::Api(auth::ApiInfo { key: key.to_string() })
}

fn json_request(method: Method, uri: &str, body: serde_json::Value) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header(axum::http::header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .expect("request should build")
}

async fn response_json(response: axum::response::Response) -> serde_json::Value {
    let body = to_bytes(response.into_body(), usize::MAX).await.expect("body should be readable");
    serde_json::from_slice(&body).expect("response should be json")
}

#[cfg(unix)]
fn set_mode(path: &FsPath, mode: u32) {
    use std::os::unix::fs::PermissionsExt;

    if let Ok(metadata) = std::fs::metadata(path) {
        let mut permissions = metadata.permissions();
        permissions.set_mode(mode);
        std::fs::set_permissions(path, permissions).expect("should update file permissions");
    }
}

#[cfg(unix)]
fn replace_auth_file_with_directory() {
    let auth_file = auth_file();
    let _ = std::fs::remove_file(&auth_file);
    std::fs::create_dir_all(&auth_file).expect("should replace auth file with directory");
}

#[cfg(unix)]
fn restore_auth_file_from_directory() {
    let auth_file = auth_file();
    let _ = std::fs::remove_dir_all(&auth_file);
    reset_auth_store();
}

#[tokio::test]
async fn auth_set_persists_credentials_and_returns_true() {
    let _guard = env_lock().await;
    reset_auth_store();

    let response = auth_set(Path("provider.openai".to_string()), Json(api_info("sk-test-abc123")))
        .await
        .expect("setting auth should succeed");

    assert!(response.0);
    match auth::get("provider.openai") {
        Some(auth::Info::Api(api)) => assert_eq!(api.key, "sk-test-abc123"),
        _ => panic!("expected stored api credentials"),
    }
}

#[tokio::test]
async fn auth_set_overwrites_existing_provider_credentials() {
    let _guard = env_lock().await;
    reset_auth_store();
    auth::set("provider.openai", &api_info("sk-old")).expect("should seed existing auth");

    let response = auth_set(Path("provider.openai".to_string()), Json(api_info("sk-new")))
        .await
        .expect("updating auth should succeed");

    assert!(response.0);
    match auth::get("provider.openai") {
        Some(auth::Info::Api(api)) => assert_eq!(api.key, "sk-new"),
        _ => panic!("expected updated api credentials"),
    }
}

#[tokio::test]
async fn auth_remove_deletes_existing_provider_and_returns_true() {
    let _guard = env_lock().await;
    reset_auth_store();
    auth::set("provider.openai", &api_info("sk-remove")).expect("should seed auth");
    auth::set(
        "provider.github",
        &auth::Info::Wellknown(auth::WellKnownInfo {
            key: "wk-key".to_string(),
            token: "wk-token".to_string(),
        }),
    )
    .expect("should seed second provider auth");

    let response = auth_remove(Path("provider.openai".to_string()))
        .await
        .expect("removing auth should succeed");

    assert!(response.0);
    assert!(auth::get("provider.openai").is_none());
    assert!(auth::get("provider.github").is_some());
}

#[tokio::test]
async fn auth_remove_missing_provider_keeps_existing_credentials() {
    let _guard = env_lock().await;
    reset_auth_store();
    auth::set("provider.github", &api_info("sk-still-there")).expect("should seed auth");

    let response = auth_remove(Path("provider.missing".to_string()))
        .await
        .expect("removing missing auth should still succeed");

    assert!(response.0);
    match auth::get("provider.github") {
        Some(auth::Info::Api(api)) => assert_eq!(api.key, "sk-still-there"),
        _ => panic!("expected untouched provider auth"),
    }
}

#[cfg(unix)]
#[tokio::test]
async fn auth_set_maps_write_errors_to_bad_request() {
    let _guard = env_lock().await;
    reset_auth_store();
    replace_auth_file_with_directory();

    let err = auth_set(Path("provider.openai".to_string()), Json(api_info("sk-denied")))
        .await
        .expect_err("setting auth should fail when auth path is a directory");

    assert_eq!(err.status, StatusCode::BAD_REQUEST);
    assert!(
        err.message.to_ascii_lowercase().contains("directory"),
        "unexpected error message: {}",
        err.message
    );

    restore_auth_file_from_directory();
}

#[cfg(unix)]
#[tokio::test]
async fn auth_remove_maps_write_errors_to_bad_request() {
    let _guard = env_lock().await;
    reset_auth_store();
    auth::set("provider.openai", &api_info("sk-denied")).expect("should seed auth");
    replace_auth_file_with_directory();

    let err = auth_remove(Path("provider.openai".to_string()))
        .await
        .expect_err("removing auth should fail when auth path is a directory");

    assert_eq!(err.status, StatusCode::BAD_REQUEST);
    assert!(
        err.message.to_ascii_lowercase().contains("directory"),
        "unexpected error message: {}",
        err.message
    );

    restore_auth_file_from_directory();
}

#[test]
fn router_builds_without_panic() {
    let _r: Router<()> = router();
}

#[tokio::test]
async fn router_put_persists_credentials_and_returns_true() {
    let _guard = env_lock().await;
    reset_auth_store();
    let app: Router<()> = router();

    let response = app
        .oneshot(json_request(
            Method::PUT,
            "/auth/provider.route",
            serde_json::json!({"type": "api", "key": "sk-route"}),
        ))
        .await
        .expect("route should respond");

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response_json(response).await, serde_json::json!(true));
    match auth::get("provider.route") {
        Some(auth::Info::Api(api)) => assert_eq!(api.key, "sk-route"),
        _ => panic!("expected route to store api credentials"),
    }
}

#[tokio::test]
async fn router_delete_removes_credentials_and_returns_true() {
    let _guard = env_lock().await;
    reset_auth_store();
    auth::set("provider.route", &api_info("sk-route")).expect("should seed auth");
    let app: Router<()> = router();

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::DELETE)
                .uri("/auth/provider.route")
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("route should respond");

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response_json(response).await, serde_json::json!(true));
    assert!(auth::get("provider.route").is_none());
}

#[tokio::test]
async fn router_put_rejects_invalid_auth_info_body() {
    let _guard = env_lock().await;
    reset_auth_store();
    let app: Router<()> = router();

    let response = app
        .oneshot(json_request(
            Method::PUT,
            "/auth/provider.route",
            serde_json::json!({"type": "unknown", "key": "sk-route"}),
        ))
        .await
        .expect("route should respond");

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert!(auth::get("provider.route").is_none());
}

#[tokio::test]
async fn router_rejects_unsupported_method_and_missing_provider_path() {
    let app: Router<()> = router();

    let method_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/auth/provider.route")
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("route should respond");
    assert_eq!(method_response.status(), StatusCode::METHOD_NOT_ALLOWED);

    let path_response = app
        .oneshot(
            Request::builder()
                .method(Method::DELETE)
                .uri("/auth")
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("route should respond");
    assert_eq!(path_response.status(), StatusCode::NOT_FOUND);
}
