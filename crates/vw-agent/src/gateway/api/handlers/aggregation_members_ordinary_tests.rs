use super::*;

async fn state(f: &Fixture, name: &str) -> Value {
    let (status, value) = f
        .request(
            "GET",
            &format!("/agent-apps/{name}/aggregation/state?targetHostId=host&projectId=project"),
            Some(TOKEN),
            json!({}),
        )
        .await;
    assert_eq!(status, 200, "{value}");
    value
}
async fn change(f: &Fixture, name: &str, path: &str, extra: Value) -> (StatusCode, Value) {
    let mut body = json!({"targetHostId":"host","projectId":"project","expectedRevision":state(f,name).await["revision"]});
    body.as_object_mut().unwrap().extend(extra.as_object().unwrap().clone());
    f.request("PUT", &format!("/agent-apps/{name}/aggregation/{path}"), Some(TOKEN), body).await
}
fn native_path(f: &Fixture, app: AppId) -> std::path::PathBuf {
    f.root.path().join(match app {
        AppId::Claude => ".claude/settings.json",
        AppId::Codex => ".codex/config.toml",
        AppId::ClaudeDesktop => "Library/Application Support/Claude-3p/configLibrary/00000000-0000-4000-8000-000000157210.json",
        _ => unreachable!(),
    })
}
fn native(f: &Fixture, app: AppId) -> (String, String, String) {
    let bytes = std::fs::read(native_path(f, app)).unwrap();
    if app == AppId::Claude {
        let value: Value = serde_json::from_slice(&bytes).unwrap();
        let env = &value["env"];
        (
            format!("{}/v1/messages", env["ANTHROPIC_BASE_URL"].as_str().unwrap()),
            env["ANTHROPIC_AUTH_TOKEN"].as_str().unwrap().into(),
            env["ANTHROPIC_MODEL"].as_str().unwrap().into(),
        )
    } else if app == AppId::ClaudeDesktop {
        let value: Value = serde_json::from_slice(&bytes).unwrap();
        (
            format!("{}/v1/messages", value["inferenceGatewayBaseUrl"].as_str().unwrap()),
            value["inferenceGatewayApiKey"].as_str().unwrap().into(),
            value["inferenceModels"][0]["name"].as_str().unwrap().into(),
        )
    } else {
        let doc: toml_edit::DocumentMut = std::str::from_utf8(&bytes)
            .unwrap()
            .parse()
            .unwrap_or_else(|_| panic!("native Codex TOML must be valid"));
        let provider = doc["model_provider"].as_str().unwrap();
        (
            format!("{}/responses", doc["model_providers"][provider]["base_url"].as_str().unwrap()),
            doc["model_providers"][provider]["experimental_bearer_token"].as_str().unwrap().into(),
            doc["model"].as_str().unwrap().into(),
        )
    }
}
async fn check_ordinary_default(app: AppId, name: &str) {
    use axum::{Json, http::HeaderMap, routing::post};
    use vw_api_types::app_providers::{
        claude::{ClaudeConfig, ClaudeProfile},
        codex_catalog::CodexProfile,
    };
    let mut f = if app == AppId::Claude { Fixture::new() } else { client_fixture(app, name) };
    let upstream = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = upstream.local_addr().unwrap();
    let (sent, mut received) = tokio::sync::mpsc::unbounded_channel();
    let mut routes = Router::new();
    for id in ["alpha", "beta"] {
        let sent = sent.clone();
        let path = if app != AppId::Codex {
            format!("/{id}/v1/messages")
        } else {
            format!("/{id}/v1/responses")
        };
        routes=routes.route(&path,post(move |headers:HeaderMap,Json(body):Json<Value>| {
                let sent=sent.clone();async move {
                    sent.send((id,headers,body)).unwrap();
                    if app!=AppId::Codex {Json(json!({"id":"fixture","type":"message","role":"assistant","content":[{"type":"text","text":"ok"}],"stop_reason":"end_turn","usage":{"input_tokens":1,"output_tokens":1}}))}
                    else {Json(json!({"id":"fixture","object":"response","status":"completed","output":[]}))}
                }
            }));
    }
    let server = tokio::spawn(async move { axum::serve(upstream, routes).await.unwrap() });
    let store = crate::app_providers::AppProviderStore::new(f.db.clone()).unwrap();
    for (id, model) in [("alpha", "first"), ("beta", "second")] {
        if app == AppId::Claude {
            let before = store.claude_record(&tenant(), id).unwrap();
            store
                .claude_save(
                    &tenant(),
                    id,
                    &ClaudeProfile {
                        name: id.into(),
                        config: ClaudeConfig::ThirdParty {
                            base_url: format!("http://{address}/{id}/v1"),
                            api_key: format!("synthetic-private-key-{id}"),
                            api_key_header: true,
                        },
                        model: Some(model.into()),
                    },
                    before.as_ref(),
                    false,
                )
                .unwrap();
        } else if app == AppId::ClaudeDesktop {
            let row = vw_agent_apps::providers::desktop_mapping::import_row(&json!({
                "name":id,"category":"custom","settingsConfig":{"env":{
                    "ANTHROPIC_BASE_URL":format!("http://{address}/{id}/v1"),
                    "ANTHROPIC_API_KEY":format!("synthetic-private-key-{id}"),
                    "ANTHROPIC_MODEL":model
                }}
            }))
            .unwrap()
            .unwrap();
            GatewaySettingsStore::new(f.db.clone())
                .unwrap()
                .upsert(
                    &tenant(),
                    "app_providers.records",
                    &format!("\"claude-desktop\":{id}"),
                    &row.to_string(),
                )
                .unwrap();
        } else {
            let before = store.codex_record(&tenant(), id).unwrap();
            store.codex_save(&tenant(),id,&CodexProfile{name:id.into(),config:format!("model='{model}'\nmodel_provider='relay'\n[model_providers.relay]\nname='Fixture'\nbase_url='http://{address}/{id}/v1'\nwire_api='responses'\n"),api_key:Some(format!("synthetic-private-key-{id}")),preserve_login:false},before.as_ref(),None).unwrap();
        }
    }
    let mut policy: Value = serde_json::from_slice(&std::fs::read(&f.policy).unwrap()).unwrap();
    let grant = &mut policy["principals"][0]["grants"][0];
    grant["routing_manage"] = json!(true);
    grant["routing_read"] = json!(true);
    grant["routing_apps"] = json!([{"project_id":"project","app_id":name}]);
    std::fs::write(&f.policy, policy.to_string()).unwrap();
    let path = native_path(&f, app);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path,if app==AppId::Claude {"{\"env\":{},\"hooks\":{\"user\":true}}"} else if app==AppId::ClaudeDesktop {"{}"} else {"model='second'\nmodel_provider='relay'\n[model_providers.relay]\nbase_url='http://127.0.0.1:1/v1'\nwire_api='responses'\n[projects.fixture]\ntrust_level='trusted'\n"}).unwrap();
    let mut guard = lock_app(app, f.root.path()).unwrap();
    let before = f.tx.state(app).unwrap();
    let mut stack = before.stack.clone();
    stack.enabled = false;
    if !stack.members.contains(&"beta".into()) {
        stack.members.push("beta".into());
        stack.keys.insert("beta".into(), "beta".into());
    }
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
                proxy_route: Some("beta".into()),
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
    let (status, value) = change(&f, name, "default", json!({"providerId":"alpha"})).await;
    assert_eq!(status, 200, "{name}: {value}");
    assert_eq!(value["mode"], "routing");
    let client = reqwest::Client::builder()
        .no_proxy()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .unwrap();
    let send = |config: &(String, String, String)| {
        client.post(&config.0).bearer_auth(&config.1).json(&if app != AppId::Codex {
            json!({"model":config.2,"messages":[{"role":"user","content":"test"}],"max_tokens":8})
        } else {
            json!({"model":config.2,"input":"test"})
        })
    };
    let alpha = native(&f, app);
    assert_eq!(send(&alpha).send().await.unwrap().status(), 200);
    let (id, headers, body) =
        tokio::time::timeout(std::time::Duration::from_secs(10), received.recv())
            .await
            .unwrap()
            .unwrap();
    assert_eq!(id, "alpha");
    assert_eq!(body["model"], "first");
    if app != AppId::Codex {
        assert!(headers["x-api-key"] == "synthetic-private-key-alpha");
    } else {
        assert!(headers["authorization"] == "Bearer synthetic-private-key-alpha");
    }
    let saved = std::fs::read(&path).unwrap();
    let (status, value) = change(&f, name, "members/beta", json!({"enabled":false})).await;
    assert_eq!(status, 200, "{value}");
    assert!(std::fs::read(&path).unwrap() == saved);
    assert_eq!(send(&alpha).send().await.unwrap().status(), 200);
    assert_eq!(received.recv().await.unwrap().0, "alpha");
    let (status, value) = change(&f, name, "default", json!({"providerId":"beta"})).await;
    assert_eq!(status, 200, "{value}");
    assert_eq!(value["defaultProviderId"], "beta");
    let settled = f.tx.state(app).unwrap();
    assert!(!settled.stack.enabled);
    assert!(settled.stack.members.contains(&"beta".into()));
    assert_eq!(settled.pointer.as_ref().unwrap().id, "alpha");
    assert_eq!(send(&alpha).send().await.unwrap().status(), 401);
    assert!(received.try_recv().is_err());
    let beta = native(&f, app);
    assert_eq!(send(&beta).send().await.unwrap().status(), 200);
    let (id, _, body) = received.recv().await.unwrap();
    assert_eq!(id, "beta");
    assert_eq!(body["model"], "second");
    let old_model =
        vw_agent_apps::aggregation::model_ids::encode(app, "alpha", "first", false).unwrap();
    let stale = (beta.0.clone(), beta.1.clone(), old_model);
    assert_eq!(
        send(&stale).send().await.unwrap().status(),
        if app == AppId::ClaudeDesktop { 404 } else { 422 }
    );
    assert!(received.try_recv().is_err());
    let revision = state(&f, name).await["revision"].clone();
    let saved = std::fs::read(&path).unwrap();
    f.routing = Some(std::sync::Arc::new(crate::app_routing::RoutingListener::default()));
    let (status, value) = change(&f, name, "default", json!({"providerId":"alpha"})).await;
    assert_eq!(status, 503, "{value}");
    assert!(f.tx.state(app).unwrap() == settled);
    assert!(std::fs::read(&path).unwrap() == saved);
    assert_eq!(state(&f, name).await["revision"], revision);
    assert!(f.tx.pending(app).unwrap().is_none());
    assert_eq!(send(&beta).send().await.unwrap().status(), 200);
    assert_eq!(received.recv().await.unwrap().0, "beta");
    listener.stop().await.unwrap();
    server.abort();
    assert!(server.await.unwrap_err().is_cancelled());
}

#[tokio::test]
async fn aggregation_t40_ordinary_claude_default_publishes_one_target_and_preserves_failed_preimage()
 {
    check_ordinary_default(AppId::Claude, "claude").await;
}
#[tokio::test]
async fn aggregation_t40_ordinary_codex_default_publishes_one_target_and_preserves_failed_preimage()
{
    check_ordinary_default(AppId::Codex, "codex").await;
}
#[tokio::test]
async fn aggregation_t40_ordinary_desktop_default_publishes_one_target_and_preserves_failed_preimage()
 {
    check_ordinary_default(AppId::ClaudeDesktop, "claude-desktop").await;
}
