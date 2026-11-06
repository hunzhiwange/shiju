use super::read;
use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode},
    middleware,
};
use sha2::{Digest, Sha256};
use std::{os::unix::fs::PermissionsExt, path::Path};
use tower::ServiceExt;

const TOKEN: &str = "fixture-token-for-a-single-explicit-principal";
fn router(path: &Path) -> Router {
    read::router_with_policy(Some(path.to_owned()))
        .layer(middleware::from_fn(crate::gateway::middleware::tenant_scope_middleware))
}
fn request(endpoint: &str, token: &str, company: &str, host: &str) -> Request<Body> {
    Request::builder()
        .uri(format!("/agent-apps/{endpoint}?targetHostId={host}&appId=claude"))
        .header("x-agent-apps-token", token)
        .header("x-platform-id", "p")
        .header("x-company-id", company)
        .body(Body::empty())
        .unwrap()
}
fn policy(root: &Path) -> std::path::PathBuf {
    std::fs::set_permissions(root, std::fs::Permissions::from_mode(0o700)).unwrap();
    let path = root.join("access.json");
    let value = serde_json::json!({
        "principals": [{"id":"reader", "token_sha256":hex::encode(Sha256::digest(TOKEN)),
            "grants":[{"platform_id":"p","company_id":"a","host_id":"host"},
                      {"platform_id":"p","company_id":"a","host_id":"offline"},
                      {"platform_id":"p","company_id":"a","host_id":"remote"}]}],
        "hosts": [{"id":"host","online":true,"binding":"local","home":root,
                   "search_paths":[root.join("bin")],"applications":root.join("SystemApplications")},
                  {"id":"offline","online":false,"binding":"local","home":root,
                   "search_paths":[],"applications":root.join("SystemApplications")},
                  {"id":"remote","online":true,"binding":"remote","home":root,
                   "search_paths":[],"applications":root.join("SystemApplications")}]
    });
    std::fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
    path
}
fn executable(root: &Path) {
    std::fs::create_dir(root.join("bin")).unwrap();
    let script = format!(
        "#!/bin/sh\nprintf probe >> '{}'\nprintf 'claude 2.3.4\\n'\n",
        root.join("probed").display()
    );
    std::fs::write(root.join("bin/claude"), script).unwrap();
    std::fs::set_permissions(root.join("bin/claude"), std::fs::Permissions::from_mode(0o700))
        .unwrap();
}
#[tokio::test]
async fn cc_apps_read_api_authorization_success_and_real_probe() {
    let root = tempfile::tempdir().unwrap();
    let path = policy(root.path());
    executable(root.path());
    let app = router(&path);
    let response = app.clone().oneshot(request("catalog", TOKEN, "a", "host")).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let value: serde_json::Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 65536).await.unwrap()).unwrap();
    assert_eq!(value["apps"].as_array().unwrap().len(), 14);
    assert!(!root.path().join("probed").exists());
    for endpoint in ["installations", "diagnostics", "versions"] {
        let response = app.clone().oneshot(request(endpoint, TOKEN, "a", "host")).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let value: serde_json::Value =
            serde_json::from_slice(&to_bytes(response.into_body(), 65536).await.unwrap()).unwrap();
        assert_eq!(value["installations"][0]["version"], "2.3.4");
        assert_eq!(value["installations"][0]["targetHostId"], "host");
    }
    assert_eq!(std::fs::read_to_string(root.path().join("probed")).unwrap(), "probeprobeprobe");
}
#[tokio::test]
async fn cc_apps_read_api_authorization_denials_never_run_probe() {
    let root = tempfile::tempdir().unwrap();
    let path = policy(root.path());
    executable(root.path());
    let app = router(&path);
    for endpoint in ["catalog", "installations", "diagnostics", "versions"] {
        for (token, company, host, status) in [
            (
                "wrong-token-with-sufficient-length-for-validation",
                "a",
                "host",
                StatusCode::UNAUTHORIZED,
            ),
            (TOKEN, "b", "host", StatusCode::FORBIDDEN),
            (TOKEN, "a", "unknown", StatusCode::FORBIDDEN),
            (TOKEN, "a", "offline", StatusCode::SERVICE_UNAVAILABLE),
            (TOKEN, "a", "remote", StatusCode::NOT_IMPLEMENTED),
        ] {
            let response =
                app.clone().oneshot(request(endpoint, token, company, host)).await.unwrap();
            assert_eq!(response.status(), status);
            let body = to_bytes(response.into_body(), 65536).await.unwrap();
            assert!(!String::from_utf8_lossy(&body).contains(TOKEN));
            assert!(!String::from_utf8_lossy(&body).contains(&root.path().display().to_string()));
        }
    }
    assert!(!root.path().join("probed").exists());
}

#[tokio::test]
async fn cc_apps_read_api_authorization_revocation_and_invalid_files_fail_closed() {
    let root = tempfile::tempdir().unwrap();
    let path = policy(root.path());
    executable(root.path());
    let app = router(&path);
    assert_eq!(
        app.clone().oneshot(request("catalog", TOKEN, "a", "host")).await.unwrap().status(),
        StatusCode::OK
    );
    let original = std::fs::read(&path).unwrap();
    let mut value: serde_json::Value = serde_json::from_slice(&original).unwrap();
    value["principals"][0]["grants"] = serde_json::json!([]);
    std::fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
    assert_eq!(
        app.clone().oneshot(request("diagnostics", TOKEN, "a", "host")).await.unwrap().status(),
        StatusCode::FORBIDDEN
    );
    std::fs::write(&path, &original).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
    assert_eq!(
        app.clone().oneshot(request("diagnostics", TOKEN, "a", "host")).await.unwrap().status(),
        StatusCode::SERVICE_UNAVAILABLE
    );
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
    std::fs::write(&path, b"{}").unwrap();
    assert_eq!(
        app.clone().oneshot(request("diagnostics", TOKEN, "a", "host")).await.unwrap().status(),
        StatusCode::SERVICE_UNAVAILABLE
    );
    std::fs::write(&path, &original).unwrap();
    let link = root.path().join("link.json");
    std::os::unix::fs::symlink(&path, &link).unwrap();
    assert_eq!(
        router(&link).oneshot(request("diagnostics", TOKEN, "a", "host")).await.unwrap().status(),
        StatusCode::SERVICE_UNAVAILABLE
    );
    std::fs::remove_file(&path).unwrap();
    assert_eq!(
        app.oneshot(request("diagnostics", TOKEN, "a", "host")).await.unwrap().status(),
        StatusCode::SERVICE_UNAVAILABLE
    );
    assert!(!root.path().join("probed").exists());
}

#[tokio::test]
async fn cc_apps_read_api_authorization_tenant_query_and_token_boundaries() {
    let root = tempfile::tempdir().unwrap();
    let path = policy(root.path());
    executable(root.path());
    let app = router(&path);
    let mut req = request("diagnostics", TOKEN, "a", "host");
    req.headers_mut().insert("x-platform-id", "other".parse().unwrap());
    assert_eq!(app.clone().oneshot(req).await.unwrap().status(), StatusCode::FORBIDDEN);
    let mut req = request("diagnostics", TOKEN, "a", "host");
    req.headers_mut().remove("x-company-id");
    assert_eq!(app.clone().oneshot(req).await.unwrap().status(), StatusCode::BAD_REQUEST);
    let mut req = request("diagnostics", TOKEN, "a", "host");
    req.headers_mut().append("x-agent-apps-token", TOKEN.parse().unwrap());
    assert_eq!(app.clone().oneshot(req).await.unwrap().status(), StatusCode::UNAUTHORIZED);
    let mut req = request("diagnostics", TOKEN, "a", "host");
    req.headers_mut().remove("x-agent-apps-token");
    assert_eq!(app.clone().oneshot(req).await.unwrap().status(), StatusCode::UNAUTHORIZED);
    for query in [
        "targetHostId=host&appId=bogus",
        "targetHostId=host&appId=claude&path=/tmp/other",
        "targetHostId=host&appId=claude&command=touch",
        "targetHostId=host",
        "appId=claude",
    ] {
        let mut req = request("diagnostics", TOKEN, "a", "host");
        *req.uri_mut() = format!("/agent-apps/diagnostics?{query}").parse().unwrap();
        assert_eq!(app.clone().oneshot(req).await.unwrap().status(), StatusCode::BAD_REQUEST);
    }
    assert!(!root.path().join("probed").exists());
}

#[tokio::test]
async fn cc_apps_read_api_authorization_reports_actual_failure_and_desktop_presence() {
    let root = tempfile::tempdir().unwrap();
    let path = policy(root.path());
    executable(root.path());
    std::fs::write(
        root.path().join("bin/claude"),
        "#!/bin/sh\nprintf 'private stderr' >&2\nexit 7\n",
    )
    .unwrap();
    let app = router(&path);
    let response = app.clone().oneshot(request("diagnostics", TOKEN, "a", "host")).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let value: serde_json::Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 65536).await.unwrap()).unwrap();
    assert_eq!(value["installations"][0]["runtimeStatus"]["status"], "exited_with_error");
    assert_eq!(value["installations"][0]["runtimeStatus"]["exitCode"], 7);
    std::fs::remove_file(root.path().join("bin/claude")).unwrap();
    let response = app.clone().oneshot(request("installations", TOKEN, "a", "host")).await.unwrap();
    let value: serde_json::Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 65536).await.unwrap()).unwrap();
    assert_eq!(value["installations"], serde_json::json!([]));
    std::fs::create_dir_all(root.path().join("Applications/Claude.app")).unwrap();
    let mut req = request("diagnostics", TOKEN, "a", "host");
    *req.uri_mut() =
        "/agent-apps/diagnostics?targetHostId=host&appId=claude-desktop".parse().unwrap();
    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let value: serde_json::Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 65536).await.unwrap()).unwrap();
    assert_eq!(value["installations"], serde_json::json!([]));
    assert_eq!(value["desktop"]["supported"], true);
    #[cfg(target_os = "macos")]
    assert_eq!(value["desktop"]["appBundles"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn cc_apps_read_api_authorization_production_registration_and_default_deny() {
    // Process-local environment injection avoids mutating global env in parallel tests.
    if let Some(path) = std::env::var_os("SHIJU_TASK33_CHILD_POLICY") {
        assert_eq!(std::env::var_os("SHIJU_AGENT_APPS_ACCESS_FILE"), Some(path.clone()));
        let app = crate::gateway::router::handler_router()
            .layer(middleware::from_fn(crate::gateway::middleware::tenant_scope_middleware))
            .layer(middleware::from_fn(|req, next| async {
                crate::gateway::middleware::basic_auth_middleware_with_credentials(
                    req,
                    next,
                    Some("secret"),
                    "test",
                )
                .await
            }));
        let req = request("versions", TOKEN, "a", "host");
        assert_eq!(app.clone().oneshot(req).await.unwrap().status(), StatusCode::UNAUTHORIZED);
        let mut req = request("versions", TOKEN, "a", "host");
        req.headers_mut().insert("authorization", "Basic dGVzdDpzZWNyZXQ=".parse().unwrap());
        let response = app.oneshot(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let value: serde_json::Value =
            serde_json::from_slice(&to_bytes(response.into_body(), 65536).await.unwrap()).unwrap();
        assert_eq!(value["installations"][0]["version"], "2.3.4");
        return;
    }
    let root = tempfile::tempdir().unwrap();
    let path = policy(root.path());
    executable(root.path());
    let status = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "gateway::api::handlers::agent_apps::read_tests::cc_apps_read_api_authorization_production_registration_and_default_deny", "--nocapture"])
        .env("SHIJU_TASK33_CHILD_POLICY", &path).env("SHIJU_AGENT_APPS_ACCESS_FILE", &path)
        .status().unwrap();
    assert!(status.success());
    assert_eq!(std::fs::read_to_string(root.path().join("probed")).unwrap(), "probe");
    let app = read::router_with_policy(None)
        .layer(middleware::from_fn(crate::gateway::middleware::tenant_scope_middleware));
    assert_eq!(
        app.oneshot(request("catalog", TOKEN, "a", "host")).await.unwrap().status(),
        StatusCode::SERVICE_UNAVAILABLE
    );
}

#[tokio::test]
async fn cc_apps_read_api_authorization_invalid_registration_never_probes() {
    let root = tempfile::tempdir().unwrap();
    let path = policy(root.path());
    executable(root.path());
    let original: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    for kind in 0..6 {
        let mut value = original.clone();
        match kind {
            0 => {
                let host = value["hosts"][0].clone();
                value["hosts"].as_array_mut().unwrap().push(host);
            }
            1 => {
                let principal = value["principals"][0].clone();
                value["principals"].as_array_mut().unwrap().push(principal);
            }
            2 => {
                value["hosts"][0]["home"] = serde_json::json!("relative");
            }
            3 => {
                value["principals"][0]["grants"][0]["host_id"] = serde_json::json!("unregistered");
            }
            4 => {
                value["hosts"][0]["command"] = serde_json::json!("touch");
            }
            _ => {
                value["hosts"][0]["search_paths"] = serde_json::json!(["relative"]);
            }
        }
        std::fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
        assert_eq!(
            router(&path)
                .oneshot(request("diagnostics", TOKEN, "a", "host"))
                .await
                .unwrap()
                .status(),
            StatusCode::SERVICE_UNAVAILABLE
        );
    }
    std::fs::write(&path, vec![b' '; 65537]).unwrap();
    assert_eq!(
        router(&path).oneshot(request("diagnostics", TOKEN, "a", "host")).await.unwrap().status(),
        StatusCode::SERVICE_UNAVAILABLE
    );
    std::fs::write(&path, serde_json::to_vec(&original).unwrap()).unwrap();
    std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o777)).unwrap();
    assert_eq!(
        router(&path).oneshot(request("diagnostics", TOKEN, "a", "host")).await.unwrap().status(),
        StatusCode::SERVICE_UNAVAILABLE
    );
    std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    assert!(!root.path().join("probed").exists());
}

#[tokio::test]
async fn cc_apps_read_api_authorization_managed_package_source_matches_lifecycle() {
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let path = policy(root.path());
    let prefix = root.path().join("prefix");
    let package = prefix.join("lib/node_modules/@anthropic-ai/claude-code");
    std::fs::create_dir_all(package.join("bin")).unwrap();
    std::fs::create_dir_all(prefix.join("bin")).unwrap();
    std::fs::write(package.join("package.json"), r#"{"name":"@anthropic-ai/claude-code"}"#)
        .unwrap();
    let binary = package.join("bin/claude");
    std::fs::write(&binary, "#!/bin/sh\nprintf 'claude 2.3.4\\n'\n").unwrap();
    std::fs::set_permissions(&binary, std::fs::Permissions::from_mode(0o700)).unwrap();
    std::os::unix::fs::symlink(&binary, prefix.join("bin/claude")).unwrap();
    let mut value: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    value["hosts"][0]["search_paths"] = serde_json::json!([prefix.join("bin")]);
    value["hosts"][0]["lifecycle"] =
        serde_json::json!({"prefix":prefix,"npm":prefix.join("bin/npm")});
    std::fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
    let app = router(&path);
    for valid_manifest in [true, false] {
        if !valid_manifest {
            std::fs::write(package.join("package.json"), r#"{"name":"different-package"}"#)
                .unwrap();
        }
        for endpoint in ["installations", "diagnostics", "versions"] {
            let response =
                app.clone().oneshot(request(endpoint, TOKEN, "a", "host")).await.unwrap();
            assert_eq!(response.status(), StatusCode::OK);
            let response: vw_api_types::agent_apps::AppReadResponse =
                serde_json::from_slice(&to_bytes(response.into_body(), 65536).await.unwrap())
                    .unwrap();
            assert_eq!(response.installations.len(), 1);
            assert_eq!(
                response.installations[0].source,
                if valid_manifest {
                    vw_api_types::agent_apps::AppInstallationSource::System
                } else {
                    vw_api_types::agent_apps::AppInstallationSource::Unknown
                }
            );
        }
    }
}
