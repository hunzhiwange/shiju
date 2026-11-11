use super::*;

pub(super) fn client(app: AppId, name: &str) -> Fixture {
    let f = Fixture::new();
    f.policy(true, true, true);
    let mut policy: Value = serde_json::from_slice(&std::fs::read(&f.policy).unwrap()).unwrap();
    for grant in ["aggregation_read", "aggregation_write", "routing_apps"] {
        policy["principals"][0]["grants"][0][grant]
            .as_array_mut()
            .unwrap()
            .push(json!({"project_id":"project","app_id":name}));
    }
    std::fs::write(&f.policy, policy.to_string()).unwrap();
    for (id, model) in [("alpha", "first"), ("beta", "second")] {
        if app == AppId::Codex {
            crate::app_providers::AppProviderStore::new(f.db.clone()).unwrap().codex_save(&tenant(), id,
                &vw_api_types::app_providers::codex_catalog::CodexProfile {
                    name:id.into(), config:format!("model='{model}'\nmodel_provider='relay'\n[model_providers.relay]\nbase_url='https://fixture.invalid/v1'\nwire_api='responses'\n"),
                    api_key:Some("synthetic-private-key".into()), preserve_login:false,
                }, None, None).unwrap();
        } else {
            let row = vw_agent_apps::providers::desktop_mapping::import_row(&json!({"name":id,"category":"custom",
                "settingsConfig":{"env":{"ANTHROPIC_BASE_URL":"https://fixture.invalid","ANTHROPIC_AUTH_TOKEN":"synthetic-private-key","ANTHROPIC_MODEL":model}}})).unwrap().unwrap();
            GatewaySettingsStore::new(f.db.clone())
                .unwrap()
                .upsert(
                    &tenant(),
                    "app_providers.records",
                    &format!("\"{name}\":{id}"),
                    &row.to_string(),
                )
                .unwrap();
        }
    }
    let mut guard = lock_app(app, f.root.path()).unwrap();
    f.tx.commit_project_checked(
        &scope("project"),
        &mut guard,
        &f.tx.state(app).unwrap(),
        true,
        &[],
        PendingTarget {
            pointer: Some("alpha".into()),
            mode: Some(ModeState {
                mode: Mode::Direct,
                attached: false,
                proxy_route: Some("alpha".into()),
                contract: None,
            }),
            stack: Some(StackState {
                enabled: false,
                members: vec!["alpha".into(), "beta".into()],
                keys: [("alpha".into(), "alpha".into()), ("beta".into(), "beta".into())].into(),
            }),
            ..Default::default()
        },
        || Ok(()),
    )
    .unwrap();
    drop(guard);
    f
}
pub(super) async fn intent(f: &Fixture, name: &str, policy: &str) -> Value {
    let (status, preview) = f
        .request(
            "POST",
            &format!("/agent-apps/{name}/aggregation/preview"),
            Some(TOKEN),
            json!({"targetHostId":"host","projectId":"project","catalogPolicy":policy}),
        )
        .await;
    assert_eq!(status, 200, "{preview}");
    assert!(preview["previewToken"].is_string(), "{preview}");
    json!({"targetHostId":"host","projectId":"project","expectedRevision":preview["revision"],"previewToken":preview["previewToken"],"requestId":"client-publication","confirmed":true})
}
#[tokio::test]
async fn aggregation_t41_codex_initial_publication_honors_confirmed_catalog_takeover() {
    let mut f = client(AppId::Codex, "codex");
    std::fs::create_dir(f.root.path().join(".codex")).unwrap();
    let config = f.root.path().join(".codex/config.toml");
    std::fs::write(&config, "model='first'\nmodel_provider='relay'\nmodel_catalog_json='external.json'\n[model_providers.relay]\nbase_url='https://fixture.invalid/v1'\nwire_api='responses'\n").unwrap();
    std::fs::write(f.root.path().join(".codex/auth.json"), b"{\"native\":\"retained\"}").unwrap();
    let listener = std::sync::Arc::new(crate::app_routing::RoutingListener::default());
    listener.start(0).unwrap();
    f.routing = Some(listener.clone());
    let before = f.tx.state(AppId::Codex).unwrap();
    let preserved = intent(&f, "codex", "preserve").await;
    let (status, progress) =
        f.request("POST", "/agent-apps/codex/aggregation/publish", Some(TOKEN), preserved).await;
    assert_eq!(status, 202, "{progress}");
    let failed = f.settled(progress["operationId"].as_str().unwrap()).await;
    assert_eq!(failed["status"], "failed", "{failed}");
    assert!(f.tx.state(AppId::Codex).unwrap() == before);
    let mut managed = intent(&f, "codex", "use_managed").await;
    managed["requestId"] = json!("managed-publication");
    let (status, progress) =
        f.request("POST", "/agent-apps/codex/aggregation/publish", Some(TOKEN), managed).await;
    assert_eq!(status, 202, "{progress}");
    let terminal = f.settled(progress["operationId"].as_str().unwrap()).await;
    assert_eq!(terminal["status"], "completed", "{terminal}");
    assert_eq!(terminal["aggregationOutcome"]["kind"], "published");
    assert_eq!(terminal["aggregationOutcome"]["state"]["mode"], "aggregation");
    let state = f.tx.state(AppId::Codex).unwrap();
    assert!(state.stack.enabled);
    assert!(state.mode.unwrap().contract.is_some());
    let doc: toml_edit::DocumentMut = std::fs::read_to_string(config)
        .unwrap()
        .parse()
        .unwrap_or_else(|_| panic!("native Codex TOML must be valid"));
    assert!(doc["model_catalog_json"].as_str().unwrap().ends_with("shiju-model-catalog.json"));
    let catalog: Value = serde_json::from_slice(
        &std::fs::read(f.root.path().join(".codex/shiju-model-catalog.json")).unwrap(),
    )
    .unwrap();
    assert!(
        catalog["models"]
            .as_array()
            .unwrap()
            .iter()
            .any(|m| m["slug"].as_str().is_some_and(|s| s.contains("second")))
    );
    assert_eq!(
        std::fs::read(f.root.path().join(".codex/auth.json")).unwrap(),
        b"{\"native\":\"retained\"}"
    );
    listener.stop().await.unwrap();
}
#[tokio::test]
async fn aggregation_t41_desktop_initial_publication_requires_installation_and_writes_original_profile()
 {
    let mut f = client(AppId::ClaudeDesktop, "claude-desktop");
    let listener = std::sync::Arc::new(crate::app_routing::RoutingListener::default());
    listener.start(0).unwrap();
    f.routing = Some(listener.clone());
    let request = intent(&f, "claude-desktop", "preserve").await;
    let original = f.tx.state(AppId::ClaudeDesktop).unwrap();
    let (status, body) = f
        .request(
            "POST",
            "/agent-apps/claude-desktop/aggregation/publish",
            Some(TOKEN),
            request.clone(),
        )
        .await;
    assert_eq!(status, 422, "{body}");
    assert_eq!(body["error"]["code"], "unsupported_target");
    assert!(f.tx.state(AppId::ClaudeDesktop).unwrap() == original);
    std::fs::create_dir_all(f.root.path().join("Applications/Claude.app")).unwrap();
    let (status, progress) = f
        .request("POST", "/agent-apps/claude-desktop/aggregation/publish", Some(TOKEN), request)
        .await;
    assert_eq!(status, 202, "{progress}");
    let terminal = f.settled(progress["operationId"].as_str().unwrap()).await;
    assert_eq!(terminal["status"], "completed", "{terminal}");
    assert_eq!(terminal["aggregationOutcome"]["kind"], "published");
    assert_eq!(terminal["aggregationOutcome"]["state"]["mode"], "aggregation");
    let state = f.tx.state(AppId::ClaudeDesktop).unwrap();
    assert!(state.stack.enabled && state.mode.unwrap().attached);
    let profile = f.root.path().join("Library/Application Support/Claude-3p/configLibrary/00000000-0000-4000-8000-000000157210.json");
    let profile: Value = serde_json::from_slice(&std::fs::read(profile).unwrap()).unwrap();
    assert!(profile.is_object());
    assert!(f.tx.pending(AppId::ClaudeDesktop).unwrap().is_none());
    listener.stop().await.unwrap();
}
