use agent_database::GlobalDatabase;
use axum::{Extension, middleware};
use serde_json::json;
use std::{collections::BTreeMap, path::Path};
use vw_api_transport::id::TenantScope;
const TOKEN: &str = "fixture-security-authorized-token-over-32-chars";
const KEY: &str = "fixture-security-nested-key-753";
const COOKIE: &str = "fixture-security-cookie-951";
const NUMERIC_SECRET: u64 = 987654321012345;

fn safe(text: &str) {
    for secret in
        [TOKEN.to_string(), KEY.to_string(), COOKIE.to_string(), NUMERIC_SECRET.to_string()]
    {
        assert!(!text.contains(&secret), "credential reached observable output");
    }
}
fn tree(root: &Path) -> BTreeMap<std::path::PathBuf, Option<Vec<u8>>> {
    fn visit(root: &Path, dir: &Path, result: &mut BTreeMap<std::path::PathBuf, Option<Vec<u8>>>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                result.insert(path.strip_prefix(root).unwrap().to_owned(), None);
                visit(root, &path, result);
            } else {
                result.insert(
                    path.strip_prefix(root).unwrap().to_owned(),
                    Some(std::fs::read(path).unwrap()),
                );
            }
        }
    }
    let mut result = BTreeMap::new();
    visit(root, root, &mut result);
    result
}
#[tokio::test]
async fn cc04_api_security_registered_routes_deny_missing_gateway_auth_before_storage() {
    use axum::{
        body::{Body, to_bytes},
        http::Request,
    };
    use tower::ServiceExt;
    let root = tempfile::tempdir().unwrap();
    let db = GlobalDatabase::for_home(root.path()).unwrap();
    let before = tree(root.path());
    let app = crate::gateway::router::handler_router()
        .layer(Extension(db.clone()))
        .layer(Extension(TenantScope::new("p", "a").unwrap()))
        .layer(middleware::from_fn(|req, next| async {
            crate::gateway::middleware::basic_auth_middleware_with_credentials(
                req,
                next,
                Some("gateway-fixture"),
                "security",
            )
            .await
        }));
    for (method, path) in [
        ("GET", "/app-providers/claude"),
        ("GET", "/app-providers/claude/saved"),
        ("POST", "/app-providers/claude/saved"),
        ("PUT", "/app-providers/claude/saved"),
        ("PATCH", "/app-providers/claude/saved"),
        ("DELETE", "/app-providers/claude/saved"),
        ("POST", "/app-providers/claude/saved/duplicate"),
        ("GET", "/app-providers/claude/presentation/entries"),
        ("PUT", "/app-providers/claude/saved/presentation"),
        ("GET", "/app-providers/claude/saved/endpoints"),
        ("GET", "/app-providers/claude/saved/icon"),
        ("GET", "/universal-providers"),
        ("PUT", "/universal-providers/saved"),
        ("POST", "/universal-providers/saved/sync"),
        ("POST", "/app-providers/claude/config/paths"),
        ("POST", "/app-providers/claude/config/preview"),
        ("GET", "/app-providers/claude/config/snippet"),
        ("POST", "/app-providers/claude/saved/editor/open"),
        ("POST", "/app-providers/claude/saved/editor/save"),
        ("GET", "/app-providers/claude/mode/state"),
        ("POST", "/app-providers/claude/mode/apply"),
        ("POST", "/app-providers/claude/environment/diagnose"),
        ("POST", "/app-providers/claude/environment/clean"),
        ("POST", "/app-providers/claude/environment/restore"),
        ("POST", "/app-providers/pi/catalog/import"),
        ("POST", "/app-providers/operations"),
        ("GET", "/app-providers/operations/saved"),
        ("DELETE", "/app-providers/operations/saved"),
        ("POST", "/app-providers/claude-desktop/providers/import-claude"),
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(method)
                    .uri(path)
                    .header("content-type", "application/json")
                    .body(Body::from(json!({"secret":KEY}).to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status().as_u16(), 401, "{method} {path}");
        safe(
            &String::from_utf8(to_bytes(response.into_body(), 4096).await.unwrap().to_vec())
                .unwrap(),
        );
    }
    assert_eq!(tree(root.path()), before);
}
