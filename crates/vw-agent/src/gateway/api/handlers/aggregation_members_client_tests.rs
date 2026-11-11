use super::*;

fn client_fixture(app: AppId, name: &str) -> Fixture {
    let f = Fixture::new();
    let mut policy: Value = serde_json::from_slice(&std::fs::read(&f.policy).unwrap()).unwrap();
    for grant in ["aggregation_read", "aggregation_write"] {
        policy["principals"][0]["grants"][0][grant]
            .as_array_mut()
            .unwrap()
            .push(json!({"project_id":"project","app_id":name}));
    }
    std::fs::write(&f.policy, policy.to_string()).unwrap();
    for (id, model) in [("alpha", "first"), ("beta", "second")] {
        if app == AppId::Codex {
            crate::app_providers::AppProviderStore::new(f.db.clone()).unwrap().codex_save(&tenant(),id,&vw_api_types::app_providers::codex_catalog::CodexProfile {
                name:id.into(),config:format!("model='{model}'\nmodel_provider='relay'\n[model_providers.relay]\nbase_url='https://fixture.invalid/v1'\nwire_api='responses'\n"),api_key:Some("synthetic-private-key".into()),preserve_login:false
            },None,None).unwrap();
        } else {
            let row=vw_agent_apps::providers::desktop_mapping::import_row(&json!({"name":id,"category":"custom","settingsConfig":{"env":{"ANTHROPIC_BASE_URL":"https://fixture.invalid","ANTHROPIC_AUTH_TOKEN":"synthetic-private-key","ANTHROPIC_MODEL":model}}})).unwrap().unwrap();
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
                members: vec!["alpha".into()],
                keys: [("alpha".into(), "alpha".into())].into(),
            }),
            ..Default::default()
        },
        || Ok(()),
    )
    .unwrap();
    drop(guard);
    f
}

#[tokio::test]
async fn aggregation_t40_codex_and_desktop_save_planned_members_in_original_slot() {
    for (app, name) in [(AppId::Codex, "codex"), (AppId::ClaudeDesktop, "claude-desktop")] {
        let f = client_fixture(app, name);
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
        let (status,changed)=f.request("PUT",&format!("/agent-apps/{name}/aggregation/default"),Some(TOKEN),json!({"targetHostId":"host","projectId":"project","expectedRevision":current["revision"],"providerId":"beta"})).await;
        assert_eq!(status, 200, "{changed}");
        assert_eq!(changed["defaultProviderId"], "beta");
        let state = f.tx.state(app).unwrap();
        assert_eq!(state.project_scope, Some(scope("project")));
        assert!(state.stack.members.contains(&"beta".into()));
        assert_eq!(state.pointer.as_ref().unwrap().id, "alpha");
        assert!(
            !f.root.path().join(if app == AppId::Codex { ".codex" } else { "Library" }).exists()
        );
    }
}

#[tokio::test]
async fn aggregation_t40_codex_active_default_changes_native_selected_model() {
    let mut f = client_fixture(AppId::Codex, "codex");
    let mut policy: Value = serde_json::from_slice(&std::fs::read(&f.policy).unwrap()).unwrap();
    let grant = &mut policy["principals"][0]["grants"][0];
    grant["routing_manage"] = json!(true);
    grant["routing_read"] = json!(true);
    grant["routing_apps"]
        .as_array_mut()
        .unwrap()
        .push(json!({"project_id":"project","app_id":"codex"}));
    std::fs::write(&f.policy, policy.to_string()).unwrap();
    std::fs::create_dir(f.root.path().join(".codex")).unwrap();
    let path = f.root.path().join(".codex/config.toml");
    std::fs::write(&path,"model='first'\nmodel_provider='relay'\n[model_providers.relay]\nbase_url='http://127.0.0.1:1/v1'\nwire_api='responses'\n").unwrap();
    let mut guard = lock_app(AppId::Codex, f.root.path()).unwrap();
    let before = f.tx.state(AppId::Codex).unwrap();
    let mut stack = before.stack.clone();
    stack.enabled = true;
    stack.members.push("beta".into());
    stack.keys.insert("beta".into(), "beta".into());
    f.tx.commit_project_checked(
        &scope("project"),
        &mut guard,
        &before,
        false,
        &[],
        PendingTarget {
            stack: Some(stack),
            mode: Some(ModeState {
                mode: Mode::Proxy,
                attached: true,
                proxy_route: Some("alpha".into()),
                contract: None,
            }),
            ..Default::default()
        },
        || Ok(()),
    )
    .unwrap();
    drop(guard);
    let listener = std::sync::Arc::new(crate::app_routing::RoutingListener::default());
    listener.start(0).unwrap();
    f.routing = Some(listener.clone());
    let (status, current) = f
        .request(
            "GET",
            "/agent-apps/codex/aggregation/state?targetHostId=host&projectId=project",
            Some(TOKEN),
            json!({}),
        )
        .await;
    assert_eq!(status, 200, "{current}");
    let (status,changed)=f.request("PUT","/agent-apps/codex/aggregation/default",Some(TOKEN),json!({"targetHostId":"host","projectId":"project","expectedRevision":current["revision"],"providerId":"beta"})).await;
    listener.stop().await.unwrap();
    assert_eq!(status, 200, "{changed}");
    assert_eq!(changed["defaultProviderId"], "beta");
    let native: toml_edit::DocumentMut = std::fs::read_to_string(&path)
        .unwrap()
        .parse()
        .unwrap_or_else(|_| panic!("native Codex TOML must be valid"));
    assert_eq!(native["model"].as_str(), Some("second"));
    let catalog: Value = serde_json::from_slice(
        &std::fs::read(f.root.path().join(".codex/shiju-model-catalog.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(catalog["models"][0]["slug"], "second");
    assert_eq!(f.tx.state(AppId::Codex).unwrap().pointer.as_ref().unwrap().id, "alpha");
}

#[test]
fn aggregation_t40_desktop_new_default_requires_a_live_publication() {
    use vw_agent_apps::aggregation::membership::{MembershipChange, SavedMember, plan_member};
    let f = client_fixture(AppId::ClaudeDesktop, "claude-desktop");
    let mut guard = lock_app(AppId::ClaudeDesktop, f.root.path()).unwrap();
    let before = f.tx.state(AppId::ClaudeDesktop).unwrap();
    let mut stack = before.stack.clone();
    stack.enabled = true;
    f.tx.commit_project_checked(
        &scope("project"),
        &mut guard,
        &before,
        false,
        &[],
        PendingTarget {
            stack: Some(stack),
            mode: Some(ModeState {
                mode: Mode::Proxy,
                attached: true,
                proxy_route: Some("alpha".into()),
                contract: None,
            }),
            ..Default::default()
        },
        || Ok(()),
    )
    .unwrap();
    drop(guard);
    let expected = f.tx.state(AppId::ClaudeDesktop).unwrap();
    let target = plan_member(
        AppId::ClaudeDesktop,
        &expected,
        MembershipChange::SetDefault {
            provider_id: "beta",
            saved: Some(SavedMember { name: "beta", icon: None, official_service: None }),
        },
    )
    .unwrap();
    let mut candidate = expected.clone();
    candidate.stack = target.stack.unwrap();
    candidate.mode = target.mode;
    let project = scope("project");
    let snapshot = super::super::aggregation_read_snapshot::load(
        &f.db,
        &project,
        "host",
        AppId::ClaudeDesktop,
        f.root.path(),
        &expected,
        None,
    )
    .unwrap();
    let revision = super::super::aggregation_read_snapshot::revision(
        &project,
        "host",
        AppId::ClaudeDesktop,
        "writer",
        &expected,
        &snapshot,
    )
    .unwrap();
    let result = crate::gateway::aggregation_resync::RecordPublicationContext {
        database: f.db.clone(),
        project,
        host: "host".into(),
        app: AppId::ClaudeDesktop,
        home: f.root.path().to_owned(),
        root: f.root.path().join("private"),
        routing: Some(std::sync::Arc::new(crate::app_routing::RoutingListener::default())),
        expected: expected.clone(),
        principal: "writer".into(),
        revision,
        failure: Default::default(),
    }
    .publish_candidate(&candidate);
    assert_eq!(result, Err(StatusCode::SERVICE_UNAVAILABLE));
    assert!(f.tx.state(AppId::ClaudeDesktop).unwrap() == expected);
    assert!(!f.root.path().join("Library").exists());
    assert!(f.tx.pending(AppId::ClaudeDesktop).unwrap().is_none());
}

#[path = "aggregation_members_ordinary_tests.rs"]
mod ordinary_tests;
