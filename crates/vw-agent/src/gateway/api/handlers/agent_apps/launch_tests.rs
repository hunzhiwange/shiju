use super::*;
use axum::{
    body::{Body, to_bytes},
    http::Request as HttpRequest,
    middleware,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::os::unix::fs::PermissionsExt;
use tower::ServiceExt;
const TOKEN: &str = "fixture-launch-api-authorized-token";
fn terminal(preferred: Option<&str>, _: &Path, material: &LaunchMaterial) -> Result<(), String> {
    assert_eq!(preferred, Some("terminal"));
    attached_terminal(preferred, Path::new("/bin/sh"), material)
}
fn failing_terminal(_: Option<&str>, _: &Path, _: &LaunchMaterial) -> Result<(), String> {
    Err("fixed failure".into())
}
async fn call(app: &Router, token: &str, value: Value) -> (StatusCode, Value) {
    let response = app
        .clone()
        .oneshot(
            HttpRequest::builder()
                .method("POST")
                .uri("/agent-apps/launch?targetHostId=h")
                .header("content-type", "application/json")
                .header("x-agent-apps-token", token)
                .header("x-platform-id", "p")
                .header("x-company-id", "c")
                .body(Body::from(value.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 65536).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap_or(Value::Null))
}
#[tokio::test]
async fn cc_apps_launch_api_target_scope_accepts_and_rejects_before_terminal() {
    let root = tempfile::tempdir().unwrap();
    std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    let cwd = root.path().join("workdir");
    std::fs::create_dir(&cwd).unwrap();
    let bin = root.path().join("bin");
    std::fs::create_dir(&bin).unwrap();
    let program = bin.join("codex");
    std::fs::write(&program, "#!/bin/sh\nif [ \"$1\" = --version ]; then echo 'codex 1.0.0'; exit 0; fi\ntest -t 0 && test -t 1 || exit 30\nprintf '%s' \"$HOME\" > home-proof\npwd > cwd-proof\nprintf '%s' \"$0\" > program-proof\nprintf '[\"%s\"]' \"$1\" > accepted\n").unwrap();
    std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o700)).unwrap();
    let policy = root.path().join("policy.json");
    let mut value = json!({"principals":[{"id":"a","token_sha256":hex::encode(Sha256::digest(TOKEN)),"grants":[{"platform_id":"p","company_id":"c","host_id":"h","launch":true}]}],"hosts":[{"id":"h","online":true,"binding":"local","home":root.path(),"search_paths":[bin],"applications":root.path()}]});
    std::fs::write(&policy, value.to_string()).unwrap();
    std::fs::set_permissions(&policy, std::fs::Permissions::from_mode(0o600)).unwrap();
    let db = GlobalDatabase::for_home(root.path()).unwrap();
    let tenant = vw_api_types::id::TenantScope::new("p", "c").unwrap();
    GatewaySettingsStore::new(db.clone())
        .unwrap()
        .upsert(&tenant, "agent_apps_host", "h", r#"{"preferredTerminal":"terminal"}"#)
        .unwrap();
    let app = router_with_terminal(Some(policy.clone()), terminal)
        .layer(Extension(db.clone()))
        .layer(middleware::from_fn(crate::gateway::middleware::tenant_scope_middleware));
    let observed = vw_agent_apps::apps::discovery::read::read(
        vw_api_types::agent_apps::AppId::Codex,
        &vw_agent_apps::apps::discovery::read::LocalReadEnvironment {
            host_id: "h".into(),
            home: root.path().into(),
            search_paths: vec![bin],
            applications: root.path().into(),
        },
    )
    .unwrap();
    let request = json!({"appId":"codex","installationId":observed.installations[0].id,"cwd":cwd,"args":["hello"]});
    for (field, replacement, status) in [
        ("cwd", json!("/"), StatusCode::UNPROCESSABLE_ENTITY),
        ("installationId", json!("forged"), StatusCode::UNPROCESSABLE_ENTITY),
        (
            "args",
            json!(["--dangerously-bypass-approvals-and-sandbox"]),
            StatusCode::UNPROCESSABLE_ENTITY,
        ),
        ("command", json!("touch injected"), StatusCode::UNPROCESSABLE_ENTITY),
        ("accountId", json!("wrong-account"), StatusCode::FORBIDDEN),
        ("providerId", json!("unknown"), StatusCode::FORBIDDEN),
    ] {
        let mut invalid = request.clone();
        invalid[field] = replacement;
        assert_eq!(call(&app, TOKEN, invalid).await.0, status, "{field}");
        assert!(!cwd.join("accepted").exists());
    }
    assert_eq!(
        call(&app, "invalid-token-with-sufficient-length", request.clone()).await.0,
        StatusCode::UNAUTHORIZED
    );
    value["principals"][0]["grants"][0]["launch"] = json!(false);
    std::fs::write(&policy, value.to_string()).unwrap();
    assert_eq!(call(&app, TOKEN, request.clone()).await.0, StatusCode::FORBIDDEN);
    assert!(!cwd.join("accepted").exists());
    value["principals"][0]["grants"][0]["launch"] = json!(true);
    std::fs::write(&policy, value.to_string()).unwrap();
    let mut failed = request.clone();
    failed["args"] = json!(["fail-terminal"]);
    let failed_app = router_with_terminal(Some(policy.clone()), failing_terminal)
        .layer(Extension(db))
        .layer(middleware::from_fn(crate::gateway::middleware::tenant_scope_middleware));
    assert_eq!(
        call(&failed_app, TOKEN, failed).await,
        (StatusCode::SERVICE_UNAVAILABLE, Value::Null)
    );
    assert!(!cwd.join("accepted").exists());
    let (status, result) = call(&app, TOKEN, request.clone()).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(result["installationId"], request["installationId"]);
    for _ in 0..200 {
        if std::fs::read_to_string(cwd.join("accepted")).is_ok_and(|s| s == "[\"hello\"]") {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    assert_eq!(std::fs::read_to_string(cwd.join("accepted")).unwrap(), "[\"hello\"]");
    assert_eq!(
        std::fs::read_to_string(cwd.join("home-proof")).unwrap(),
        root.path().canonicalize().unwrap().to_str().unwrap()
    );
    assert_eq!(
        std::fs::read_to_string(cwd.join("cwd-proof")).unwrap().trim(),
        cwd.canonicalize().unwrap().to_str().unwrap()
    );
    assert_eq!(
        std::fs::read_to_string(cwd.join("program-proof")).unwrap(),
        program.canonicalize().unwrap().to_str().unwrap()
    );
    assert!(result.get("sessionId").and_then(Value::as_str).is_some());
    let id = result["sessionId"].as_str().unwrap();
    value["principals"][0]["grants"]
        .as_array_mut()
        .unwrap()
        .push(json!({"platform_id":"p","company_id":"foreign","host_id":"h","launch":true}));
    value["principals"].as_array_mut().unwrap().push(json!({"id":"b","token_sha256":hex::encode(Sha256::digest("fixture-other-principal-authorized-token")),"grants":[{"platform_id":"p","company_id":"c","host_id":"h","launch":true}]}));
    std::fs::write(&policy, value.to_string()).unwrap();
    for (company, token) in [("foreign", TOKEN), ("c", "fixture-other-principal-authorized-token")]
    {
        let response = app
            .clone()
            .oneshot(
                HttpRequest::builder()
                    .uri(format!("/agent-apps/launch-sessions/{id}?targetHostId=h"))
                    .header("x-platform-id", "p")
                    .header("x-company-id", company)
                    .header("x-agent-apps-token", token)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}

fn attached_terminal(_: Option<&str>, _: &Path, material: &LaunchMaterial) -> Result<(), String> {
    use std::io::Read;
    assert!(!format!("{material:?}").contains("fixture-private-key"));
    assert!(!format!("{material:?}").contains("fixture-old-refresh"));
    let script = std::fs::read_to_string(&material.args[0]).unwrap();
    assert!(!script.contains("fixture-private-key") && !script.contains("fixture-old-refresh"));
    let socket = material.cwd.join("io");
    let mut stream =
        std::os::unix::net::UnixStream::connect(socket).map_err(|_| "fixture attach failed")?;
    std::thread::spawn(move || {
        let mut output = Vec::new();
        let _ = stream.read_to_end(&mut output);
    });
    Ok(())
}

#[tokio::test]
async fn cc_apps_launch_identity_list_sdk_http_tty_lifetime_and_account_rotation() {
    identity_roundtrip(attached_terminal).await;
}
#[cfg(target_os = "macos")]
#[tokio::test]
async fn cc_apps_launch_native_terminal_sdk_http_lifetime_and_rotation() {
    identity_roundtrip(launch_posix_terminal).await;
}
async fn identity_roundtrip(
    terminal_boundary: fn(Option<&str>, &Path, &LaunchMaterial) -> Result<(), String>,
) {
    use vw_api_types::{
        agent_apps::*,
        app_providers::{AppProviderWrite, ProviderProtocol},
    };
    use vw_gateway_client::{AgentAppsSettingsError, GatewayClient, GatewayEndpoint};
    let root = tempfile::tempdir().unwrap();
    std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    let bin = root.path().join("bin");
    std::fs::create_dir(&bin).unwrap();
    for name in ["claude", "codex"] {
        let script = if name == "claude" {
            "#!/bin/sh\nif [ \"$1\" = --version ]; then echo 'claude 1.0.0'; exit 0; fi\ntest -t 0 && test -t 1 || exit 30\ntest \"$ANTHROPIC_API_KEY\" = fixture-private-key || exit 31\nprintf '%s' \"$CLAUDE_CONFIG_DIR\" > private-path\nwhile [ ! -f release ]; do sleep 0.05; done\n"
        } else {
            "#!/bin/sh\nif [ \"$1\" = --version ]; then echo 'codex 1.0.0'; exit 0; fi\ntest -t 0 && test -t 1 || exit 30\ntest -z \"$OPENAI_API_KEY\" || exit 31\nprintf '%s' \"$CODEX_HOME\" > account-path\nprintf '%s' '{\"auth_mode\":\"chatgpt\",\"tokens\":{\"access_token\":\"fixture-new-access\",\"refresh_token\":\"fixture-new-refresh\",\"id_token\":\"fixture-id\",\"account_id\":\"fixture-account\"}}' > \"$CODEX_HOME/auth.json\"\nwhile [ ! -f release ]; do sleep 0.05; done\nexit 9\n"
        };
        let path = bin.join(name);
        std::fs::write(&path, script).unwrap();
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700)).unwrap();
    }
    let credentials = root.path().join("credentials");
    std::fs::create_dir(&credentials).unwrap();
    std::fs::set_permissions(&credentials, std::fs::Permissions::from_mode(0o700)).unwrap();
    let policy = root.path().join("policy.json");
    let other_token = "fixture-other-principal-authorized-token-token-12345678";
    let restricted_token = "fixture-no-identity-grant-token-12345678";
    let permitted = json!([
        {"appId":"claude","kind":"provider","id":"selected-provider"},
        {"appId":"codex","kind":"official_account","id":"selected-account"},
        {"appId":"claude","kind":"provider","id":"missing-record"},
        {"appId":"codex","kind":"official_account","id":"missing-record"}
    ]);
    let grants = json!([{"platform_id":"p","company_id":"c","host_id":"h","launch":true,"identities":permitted},{"platform_id":"p","company_id":"other","host_id":"h","launch":true,"identities":permitted}]);
    let value = json!({"principals":[{"id":"a","token_sha256":hex::encode(Sha256::digest(TOKEN)),"grants":grants},{"id":"b","token_sha256":hex::encode(Sha256::digest(other_token)),"grants":grants},{"id":"restricted","token_sha256":hex::encode(Sha256::digest(restricted_token)),"grants":[{"platform_id":"p","company_id":"c","host_id":"h","launch":true}]}],"hosts":[{"id":"h","online":true,"binding":"local","home":root.path(),"search_paths":[bin],"applications":root.path(),"credential_root":credentials}]});
    std::fs::write(&policy, value.to_string()).unwrap();
    std::fs::set_permissions(&policy, std::fs::Permissions::from_mode(0o600)).unwrap();
    let db = GlobalDatabase::for_home(root.path()).unwrap();
    let tenant = TenantScope::new("p", "c").unwrap();
    let providers = crate::app_providers::AppProviderStore::new(db.clone()).unwrap();
    providers
        .save(
            &tenant,
            AppId::Claude,
            "selected-provider",
            &AppProviderWrite {
                request_options: None,
                name: "中文供应商".into(),
                protocol: ProviderProtocol::Anthropic,
                api_key: "fixture-private-key".into(),
                base_url: "https://fixture.example.test".into(),
                model: "fixture".into(),
            },
        )
        .unwrap();
    let accounts =
        crate::agent_apps::auth_accounts::AuthAccountStore::new(db.clone(), &credentials).unwrap();
    accounts.save(&tenant, "h", AppId::Codex, "selected-account", &vw_agent_apps::auth::credential_file::AccountCredential::CodexAuth(json!({"auth_mode":"chatgpt","tokens":{"access_token":"fixture-old-access","refresh_token":"fixture-old-refresh","id_token":"fixture-id","account_id":"fixture-account"}}))).unwrap();
    let router = Router::new()
        .nest("/v1", router_with_terminal(Some(policy), terminal_boundary))
        .layer(Extension(db))
        .layer(middleware::from_fn(crate::gateway::middleware::tenant_scope_middleware));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let server = tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    let client = GatewayClient::new(
        GatewayEndpoint::new("127.0.0.1", port).with_tenant_scope(tenant.clone()),
    )
    .unwrap();
    assert!(matches!(
        client
            .agent_apps_launch_identities(
                "h",
                AppId::Claude,
                "invalid-token-long-enough-1234567890"
            )
            .await,
        Err(AgentAppsSettingsError::Http { status: 401, .. })
    ));
    let other = GatewayClient::new(
        GatewayEndpoint::new("127.0.0.1", port)
            .with_tenant_scope(TenantScope::new("p", "other").unwrap()),
    )
    .unwrap();
    assert!(
        other.agent_apps_launch_identities("h", AppId::Claude, TOKEN).await.unwrap().is_empty()
    );
    for (app_id, path_file, expected) in [
        (AppId::Claude, "private-path", LaunchSessionPhase::Completed),
        (AppId::Codex, "account-path", LaunchSessionPhase::Failed),
    ] {
        let identities = client.agent_apps_launch_identities("h", app_id, TOKEN).await.unwrap();
        assert_eq!(identities.len(), 1);
        let serialized = serde_json::to_string(&identities).unwrap();
        assert!(
            !serialized.contains("fixture-private-key")
                && !serialized.contains("fixture-old-refresh")
                && !serialized.contains(root.path().to_str().unwrap())
        );
        let observed = vw_agent_apps::apps::discovery::read::read(
            app_id,
            &vw_agent_apps::apps::discovery::read::LocalReadEnvironment {
                host_id: "h".into(),
                home: root.path().into(),
                search_paths: vec![bin.clone()],
                applications: root.path().into(),
            },
        )
        .unwrap();
        let request = LaunchRequest {
            app_id,
            installation_id: observed.installations[0].id.clone(),
            cwd: root.path().to_str().unwrap().into(),
            args: vec![],
            skip_permissions: false,
        };
        let mut missing = identities[0].clone();
        missing.id = "missing-record".into();
        assert!(matches!(
            client.agent_apps_launch_identity_authorized("h", &request, &missing, TOKEN).await,
            Err(AgentAppsSettingsError::Http { status: 422, .. })
        ));
        assert!(
            client
                .agent_apps_launch_identities("h", app_id, restricted_token)
                .await
                .unwrap()
                .is_empty()
        );
        assert!(matches!(
            client
                .agent_apps_launch_identity_authorized(
                    "h",
                    &request,
                    &identities[0],
                    restricted_token
                )
                .await,
            Err(AgentAppsSettingsError::Http { status: 403, .. })
        ));
        let mut wrong_kind = identities[0].clone();
        wrong_kind.kind = if wrong_kind.kind == LaunchIdentityKind::Provider {
            LaunchIdentityKind::OfficialAccount
        } else {
            LaunchIdentityKind::Provider
        };
        assert!(matches!(
            client.agent_apps_launch_identity_authorized("h", &request, &wrong_kind, TOKEN).await,
            Err(AgentAppsSettingsError::Http { status: 403, .. })
        ));
        assert!(!root.path().join(path_file).exists());
        let result = client
            .agent_apps_launch_identity_authorized("h", &request, &identities[0], TOKEN)
            .await
            .unwrap();
        let id = result.session_id.unwrap();
        assert_eq!(
            client.agent_apps_launch_session("h", &id, TOKEN).await.unwrap().phase,
            LaunchSessionPhase::Running
        );
        assert!(matches!(
            client.agent_apps_launch_session("h", &id, other_token).await,
            Err(AgentAppsSettingsError::Http { status: 404, .. })
        ));
        assert!(matches!(
            other.agent_apps_launch_session("h", &id, TOKEN).await,
            Err(AgentAppsSettingsError::Http { status: 404, .. })
        ));
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while !root.path().join(path_file).exists() {
            assert!(std::time::Instant::now() < deadline);
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        let private =
            std::path::PathBuf::from(std::fs::read_to_string(root.path().join(path_file)).unwrap());
        assert!(private.is_dir());
        assert_eq!(private.metadata().unwrap().permissions().mode() & 0o777, 0o700);
        if app_id == AppId::Codex {
            assert!(
                accounts
                    .prepare_launch(
                        &tenant,
                        "h",
                        "selected-account",
                        &request,
                        &observed.installations,
                        &[root.path().into()],
                        Path::new("/tmp")
                    )
                    .is_err(),
                "account lease must remain held while CLI runs"
            );
        }
        std::fs::write(root.path().join("release"), "done").unwrap();
        loop {
            let phase = client.agent_apps_launch_session("h", &id, TOKEN).await.unwrap().phase;
            if phase == expected {
                break;
            }
            assert!(std::time::Instant::now() < deadline, "CLI not reaped/cleaned");
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
        assert!(!private.exists(), "completion must follow private material cleanup");
        std::fs::remove_file(root.path().join("release")).unwrap();
    }
    let stored = vw_agent_apps::auth::credential_file::CredentialFileStore::new(&credentials)
        .unwrap()
        .read(&tenant, "h", AppId::Codex, "selected-account")
        .unwrap();
    let vw_agent_apps::auth::credential_file::AccountCredential::CodexAuth(auth) = stored else {
        panic!("wrong material");
    };
    assert_eq!(auth["tokens"]["refresh_token"], "fixture-new-refresh");
    server.abort();
    let _ = server.await;
}
