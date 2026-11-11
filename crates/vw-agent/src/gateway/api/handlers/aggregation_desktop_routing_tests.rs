use super::*;

#[tokio::test]
async fn aggregation_t46_desktop_discovery_and_messages_use_one_scoped_listener() {
    use crate::app_routing::{
        RoutingListener, SharedTransport, VerifiedRouteBinding,
        aggregation_routing::AggregationTarget,
        desktop_routing::{DesktopPublication, DesktopRouteTarget},
        dispatch::JsonTarget,
    };
    use axum::{Json, http::HeaderMap, response::IntoResponse, routing::post};
    use vw_agent_apps::providers::{
        claude_desktop::{
            PROFILE_ID,
            publication::{self, DesktopInstallation, DesktopTarget},
        },
        desktop_mapping::catalog::{DesktopMappingMember, DesktopModelCatalog},
        mode_state::StackState,
    };
    use vw_api_types::app_providers::{
        claude_desktop::{ClaudeDesktopConfig, ClaudeDesktopProfile},
        desktop_mapping::{DesktopMapping, DesktopMappingMode, DesktopModelRoute},
    };
    let f = Fixture::new();
    let (send, mut received) = tokio::sync::mpsc::unbounded_channel();
    let upstream = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let upstream_address = upstream.local_addr().unwrap();
    let native_send = send.clone();
    let router=Router::new().route("/custom-native",post(move |headers:HeaderMap,Json(body):Json<Value>|{let send=native_send.clone();async move{
  send.send(("alpha",headers,body)).unwrap();Json(json!({"id":"native","type":"message","role":"assistant","content":[{"type":"text","text":"native"}],"model":"upstream-one","stop_reason":"end_turn","usage":{"input_tokens":1,"output_tokens":1}}))
 }})).route("/v1/chat/completions",post(move |headers:HeaderMap,Json(body):Json<Value>|{let send=send.clone();async move{
  let streaming=body["stream"]==true;send.send(("beta",headers,body)).unwrap();
  if streaming {
    let first=json!({"id":"chat","object":"chat.completion.chunk","model":"upstream-one","choices":[{"index":0,"delta":{"role":"assistant","content":"streamed"},"finish_reason":null}]});
    let last=json!({"id":"chat","object":"chat.completion.chunk","model":"upstream-one","choices":[{"index":0,"delta":{},"finish_reason":"stop"}],"usage":{"prompt_tokens":2,"completion_tokens":3}});
    return ([("content-type","text/event-stream")],format!("data: {first}\n\ndata: {last}\n\ndata: [DONE]\n\n")).into_response();
  }
  Json(json!({"id":"chat","choices":[{"message":{"role":"assistant","content":"converted"},"finish_reason":"stop"}],"usage":{"prompt_tokens":2,"completion_tokens":3}})).into_response()
 }}));
    let task = tokio::spawn(async move { axum::serve(upstream, router).await.unwrap() });
    let listener = RoutingListener::default();
    let address = listener.start(0).unwrap();
    let binding = VerifiedRouteBinding::new(
        ProjectScope::new(tenant(), "project".into()).unwrap(),
        "host".into(),
        AppId::ClaudeDesktop,
        "saved-aggregation".into(),
        1,
    );
    let mapping = DesktopMapping {
        mode: DesktopMappingMode::Proxy,
        routes: [(
            "claude-sonnet-5".into(),
            DesktopModelRoute {
                model: "upstream-one".into(),
                label_override: Some("Fixture model".into()),
                supports_1m: true,
            },
        )]
        .into(),
    };
    let catalog = || {
        DesktopModelCatalog::new(
            DesktopMappingMember {
                provider_id: "alpha",
                key: "primary",
                name: "Alpha",
                mapping: &mapping,
            },
            &[DesktopMappingMember {
                provider_id: "beta",
                key: "secondary",
                name: "Beta",
                mapping: &mapping,
            }],
        )
        .unwrap()
    };
    let native_catalog = catalog();
    let models = native_catalog.models();
    let beta_id = models.iter().find(|model| model.provider_id == "beta").unwrap().id.clone();
    let before = f.tx.state(AppId::ClaudeDesktop).unwrap();
    let mut candidate = before.clone();
    candidate.stack = StackState {
        enabled: true,
        members: vec!["alpha".into(), "beta".into()],
        keys: [("primary".into(), "alpha".into()), ("secondary".into(), "beta".into())].into(),
    };
    candidate.stack_tenant = Some(tenant());
    candidate.mode_tenant = Some(tenant());
    candidate.mode = Some(ModeState {
        mode: Mode::Proxy,
        attached: true,
        proxy_route: Some("alpha".into()),
        contract: None,
    });
    let target = |id: &str, path: &str| DesktopRouteTarget {
        protocol: if id == "alpha" {
            crate::model_relay::generation::GenerationProtocol::Messages
        } else {
            crate::model_relay::generation::GenerationProtocol::Chat
        },
        route: AggregationTarget {
            messages_api_key: id == "alpha",
            binding: binding.clone(),
            revision: "revision-one".into(),
            provider_id: id.into(),
            target: JsonTarget {
                #[cfg(target_os = "macos")]
                managed: None,
                url: format!("http://{upstream_address}/{path}").parse().unwrap(),
                authorization: if id == "alpha" {
                    "fixture-alpha".parse().unwrap()
                } else {
                    format!("Bearer fixture-{id}").parse().unwrap()
                },
                transport: SharedTransport::new().unwrap(),
                request_options: Default::default(),
            },
        },
    };
    let publication = DesktopPublication::new(
        binding.clone(),
        "revision-one".into(),
        candidate.clone(),
        catalog(),
        target("alpha", "custom-native"),
        vec![target("beta", "v1/chat/completions")],
    )
    .unwrap();
    let mut token = String::new();
    let handle = listener
        .publish_desktop(publication, None, |actual, secret| {
            assert_eq!(actual, address);
            token = secret.into();
            let gateway = ClaudeDesktopProfile {
                name: "Gateway".into(),
                config: ClaudeDesktopConfig::ThirdParty {
                    base_url: format!("http://{actual}/claude-desktop"),
                    auth_token: secret.into(),
                },
            };
            publication::run(
                f.root.path(),
                DesktopInstallation::Installed,
                &f.tx,
                &tenant(),
                &before,
                DesktopTarget::Route {
                    gateway: &gateway,
                    catalog: &native_catalog,
                    stack: Some(candidate.stack.clone()),
                },
                || true,
            )
            .map_err(|_| crate::app_routing::RouteCommitError {
                status: StatusCode::CONFLICT,
                recovery_pending: true,
            })
        })
        .unwrap();
    let client = reqwest::Client::builder()
        .no_proxy()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .unwrap();
    let url = format!("http://{address}/claude-desktop");
    let discovery =
        client.get(format!("{url}/v1/models")).bearer_auth(&token).send().await.unwrap();
    assert_eq!(discovery.status(), 200);
    let discovery: Value = discovery.json().await.unwrap();
    assert_eq!(discovery["data"][0]["display_name"], "Fixture model");
    assert!(discovery["data"].as_array().unwrap().iter().any(|m| m["id"] == beta_id));
    for (model, provider) in [("claude-sonnet-5", "alpha"), (beta_id.as_str(), "beta")] {
        let response=client.post(format!("{url}/v1/messages")).bearer_auth(&token).json(&json!({"model":model,"max_tokens":32,"messages":[{"role":"user","content":"hello"}]})).send().await.unwrap();
        assert_eq!(response.status(), 200);
        let body: Value = response.json().await.unwrap();
        assert_eq!(body["type"], "message");
        assert_eq!(body["model"], model);
        let (actual, headers, body) = received.try_recv().unwrap();
        assert_eq!(actual, provider);
        assert_eq!(body["model"], "upstream-one");
        if provider == "alpha" {
            assert_eq!(headers["x-api-key"], "fixture-alpha");
            assert!(!headers.contains_key("authorization"));
        } else {
            assert_eq!(headers["authorization"], format!("Bearer fixture-{provider}"));
        }
    }
    let response=client.post(format!("{url}/v1/messages")).bearer_auth(&token).json(&json!({"model":beta_id,"max_tokens":32,"stream":true,"messages":[{"role":"user","content":"stream"}]})).send().await.unwrap();
    assert_eq!(response.status(), 200);
    let stream = response.text().await.unwrap();
    assert!(stream.contains("streamed"));
    assert!(stream.contains("message_stop"));
    assert!(!stream.contains("\"type\":\"error\""));
    assert!(stream.contains(&beta_id));
    let (provider, _, body) = received.try_recv().unwrap();
    assert_eq!(provider, "beta");
    assert_eq!(body["model"], "upstream-one");
    assert_eq!(body["stream"], true);
    let settings = GatewaySettingsStore::new(f.db.clone()).unwrap();
    for (id, format) in [("alpha", "anthropic"), ("beta", "openai_chat")] {
        settings.upsert(&tenant(),"app_providers.records",&format!("\"claude-desktop\":{id}"),&json!({"name":id,"category":"custom","settingsConfig":{"env":{"ANTHROPIC_BASE_URL":if id=="alpha" {format!("http://{upstream_address}/custom-native")}else{format!("http://{upstream_address}")},"ANTHROPIC_AUTH_TOKEN":format!("fixture-{id}")}},"meta":{"apiFormat":format,"isFullUrl":id=="alpha","claudeDesktopMode":"proxy","claudeDesktopModelRoutes":{"claude-sonnet-5":{"model":"upstream-one","labelOverride":"Fixture model"}}}}).to_string()).unwrap();
    }
    let mut policy: Value = serde_json::from_slice(&fs::read(&f.policy).unwrap()).unwrap();
    let grant = &mut policy["principals"][0]["grants"][0];
    grant["routing_read"] = json!(true);
    grant["routing_manage"] = json!(true);
    grant["routing_apps"] = json!([{"project_id":"project","app_id":"claude-desktop"}]);
    fs::write(&f.policy, policy.to_string()).unwrap();
    let (status, form) = f
        .request(
            "POST",
            "/app-providers/claude-desktop/beta/form/read",
            json!({"targetHostId":"host","projectId":"project"}),
            true,
        )
        .await;
    assert_eq!(status, 200, "{form}");
    let control_listener = std::sync::Arc::new(listener);
    let router = super::super::app_routing::configure_with_policy(
        f.router(),
        &control_listener,
        f.policy.clone(),
    )
    .await;
    let request=Request::builder().method("PUT").uri("/app-providers/claude-desktop/beta/model-mapping").header(PLATFORM_ID_HEADER,"p").header(COMPANY_ID_HEADER,"c").header("authorization",format!("Bearer {TOKEN}")).header("content-type","application/json").body(Body::from(json!({"targetHostId":"host","projectId":"project","expectedRevision":form["revision"],"mapping":{"mode":"proxy","routes":{"claude-sonnet-5":{"model":"upstream-two","labelOverride":"Updated member"}}}}).to_string())).unwrap();
    let response = router.oneshot(request).await.unwrap();
    let status = response.status();
    let saved: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 65536).await.unwrap()).unwrap();
    assert_eq!(status, 200, "{saved}");
    assert_ne!(saved["revision"], form["revision"]);
    let profile_path = f
        .root
        .path()
        .join(format!("Library/Application Support/Claude-3p/configLibrary/{PROFILE_ID}.json"));
    let profile: Value = serde_json::from_slice(&fs::read(&profile_path).unwrap()).unwrap();
    let replacement = profile["inferenceGatewayApiKey"].as_str().unwrap().to_owned();
    assert_eq!(
        client.get(format!("{url}/v1/models")).bearer_auth(&token).send().await.unwrap().status(),
        401
    );
    token = replacement;
    let revised: Value = client
        .get(format!("{url}/v1/models"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let revised_id = revised["data"]
        .as_array()
        .unwrap()
        .iter()
        .find(|model| {
            model["display_name"].as_str().is_some_and(|name| name.contains("Updated member"))
        })
        .unwrap()["id"]
        .as_str()
        .unwrap();
    let response=client.post(format!("{url}/v1/messages")).bearer_auth(&token).json(&json!({"model":revised_id,"max_tokens":32,"messages":[{"role":"user","content":"updated"}]})).send().await.unwrap();
    assert_eq!(response.status(), 200);
    let result: Value = response.json().await.unwrap();
    assert_eq!(result["model"], revised_id);
    let (provider, _, body) = received.try_recv().unwrap();
    assert_eq!(provider, "beta");
    assert_eq!(body["model"], "upstream-two");
    let row: Value = serde_json::from_str(
        &settings
            .get(&tenant(), "app_providers.records", "\"claude-desktop\":beta")
            .unwrap()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(row["meta"]["claudeDesktopModelRoutes"]["claude-sonnet-5"]["model"], "upstream-two");
    let response=client.post(format!("{url}/v1/messages")).bearer_auth(&token).json(&json!({"model":"claude-sonnet-5","max_tokens":32,"messages":[{"role":"user","content":"native custom URL"}]})).send().await.unwrap();
    assert_eq!(response.status(), 200);
    let result: Value = response.json().await.unwrap();
    assert_eq!(result["model"], "claude-sonnet-5");
    let (provider, headers, body) = received.try_recv().unwrap();
    assert_eq!(provider, "alpha");
    assert_eq!(headers["authorization"], "Bearer fixture-alpha");
    assert_eq!(body["model"], "upstream-one");
    let listener = control_listener;
    for (path, model, expected) in [
        ("/claude-desktop/v1/messages", "claude-fable-ccs-999999", StatusCode::NOT_FOUND),
        ("/v1/messages", "claude-sonnet-5", StatusCode::NOT_FOUND),
    ] {
        assert_eq!(client.post(format!("http://{address}{path}")).bearer_auth(&token).json(&json!({"model":model,"max_tokens":32,"messages":[{"role":"user","content":"hello"}]})).send().await.unwrap().status(),expected);
    }
    assert!(received.try_recv().is_err());
    let profile: Value = serde_json::from_slice(
        &fs::read(f.root.path().join(format!(
            "Library/Application Support/Claude-3p/configLibrary/{PROFILE_ID}.json"
        )))
        .unwrap(),
    )
    .unwrap();
    assert_eq!(
        profile["inferenceModels"].as_array().unwrap().len(),
        discovery["data"].as_array().unwrap().len()
    );
    // A stale revocation handle cannot revoke the mapping's replacement token.
    listener.revoke_aggregation(&handle).unwrap();
    assert_eq!(
        client.get(format!("{url}/v1/models")).bearer_auth(&token).send().await.unwrap().status(),
        200
    );
    let current = f.tx.state(AppId::ClaudeDesktop).unwrap();
    let scope = vw_api_types::agent_apps_high_availability::FailoverQueueScope {
        project: ProjectScope::new(tenant(), "project".into()).unwrap(),
        target_host_id: "host".into(),
        app_id: AppId::ClaudeDesktop,
    };
    listener
        .restore_provider(&scope, || {
            publication::run(
                f.root.path(),
                DesktopInstallation::Installed,
                &f.tx,
                &tenant(),
                &current,
                DesktopTarget::Restore(None),
                || true,
            )
            .map_err(|_| crate::app_routing::RouteCommitError {
                status: StatusCode::CONFLICT,
                recovery_pending: true,
            })
        })
        .unwrap();

    assert_eq!(
        client.get(format!("{url}/v1/models")).bearer_auth(&token).send().await.unwrap().status(),
        401
    );
    listener.stop().await.unwrap();
    task.abort();
    let _ = task.await;
}
