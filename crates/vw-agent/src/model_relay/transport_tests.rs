use super::transport::credential;
use crate::{auth, provider::provider};
use axum::http::StatusCode;

#[test]
fn acp_model_source_messages_selects_the_official_alibaba_token_plan_endpoint() {
    let mut api = provider::ApiInfo {
        protocols: None,
        id: "deepseek-v4.1-flash-1017".into(),
        url: "https://token-plan.cn-beijing.maas.aliyuncs.com/compatible-mode/v1".into(),
        adapter: "openai-compatible".into(),
    };
    for host in
        ["token-plan.cn-beijing.maas.aliyuncs.com", "token-plan.ap-southeast-1.maas.aliyuncs.com"]
    {
        for path in ["/compatible-mode/v1", "/compatible-mode/v1/chat/completions/"] {
            api.url = format!("https://{host}{path}");
            assert_eq!(
                super::transport::official_alibaba_messages_url(&api).unwrap().as_str(),
                format!("https://{host}/apps/anthropic/v1/messages")
            );
        }
    }
    for url in [
        "https://token-plan.cn-beijing.maas.aliyuncs.com.example.invalid/compatible-mode/v1",
        "http://token-plan.cn-beijing.maas.aliyuncs.com/compatible-mode/v1",
        "https://token-plan.cn-beijing.maas.aliyuncs.com:8443/compatible-mode/v1",
        "https://user@token-plan.cn-beijing.maas.aliyuncs.com/compatible-mode/v1",
        "https://token-plan.cn-beijing.maas.aliyuncs.com/compatible-mode/v1?custom=1",
        "https://token-plan.cn-beijing.maas.aliyuncs.com/compatible-mode/v1#custom",
        "https://token-plan.cn-beijing.maas.aliyuncs.com/custom/v1",
    ] {
        api.url = url.into();
        assert!(super::transport::official_alibaba_messages_url(&api).is_none());
    }
    api.url = "https://token-plan.cn-beijing.maas.aliyuncs.com/compatible-mode/v1".into();
    api.adapter = "google".into();
    assert!(super::transport::official_alibaba_messages_url(&api).is_none());
}

#[test]
fn acp_model_source_messages_selects_the_official_deepseek_native_endpoint() {
    let mut api = provider::ApiInfo {
        protocols: None,
        id: "deepseek-flash".into(),
        url: "https://api.deepseek.com/v1/chat/completions".into(),
        adapter: "openai-compatible".into(),
    };
    let url = super::transport::official_deepseek_messages_url(&api).unwrap();
    assert_eq!(url.as_str(), "https://api.deepseek.com/anthropic/v1/messages");
    for url in [
        "https://api.deepseek.com.example.invalid/v1",
        "http://api.deepseek.com/v1",
        "https://api.deepseek.com/beta",
        "https://api.deepseek.com/v1?custom=1",
        "https://api.deepseek.com:8443/v1",
    ] {
        api.url = url.into();
        assert!(super::transport::official_deepseek_messages_url(&api).is_none());
    }
    api.url = "https://api.deepseek.com".into();
    api.id = "claude-opus-latest".into();
    assert!(super::transport::official_deepseek_messages_url(&api).is_none());
}

fn chat_model(provider_id: &str, id: &str, url: &str) -> provider::Model {
    provider::Model {
        id: id.into(),
        provider_id: provider_id.into(),
        api: provider::ApiInfo {
            protocols: None,
            id: id.into(),
            url: url.into(),
            adapter: "openai-compatible".into(),
        },
        name: id.into(),
        family: None,
        capabilities: provider::default_model_capabilities(),
        cost: provider::ModelCost {
            input: 0.0,
            output: 0.0,
            cache: provider::ModelCostCache { read: 0.0, write: 0.0 },
            experimental_over_200k: None,
        },
        limit: provider::ModelLimit { context: 4096, input: None, output: 1024 },
        status: "active".into(),
        options: Default::default(),
        headers: Default::default(),
        release_date: String::new(),
        variants: Default::default(),
    }
}

#[test]
fn acp_model_source_messages_chat_matches_the_client_thinking_mode() {
    use vw_shared::provider::reasoning::ReasoningOption;
    let request = serde_json::json!({"model":"selected","messages":[]});
    for (provider, id, url, field, expected) in [
        (
            "deepseek",
            "deepseek-flash",
            "https://api.deepseek.com/v1",
            "thinking",
            serde_json::json!({"type":"disabled"}),
        ),
        (
            "alibaba-token-plan-cn",
            "deepseek-v4.1-flash-1017",
            "https://token-plan.cn-beijing.maas.aliyuncs.com/compatible-mode/v1",
            "enable_thinking",
            serde_json::json!(false),
        ),
        (
            "custom",
            "gpt-5.4",
            "https://example.invalid/v1",
            "reasoning_effort",
            serde_json::json!("none"),
        ),
    ] {
        let mut model = chat_model(provider, id, url);
        model.capabilities.reasoning = true;
        model.capabilities.reasoning_options = Some(vec![ReasoningOption::Toggle]);
        let mut body = request.clone();
        super::transport::configure_messages_chat(&mut body, &model);
        assert_eq!(body[field], expected);
    }
    let mut model = chat_model("custom", "custom-model", "https://example.invalid/v1");
    let mut body = request.clone();
    super::transport::configure_messages_chat(&mut body, &model);
    assert_eq!(body, request);
    model.capabilities.reasoning_options = Some(vec![ReasoningOption::Toggle]);
    super::transport::configure_messages_chat(&mut body, &model);
    assert_eq!(body, request);
    model.api.id = "deepseek-v4.1-flash-1017".into();
    model.provider_id = "alibaba-token-plan-cn".into();
    model.capabilities.reasoning_options = Some(Vec::new());
    super::transport::configure_messages_chat(&mut body, &model);
    assert_eq!(body, request);
}

#[test]
fn acp_model_source_relay_chat_oauth_expiry_and_revocation_never_use_cached_keys() {
    let info = provider::Info {
        id: "relay-oauth-fixture".into(),
        name: "fixture".into(),
        source: provider::ProviderSource::Api,
        env: Vec::new(),
        key: Some("stale-api-secret".into()),
        options: Default::default(),
        models: Default::default(),
    };
    let oauth = |expires| {
        Some(auth::Info::Oauth(auth::OauthInfo {
            access: "fresh-access".into(),
            refresh: "refresh-not-a-bearer".into(),
            expires,
            account_id: None,
            enterprise_url: None,
        }))
    };
    assert_eq!(
        credential(&info, oauth(chrono::Utc::now().timestamp_millis() + 60_000)).unwrap(),
        "fresh-access"
    );
    assert_eq!(credential(&info, oauth(0)).unwrap_err(), StatusCode::UNAUTHORIZED);
    assert_eq!(credential(&info, None).unwrap_err(), StatusCode::UNAUTHORIZED);
    assert_eq!(
        credential(
            &info,
            Some(auth::Info::Api(auth::ApiInfo { key: "rotated-api-secret".into() }))
        )
        .unwrap(),
        "rotated-api-secret"
    );
}

#[tokio::test]
async fn model_protocol_012_relay_native_conversion_and_credential_isolation() {
    use super::{ProviderRelayBackend, RelayBinding, RelayIdentity, RelayServer};
    use axum::{Router, http::HeaderMap, routing::post};
    use serde_json::{Value, json};
    use std::{sync::Arc, time::Duration};
    let _lock = crate::test_support::global_async_env_lock().lock().await;
    let temp = tempfile::tempdir().unwrap();
    let previous = std::env::var_os("VIBEWINDOW_CONFIG_DIR");
    struct Restore(Option<std::ffi::OsString>);
    impl Drop for Restore {
        fn drop(&mut self) {
            unsafe {
                match self.0.take() {
                    Some(value) => std::env::set_var("VIBEWINDOW_CONFIG_DIR", value),
                    None => std::env::remove_var("VIBEWINDOW_CONFIG_DIR"),
                }
            }
        }
    }
    let _restore = Restore(previous);
    struct RestoreKey(Option<std::ffi::OsString>, Option<String>);
    impl Drop for RestoreKey {
        fn drop(&mut self) {
            crate::env::remove("SHIJU_PROTOCOL012_TEST_KEY");
            if let Some(value) = self.1.take() {
                crate::env::set("SHIJU_PROTOCOL012_TEST_KEY", value);
            }
            unsafe {
                match self.0.take() {
                    Some(value) => std::env::set_var("SHIJU_PROTOCOL012_TEST_KEY", value),
                    None => std::env::remove_var("SHIJU_PROTOCOL012_TEST_KEY"),
                }
            }
        }
    }
    let _key = RestoreKey(
        std::env::var_os("SHIJU_PROTOCOL012_TEST_KEY"),
        crate::env::get("SHIJU_PROTOCOL012_TEST_KEY"),
    );
    crate::env::set("SHIJU_PROTOCOL012_TEST_KEY", "selected-account-secret");
    unsafe {
        std::env::set_var("SHIJU_PROTOCOL012_TEST_KEY", "selected-account-secret");
    }
    unsafe {
        std::env::set_var("VIBEWINDOW_CONFIG_DIR", temp.path().join("config"));
    }
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let (record, mut requests) = tokio::sync::mpsc::unbounded_channel();
    let native_record = record.clone();
    let router = Router::new()
        .route("/custom/messages", post(move |headers: HeaderMap, axum::Json(body): axum::Json<Value>| {
            let record = native_record.clone();
            async move {
                record.send(("native", headers, body)).unwrap();
                axum::Json(json!({"id":"msg","type":"message","role":"assistant","content":[{"type":"text","text":"native"}],"stop_reason":"end_turn","usage":{"input_tokens":1,"output_tokens":1}}))
            }
        }))
        .route("/custom/chat/completions", post(move |headers: HeaderMap, axum::Json(body): axum::Json<Value>| {
            let record = record.clone();
            async move {
                record.send(("chat", headers, body)).unwrap();
                axum::Json(json!({"id":"chat","choices":[{"message":{"role":"assistant","content":"converted"},"finish_reason":"stop"}],"usage":{"prompt_tokens":1,"completion_tokens":1}}))
            }
        }));
    let upstream = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let tenant = vw_api_types::id::TenantScope::new("protocol012", "fixture").unwrap();
    let root = temp.path().join("project");
    std::fs::create_dir(&root).unwrap();
    let (project, _) = crate::project::from_directory(&tenant, &root).await.unwrap();
    let endpoint = |protocol: &str, url: String, auth: Value| {
        json!({
            "protocol":protocol,"url":url,"deployment":"fixture", "authentication":auth,
            "models":{"kind":"models","ids":["wire-model"]},
            "evidence":[{"kind":"configuration","source":"fixture","reference":"explicit deployment","checked_at":"2026-11-11"}]
        })
    };
    let chat = endpoint(
        "chat",
        format!("http://{address}/custom/chat/completions"),
        json!({"kind":"bearer"}),
    );
    let native =
        endpoint("messages", format!("http://{address}/custom/messages"), json!({"kind":"bearer"}));
    let foreign =
        endpoint("messages", "http://127.0.0.1:1/custom/messages".into(), json!({"kind":"bearer"}));
    let mut native_key = native.clone();
    native_key["authentication"] = json!({"kind":"api_key_header","name":"x-api-key"});
    let mut unsupported = native.clone();
    unsupported["authentication"] = json!({"kind":"aws_sigv4"});
    let mut wrong_scope = chat.clone();
    wrong_scope["models"] = json!({"kind":"models","ids":["another-model"]});
    let models = json!({
        "native":{"id":"wire-model","status":"active","protocols":[chat.clone(),native]},
        "converted":{"id":"wire-model","status":"active","protocols":[chat],"reasoning":false},
        "native-key":{"id":"wire-model","status":"active","protocols":[native_key]},
        "unsupported":{"id":"wire-model","status":"active","protocols":[unsupported]},
        "foreign":{"id":"wire-model","status":"active","protocols":[foreign]},
        "empty":{"id":"wire-model","status":"active","protocols":[]},
        "wrong-scope":{"id":"wire-model","status":"active","protocols":[wrong_scope]}
    });
    crate::config::update_project_for_tenant(
        &tenant,
        &root,
        json!({"providers":{"protocol-fixture":{
            "api":format!("http://{address}/v1"),"env":["SHIJU_PROTOCOL012_TEST_KEY"],"models":models
        }}}),
    )
    .await
    .unwrap();
    let context = crate::project::instance::Context {
        directory: root.to_string_lossy().into(),
        worktree: project.worktree.clone(),
        project: project.clone(),
    };
    let client =
        reqwest::Client::builder().no_proxy().timeout(Duration::from_secs(10)).build().unwrap();
    for (model, expected, path) in [
        ("native", Some("native"), "/v1/messages"),
        ("converted", Some("chat"), "/v1/messages"),
        ("native-key", Some("native"), "/v1/messages"),
        ("unsupported", None, "/v1/messages"),
        ("foreign", None, "/v1/messages"),
        ("empty", None, "/v1/messages"),
        ("wrong-scope", None, "/v1/messages"),
        ("converted", Some("chat"), "/v1/responses"),
        ("converted", Some("chat"), "/v1/chat/completions"),
    ] {
        let binding = RelayBinding::new(
            RelayIdentity {
                tenant: tenant.clone(),
                project: project.id.clone(),
                session: if path == "/v1/responses" { "fixture:codex" } else { "fixture:claude" }
                    .into(),
                provider: "protocol-fixture".into(),
                model: model.into(),
                generation: 1,
            },
            Duration::from_secs(60),
        )
        .unwrap();
        let server = RelayServer::start(
            binding.clone(),
            Arc::new(ProviderRelayBackend::new().unwrap()),
            context.clone(),
        )
        .await
        .unwrap();
        let metadata = client
            .get(format!("http://{}/v1beta/models", server.address()))
            .bearer_auth(binding.bearer_token())
            .send()
            .await
            .unwrap();
        assert_eq!(
            metadata.status(),
            if matches!(model, "native" | "converted" | "native-key") {
                StatusCode::OK
            } else {
                StatusCode::UNPROCESSABLE_ENTITY
            }
        );
        assert!(requests.try_recv().is_err(), "metadata must not contact the upstream");
        let body = match path {
            "/v1/responses" => json!({"model":model,"input":"hello","reasoning":{"effort":"none"}}),
            "/v1/chat/completions" => {
                json!({"model":model,"messages":[{"role":"assistant","content":"previous","reasoning_content":""},{"role":"user","content":"hello"}]})
            }
            _ => {
                json!({"model":model,"max_tokens":16,"messages":[{"role":"user","content":"hello"}]})
            }
        };
        let response = client
            .post(format!("http://{}{path}", server.address()))
            .bearer_auth(binding.bearer_token())
            .json(&body)
            .send()
            .await
            .unwrap();
        if let Some(expected) = expected {
            assert!(response.status().is_success(), "{model}: {}", response.status());
            let result: Value = response.json().await.unwrap();
            match path {
                "/v1/responses" => {
                    assert_eq!(result["status"], "completed");
                    assert_eq!(result["output"][0]["content"][0]["text"], "converted");
                }
                "/v1/chat/completions" => {
                    assert_eq!(result["choices"][0]["finish_reason"], "stop");
                    assert_eq!(result["choices"][0]["message"]["content"], "converted");
                }
                _ => {
                    assert_eq!(result["stop_reason"], "end_turn");
                    assert_eq!(
                        result["content"][0]["text"],
                        if expected == "native" { "native" } else { "converted" }
                    );
                }
            }
            let (actual_protocol, headers, body) = requests.try_recv().unwrap();
            assert_eq!(actual_protocol, expected);
            assert_eq!(body["model"], "wire-model");
            if path == "/v1/chat/completions" {
                assert_eq!(body["messages"][0]["reasoning_content"], "");
            }
            if model == "native-key" {
                assert_eq!(headers["x-api-key"], "selected-account-secret");
                assert!(!headers.contains_key("authorization"));
            } else {
                assert_eq!(headers["authorization"], "Bearer selected-account-secret");
                assert!(!headers.contains_key("x-api-key"));
            }
        } else {
            assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY, "{model}");
            assert!(requests.try_recv().is_err());
        }
        server.shutdown().await.unwrap();
    }
    upstream.abort();
    assert!(upstream.await.unwrap_err().is_cancelled());
}
