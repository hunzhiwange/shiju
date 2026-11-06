use axum::{
    body::Body,
    http::{Request, StatusCode},
    middleware,
};
use tower::ServiceExt;
#[tokio::test]
async fn cc_apps_lifecycle_api_scope_missing_identity_rejected() {
    let app = super::lifecycle::router_with_policy(None)
        .layer(middleware::from_fn(crate::gateway::middleware::tenant_scope_middleware));
    let response = app.oneshot(Request::builder().method("POST")
        .uri("/agent-apps/actions/preview?targetHostId=host")
        .header("x-platform-id", "p").header("x-company-id", "a")
        .header("content-type", "application/json")
        .body(Body::from(r#"{"appId":"codex","action":"install","target":{"kind":"environment","environment":{"kind":"macos"}}}"#)).unwrap()).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

use axum::{Router, body::to_bytes};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
};
static WRITES: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
const TOKEN: &str = "lifecycle-fixture-token-with-explicit-write-grants";
fn script(path: &Path, value: &str) {
    std::fs::write(path, value).unwrap();
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700)).unwrap();
}
struct Fixture {
    root: tempfile::TempDir,
    app: Router,
    policy: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
        let prefix = root.path().join("prefix");
        std::fs::create_dir_all(prefix.join("bin")).unwrap();
        script(&prefix.join("bin/node"), "#!/bin/sh\necho v24.0.0\n");
        script(
            &prefix.join("bin/npm"),
            &format!(
                r#"#!/bin/sh
printf '%s\n' "$@" >> '{root}/argv'
printf '%s\n' "$PWD" "$HOME" "$npm_config_prefix" "$npm_config_engine_strict" >> '{root}/environment'
if [ -f '{root}/delay' ]; then touch '{root}/started'; sleep 20; touch '{root}/escaped'; fi
if [ -f '{root}/fail' ]; then echo secret-fixture-payload >&2; exit 7; fi
if [ "$1" = uninstall ]; then rm -f '{prefix}/bin/codex'; exit 0; fi
mkdir -p '{prefix}/lib/node_modules/@openai/codex'
printf '{{"name":"@openai/codex"}}' > '{prefix}/lib/node_modules/@openai/codex/package.json'
printf '#!/bin/sh\necho codex 1.2.3\n' > '{prefix}/lib/node_modules/@openai/codex/cli'
chmod 700 '{prefix}/lib/node_modules/@openai/codex/cli'
ln -sf '{prefix}/lib/node_modules/@openai/codex/cli' '{prefix}/bin/codex'
"#,
                root = root.path().display(),
                prefix = prefix.display()
            ),
        );
        let policy = root.path().join("policy.json");
        let p = json!({"principals":[{"id":"writer","token_sha256":hex::encode(Sha256::digest(TOKEN)),"grants":[
        {"platform_id":"p","company_id":"a","host_id":"host","actions":["install","update","uninstall"]},
        {"platform_id":"p","company_id":"b","host_id":"host","actions":["install","update","uninstall"]}]}],
        "hosts":[{"id":"host","online":true,"binding":"local","home":root.path(),"search_paths":[prefix.join("bin")],"applications":root.path().join("Applications"),"lifecycle":{"prefix":prefix,"npm":prefix.join("bin/npm")}}]});
        std::fs::write(&policy, serde_json::to_vec(&p).unwrap()).unwrap();
        std::fs::set_permissions(&policy, std::fs::Permissions::from_mode(0o600)).unwrap();
        let app = super::lifecycle::router_with_policy(Some(policy.clone()))
            .merge(super::read::router_with_policy(Some(policy.clone())))
            .layer(middleware::from_fn(crate::gateway::middleware::tenant_scope_middleware));
        Self { root, app, policy }
    }
    async fn call(
        &self,
        method: &str,
        path: &str,
        company: &str,
        value: Value,
    ) -> (StatusCode, Value) {
        let response = self
            .app
            .clone()
            .oneshot(
                Request::builder()
                    .method(method)
                    .uri(format!(
                        "/agent-apps/{path}{}targetHostId=host",
                        if path.contains('?') { "&" } else { "?" }
                    ))
                    .header("x-agent-apps-token", TOKEN)
                    .header("x-platform-id", "p")
                    .header("x-company-id", company)
                    .header("content-type", "application/json")
                    .body(Body::from(value.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        let code = response.status();
        let bytes = to_bytes(response.into_body(), 65536).await.unwrap();
        assert!(!String::from_utf8_lossy(&bytes).contains("secret-fixture-payload"));
        (code, serde_json::from_slice(&bytes).unwrap_or(Value::Null))
    }
    async fn preview(&self, action: &str, target: Value) -> Value {
        let (code, body) = self
            .call(
                "POST",
                "actions/preview",
                "a",
                json!({"appId":"codex","action":action,"target":target}),
            )
            .await;
        assert_eq!(code, StatusCode::OK, "{body}");
        body
    }
    async fn submit(&self, preview: &Value) -> String {
        let (code, body) = self
            .call(
                "POST",
                "actions",
                "a",
                json!({"previewId":preview["previewId"],"confirmed":true}),
            )
            .await;
        assert_eq!(code, StatusCode::ACCEPTED, "{body}");
        body["operationId"].as_str().unwrap().into()
    }
    async fn wait(&self, id: &str) -> Value {
        for _ in 0..100 {
            let (code, body) =
                self.call("GET", &format!("operations/{id}/events"), "a", Value::Null).await;
            assert_eq!(code, StatusCode::OK);
            if body["phase"] == "terminal" {
                return body;
            }
        }
        panic!("operation did not terminate")
    }
}
fn environment() -> Value {
    json!({"kind":"environment","environment":{"kind":if cfg!(target_os="macos") {"macos"} else {"linux"}}})
}
#[tokio::test]
async fn cc_apps_lifecycle_api_scope_install_update_uninstall_and_tenant_isolation() {
    let _guard = WRITES.lock().await;
    let f = Fixture::new();
    let p = f.preview("install", environment()).await;
    assert!(!f.root.path().join("argv").exists());
    assert_eq!(
        f.call("POST", "actions", "b", json!({"previewId":p["previewId"],"confirmed":true}))
            .await
            .0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        f.call("POST", "actions", "a", json!({"previewId":p["previewId"],"confirmed":false}))
            .await
            .0,
        StatusCode::CONFLICT
    );
    let id = f.submit(&p).await;
    for method in ["GET", "DELETE"] {
        assert_eq!(
            f.call(method, &format!("operations/{id}"), "b", Value::Null).await.0,
            StatusCode::NOT_FOUND
        );
    }
    assert_eq!(
        f.call("GET", &format!("operations/{id}/events"), "b", Value::Null).await.0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(f.wait(&id).await["status"], "completed");
    assert_eq!(
        f.call("POST", "actions", "a", json!({"previewId":p["previewId"],"confirmed":true}))
            .await
            .0,
        StatusCode::NOT_FOUND
    );
    let (code, list) = f.call("GET", "installations?appId=codex", "a", Value::Null).await;
    assert_eq!(code, StatusCode::OK);
    let inst = list["installations"][0]["id"].clone();
    assert!(inst.is_string());
    let target = json!({"kind":"installation","installationId":inst});
    let p = f.preview("update", target.clone()).await;
    let id = f.submit(&p).await;
    assert_eq!(f.wait(&id).await["status"], "completed");
    let p = f.preview("uninstall", target).await;
    let id = f.submit(&p).await;
    assert_eq!(f.wait(&id).await["status"], "completed");
    assert!(!f.root.path().join("prefix/bin/codex").exists());
    let argv = std::fs::read_to_string(f.root.path().join("argv")).unwrap();
    assert!(argv.contains("@openai/codex@latest"));
    assert!(argv.contains("uninstall"));
    let environment = std::fs::read_to_string(f.root.path().join("environment")).unwrap();
    assert!(environment.contains(&f.root.path().join("prefix").display().to_string()));
    assert!(environment.contains("true"));
}
#[tokio::test]
async fn cc_apps_lifecycle_api_scope_failure_cancel_and_revocation() {
    let _guard = WRITES.lock().await;
    let f = Fixture::new();
    std::fs::write(f.root.path().join("fail"), "").unwrap();
    let p = f.preview("install", environment()).await;
    let id = f.submit(&p).await;
    assert_eq!(f.wait(&id).await["status"], "failed");
    std::fs::remove_file(f.root.path().join("fail")).unwrap();
    std::fs::write(f.root.path().join("delay"), "").unwrap();
    let p = f.preview("install", environment()).await;
    let id = f.submit(&p).await;
    for _ in 0..200 {
        if f.root.path().join("started").exists() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    assert!(f.root.path().join("started").exists());
    let (code, body) = f.call("DELETE", &format!("operations/{id}"), "a", Value::Null).await;
    assert_eq!(code, StatusCode::OK);
    assert_eq!(body["status"], "cancelled");
    assert!(!f.root.path().join("escaped").exists());
    let mut policy: Value = serde_json::from_slice(&std::fs::read(&f.policy).unwrap()).unwrap();
    policy["principals"][0]["grants"][0]["actions"] = json!([]);
    std::fs::write(&f.policy, serde_json::to_vec(&policy).unwrap()).unwrap();
    assert_eq!(
        f.call(
            "POST",
            "actions/preview",
            "a",
            json!({"appId":"codex","action":"install","target":environment()})
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        f.call("DELETE", &format!("operations/{id}"), "a", Value::Null).await.0,
        StatusCode::FORBIDDEN
    );
}
#[tokio::test]
async fn cc_apps_lifecycle_api_scope_rejects_unsupported_and_changed_plan() {
    let _guard = WRITES.lock().await;
    let f = Fixture::new();
    for body in [
        json!({"appId":"claude-desktop","action":"install","target":environment()}),
        json!({"appId":"codex","action":"update","target":environment()}),
        json!({"appId":"codex","action":"install","target":{"kind":"environment","environment":{"kind":"windows"}}}),
    ] {
        assert_eq!(
            f.call("POST", "actions/preview", "a", body).await.0,
            StatusCode::UNPROCESSABLE_ENTITY
        );
    }
    assert_eq!(
        f.call(
            "POST",
            "actions/preview",
            "a",
            json!({"appId":"codex","action":"execute","target":environment()})
        )
        .await
        .0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    let p = f.preview("install", environment()).await;
    script(&f.root.path().join("prefix/bin/codex"), "#!/bin/sh\necho codex 9.0.0\n");
    let id = f.submit(&p).await;
    assert_eq!(f.wait(&id).await["status"], "failed");
    assert!(!f.root.path().join("argv").exists());
}

#[tokio::test]
async fn cc_apps_lifecycle_api_scope_read_grants_do_not_grant_writes_or_accept_paths() {
    let f = Fixture::new();
    let mut policy: Value = serde_json::from_slice(&std::fs::read(&f.policy).unwrap()).unwrap();
    policy["principals"][0]["grants"][0].as_object_mut().unwrap().remove("actions");
    std::fs::write(&f.policy, serde_json::to_vec(&policy).unwrap()).unwrap();
    assert_eq!(
        f.call("GET", "installations?appId=codex", "a", Value::Null).await.0,
        StatusCode::OK
    );
    for action in ["install", "update", "uninstall"] {
        assert_eq!(
            f.call(
                "POST",
                "actions/preview",
                "a",
                json!({"appId":"codex","action":action,"target":environment()})
            )
            .await
            .0,
            StatusCode::FORBIDDEN
        );
    }
    for field in ["command", "args", "env", "path", "targetHostId"] {
        let mut body = json!({"appId":"codex","action":"install","target":environment()});
        body[field] = json!("untrusted");
        assert_eq!(
            f.call("POST", "actions/preview", "a", body).await.0,
            StatusCode::UNPROCESSABLE_ENTITY
        );
    }
    assert!(!f.root.path().join("argv").exists());
}
