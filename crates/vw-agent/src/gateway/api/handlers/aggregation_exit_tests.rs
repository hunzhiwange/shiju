use super::*;
#[tokio::test]
async fn aggregation_t42_aggregation_exit_preserves_contract() {
    let f = Fixture::new();
    f.policy(true, true, true);
    let revision = f.current().await["revision"].clone();
    let response = f.router().await.oneshot(Request::builder().method("POST")
        .uri("/agent-apps/claude/aggregation/exit")
        .header(PLATFORM_ID_HEADER,"platform").header(COMPANY_ID_HEADER,"company")
        .header("authorization",format!("Bearer {TOKEN}")).header("content-type","application/json")
        .body(Body::from(json!({"targetHostId":"host","projectId":"project","expectedRevision":revision,"requestId":"exit","confirmed":false}).to_string())).unwrap()).await.unwrap();
    assert_eq!(response.status(), 422);
    let body: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 64 * 1024).await.unwrap()).unwrap();
    assert_eq!(body["error"]["code"], "confirmation_required");
}

async fn published() -> (Fixture, std::sync::Arc<crate::app_routing::RoutingListener>) {
    let mut f = Fixture::new();
    f.prepare_native();
    let listener = f.listener();
    let store = crate::app_providers::AppProviderStore::new(f.db.clone()).unwrap();
    for (id, model) in [("alpha", "first"), ("beta", "second")] {
        let old = store.claude_record(&tenant(), id).unwrap();
        store
            .claude_save(
                &tenant(),
                id,
                &vw_api_types::app_providers::claude::ClaudeProfile {
                    name: id.into(),
                    config: vw_api_types::app_providers::claude::ClaudeConfig::ThirdParty {
                        base_url: "http://127.0.0.1:9".into(),
                        api_key: "synthetic-private-key".into(),
                        api_key_header: true,
                    },
                    model: Some(model.into()),
                },
                old.as_ref(),
                false,
            )
            .unwrap();
    }
    let request = f.publish_request().await;
    let (status, operation) = f.publish(request).await;
    assert_eq!(status, 202, "{operation}");
    let terminal = f.settled(operation["operationId"].as_str().unwrap()).await;
    assert_eq!(terminal["status"], "completed", "{terminal}");
    (f, listener)
}
async fn exit_request(f: &Fixture) -> Value {
    json!({"targetHostId":"host","projectId":"project","expectedRevision":f.current().await["revision"],"requestId":"publication","confirmed":true})
}
async fn exit(f: &Fixture, request: Value) -> (StatusCode, Value) {
    f.request("POST", "/agent-apps/claude/aggregation/exit", Some(TOKEN), request).await
}
#[tokio::test]
async fn aggregation_t42_exit_restores_direct_and_replay_is_action_scoped() {
    let (f, listener) = published().await;
    let before = f.tx.state(AppId::Claude).unwrap();
    let native: Value = serde_json::from_slice(&std::fs::read(f.native()).unwrap()).unwrap();
    let token = native["env"]["ANTHROPIC_AUTH_TOKEN"].as_str().unwrap();
    let url = native["env"]["ANTHROPIC_BASE_URL"].as_str().unwrap();
    let request = exit_request(&f).await;
    let ((status, operation), (again, replay)) =
        tokio::join!(exit(&f, request.clone()), exit(&f, request.clone()));
    assert_eq!(status, 202, "{operation}");
    assert_eq!(again, 202, "{replay}");
    assert_eq!(operation["operationId"], replay["operationId"]);
    let id = operation["operationId"].as_str().unwrap();
    let terminal = f.settled(id).await;
    assert_eq!(terminal["status"], "completed", "{terminal}");
    assert_eq!(terminal["aggregationOutcome"]["kind"], "exited");
    assert_eq!(terminal["aggregationOutcome"]["state"]["recovery"], "restored");
    assert_eq!(terminal["aggregationOutcome"]["state"]["mode"], "direct");
    let _: vw_api_types::app_providers::operations::ProviderOperationProgress =
        serde_json::from_value(terminal).unwrap();
    let state = f.tx.state(AppId::Claude).unwrap();
    assert!(!state.stack.enabled && !state.mode.unwrap().attached);
    assert_eq!(state.stack.keys, before.stack.keys);
    assert_eq!(state.stack.members, before.stack.members);
    assert_eq!(state.pointer.unwrap().id, "alpha");
    let restored = std::fs::read(f.native()).unwrap();
    let native: Value = serde_json::from_slice(&restored).unwrap();
    assert_eq!(native["env"]["ANTHROPIC_BASE_URL"], "http://127.0.0.1:9");
    assert_eq!(native["hooks"]["user"], true);
    assert!(native["modelPicker"].is_null());
    let response = reqwest::Client::new()
        .post(format!("{url}/v1/messages"))
        .bearer_auth(token)
        .json(
            &json!({"model":"first","max_tokens":1,"messages":[{"role":"user","content":"probe"}]}),
        )
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 401);
    let (_, replay) = exit(&f, request.clone()).await;
    assert_eq!(replay["operationId"], id);
    assert_eq!(std::fs::read(f.native()).unwrap(), restored);
    let mut unconfirmed = request.clone();
    unconfirmed["confirmed"] = json!(false);
    let (status, body) = exit(&f, unconfirmed).await;
    assert_eq!(status, 409);
    assert_eq!(body["error"]["code"], "request_conflict");
    let mut altered = request;
    altered["expectedRevision"] = json!("new-payload");
    let (status, body) = exit(&f, altered).await;
    assert_eq!(status, 409);
    assert_eq!(body["error"]["code"], "request_conflict");
    assert!(f.tx.pending(AppId::Claude).unwrap().is_none());
    listener.stop().await.unwrap();
}
#[tokio::test]
async fn aggregation_t42_auth_and_old_revision_leave_files_unchanged() {
    let (f, listener) = published().await;
    let request = exit_request(&f).await;
    let state = f.tx.state(AppId::Claude).unwrap();
    let bytes = std::fs::read(f.native()).unwrap();
    assert_eq!(
        f.request("POST", "/agent-apps/claude/aggregation/exit", None, request.clone()).await.0,
        401
    );
    f.policy(true, true, false);
    assert_eq!(exit(&f, request.clone()).await.0, 403);
    f.policy(true, true, true);
    let mut stale = request.clone();
    stale["expectedRevision"] = json!("old");
    assert_eq!(exit(&f, stale).await.0, 409);
    let mut extra = request.clone();
    extra["path"] = json!("/tmp/injected");
    assert_eq!(exit(&f, extra).await.0, 400);
    for (field, value) in [("projectId", "other"), ("targetHostId", "other")] {
        let mut other = request.clone();
        other[field] = json!(value);
        assert_eq!(exit(&f, other).await.0, 403);
    }
    assert!(f.tx.state(AppId::Claude).unwrap() == state);
    assert_eq!(std::fs::read(f.native()).unwrap(), bytes);
    // A current request still cannot overwrite client edits to controlled values.
    let mut native: Value = serde_json::from_slice(&bytes).unwrap();
    native["modelPicker"] = json!({"user":"changed"});
    std::fs::write(f.native(), native.to_string()).unwrap();
    assert_eq!(exit(&f, request).await.0, 409);
    let fresh = exit_request(&f).await;
    let (_, op) = exit(&f, fresh).await;
    let result = f.settled(op["operationId"].as_str().unwrap()).await;
    assert_eq!(result["status"], "failed", "{result}");
    assert_eq!(result["aggregationOutcome"]["code"], "revision_conflict");
    assert_eq!(
        serde_json::from_slice::<Value>(&std::fs::read(f.native()).unwrap()).unwrap(),
        native
    );
    std::fs::write(f.native(), bytes).unwrap();
    listener.stop().await.unwrap();
}
#[tokio::test]
async fn aggregation_t42_restore_io_failure_is_queryable_and_not_success() {
    let (f, listener) = published().await;
    let request = exit_request(&f).await;
    let before = f.tx.state(AppId::Claude).unwrap();
    let bytes = std::fs::read(f.native()).unwrap();
    let resource = f.root.path().join("private/provider-live/aG9zdA");
    drop(f.tx.lock_claude_directory().unwrap());
    std::fs::set_permissions(&resource, std::fs::Permissions::from_mode(0o500)).unwrap();
    let (status, operation) = exit(&f, request).await;
    assert_eq!(status, 202, "{operation}");
    let terminal = f.settled(operation["operationId"].as_str().unwrap()).await;
    std::fs::set_permissions(&resource, std::fs::Permissions::from_mode(0o700)).unwrap();
    assert_eq!(terminal["status"], "failed", "{terminal}");
    assert_eq!(terminal["aggregationOutcome"]["status"], 502);
    assert_eq!(terminal["aggregationOutcome"]["code"], "recovery_failed");
    assert_eq!(terminal["aggregationOutcome"]["recovery"], "none");
    assert!(f.tx.state(AppId::Claude).unwrap() == before);
    assert_eq!(std::fs::read(f.native()).unwrap(), bytes);
    let id = operation["operationId"].as_str().unwrap();
    let wrong = format!("/app-providers/operations/{id}?targetHostId=host&projectId=other");
    assert_eq!(f.request("GET", &wrong, Some(TOKEN), json!({})).await.0, 404);
    f.policy(true, true, false);
    let path = format!("/app-providers/operations/{id}?targetHostId=host&projectId=project");
    assert_eq!(f.request("GET", &path, Some(TOKEN), json!({})).await.0, 403);
    assert_eq!(f.request("DELETE", &path, Some(TOKEN), json!({})).await.0, 403);
    f.policy(true, true, true);
    listener.stop().await.unwrap();
}

#[tokio::test]
async fn aggregation_t42_codex_and_desktop_use_native_restore_owners() {
    for (app, name) in [(AppId::Codex, "codex"), (AppId::ClaudeDesktop, "claude-desktop")] {
        let mut f = super::clients::client(app, name);
        let native = if app == AppId::Codex {
            std::fs::create_dir_all(f.root.path().join(".codex")).unwrap();
            let path = f.root.path().join(".codex/config.toml");
            std::fs::write(&path,"model='first'\nmodel_provider='relay'\n[model_providers.relay]\nbase_url='https://fixture.invalid/v1'\nwire_api='responses'\n").unwrap();
            std::fs::write(f.root.path().join(".codex/auth.json"), b"{\"native\":\"retained\"}")
                .unwrap();
            path
        } else {
            std::fs::create_dir_all(f.root.path().join("Applications/Claude.app")).unwrap();
            f.root.path().join("Library/Application Support/Claude-3p/configLibrary/00000000-0000-4000-8000-000000157210.json")
        };
        let listener = std::sync::Arc::new(crate::app_routing::RoutingListener::default());
        listener.start(0).unwrap();
        f.routing = Some(listener.clone());
        let intent = super::clients::intent(&f, name, "preserve").await;
        let (status, op) = f
            .request(
                "POST",
                &format!("/agent-apps/{name}/aggregation/publish"),
                Some(TOKEN),
                intent,
            )
            .await;
        assert_eq!(status, 202, "{op}");
        let terminal = f.settled(op["operationId"].as_str().unwrap()).await;
        assert_eq!(terminal["status"], "completed", "{terminal}");
        let (status, current) = f
            .request(
                "GET",
                &format!(
                    "/agent-apps/{name}/aggregation/state?targetHostId=host&projectId=project"
                ),
                Some(TOKEN),
                json!({}),
            )
            .await;
        assert_eq!(status, 200, "{current}");
        let (status,op)=f.request("POST",&format!("/agent-apps/{name}/aggregation/exit"),Some(TOKEN),json!({"targetHostId":"host","projectId":"project","requestId":"exit","confirmed":true,"expectedRevision":current["revision"]})).await;
        assert_eq!(status, 202, "{op}");
        let terminal = f.settled(op["operationId"].as_str().unwrap()).await;
        assert_eq!(terminal["status"], "completed", "{name}: {terminal}");
        assert_eq!(terminal["aggregationOutcome"]["kind"], "exited");
        assert!(!f.tx.state(app).unwrap().stack.enabled);
        let restored = std::fs::read_to_string(native).unwrap();
        if app == AppId::Codex {
            let doc: toml::Value = toml::from_str(&restored).unwrap();
            assert_eq!(doc["model"].as_str(), Some("first"));
            assert!(doc.get("model_catalog_json").is_none());
            // The saved direct profile explicitly disables login preservation.
            assert!(!f.root.path().join(".codex/auth.json").exists());
        } else {
            let doc: Value = serde_json::from_str(&restored).unwrap();
            assert_eq!(doc["inferenceGatewayBaseUrl"], "https://fixture.invalid");
            assert!(doc.get("inferenceModels").is_none());
        }
        listener.stop().await.unwrap();
    }
}

#[tokio::test]
async fn aggregation_t42_exit_after_gateway_detach_clears_saved_intent_without_rewriting_client() {
    let (f, listener) = published().await;
    listener.stop().await.unwrap();
    let state = f.tx.state(AppId::Claude).unwrap();
    assert!(state.stack.enabled && !state.mode.unwrap().attached);
    let bytes = std::fs::read(f.native()).unwrap();
    let request = exit_request(&f).await;
    let (status, op) = exit(&f, request).await;
    assert_eq!(status, 202, "{op}");
    let terminal = f.settled(op["operationId"].as_str().unwrap()).await;
    assert_eq!(terminal["status"], "completed", "{terminal}");
    assert_eq!(terminal["aggregationOutcome"]["kind"], "exited");
    assert!(!f.tx.state(AppId::Claude).unwrap().stack.enabled);
    assert_eq!(std::fs::read(f.native()).unwrap(), bytes);
}

#[tokio::test]
async fn aggregation_t42_ordinary_routing_intent_is_not_reported_as_exited() {
    let mut f = Fixture::new();
    let listener = f.listener();
    let before = f.tx.state(AppId::Claude).unwrap();
    let mut guard = lock_app(AppId::Claude, f.root.path()).unwrap();
    f.tx.commit_project_checked(
        &scope("project"),
        &mut guard,
        &before,
        false,
        &[],
        PendingTarget {
            mode: Some(ModeState {
                mode: Mode::Proxy,
                attached: false,
                proxy_route: Some("alpha".into()),
                contract: None,
            }),
            ..Default::default()
        },
        || Ok(()),
    )
    .unwrap();
    drop(guard);
    let before = f.tx.state(AppId::Claude).unwrap();
    let (status, op) = exit(&f, exit_request(&f).await).await;
    assert_eq!(status, 202, "{op}");
    let result = f.settled(op["operationId"].as_str().unwrap()).await;
    assert_eq!(result["status"], "failed", "{result}");
    assert_eq!(result["aggregationOutcome"]["code"], "revision_conflict");
    assert!(f.tx.state(AppId::Claude).unwrap() == before);
    assert!(!f.native().exists());
    listener.stop().await.unwrap();
}
