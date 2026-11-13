//! Explicit isolated production gateway for manual DeepAgents desktop acceptance.
use axum::Router;
use serde_json::json;
use std::sync::Arc;

#[tokio::test(flavor = "multi_thread")]
#[ignore = "explicit desktop acceptance; waits for a stop file"]
async fn acp_onboarding_deepagents_desktop_gateway_fixture() {
    let root = std::path::PathBuf::from(std::env::var("VW_DEEPAGENTS_WINDOW_ROOT").unwrap());
    assert!(root.is_absolute() && root.is_dir());
    // Match the native acceptance fixture: isolate databases and use real transport.
    struct Restore(Vec<(&'static str, Option<std::ffi::OsString>)>);
    impl Drop for Restore {
        fn drop(&mut self) {
            for (key, value) in &self.0 {
                unsafe {
                    match value {
                        Some(value) => std::env::set_var(key, value),
                        None => std::env::remove_var(key),
                    }
                }
            }
        }
    }
    let _restore = Restore(
        ["VW_MODEL_RECOVERY_TEST_ROOT", "VW_RECOVERY_WINDOW_HTTP"]
            .into_iter()
            .map(|key| (key, std::env::var_os(key)))
            .collect(),
    );
    unsafe {
        std::env::set_var("VW_MODEL_RECOVERY_TEST_ROOT", &root);
        std::env::set_var("VW_RECOVERY_WINDOW_HTTP", "1");
    }
    let workspace = root.join("workspace");
    std::fs::create_dir_all(&workspace).unwrap();
    let entry = std::env::var("SHIJU_DEEPAGENTS_TEST_ENTRY").unwrap();
    vw_acp::verify_reviewed_agent_executable("deepagents", &entry).unwrap();
    let endpoint = std::env::var("SHIJU_DEEPAGENTS_TEST_ENDPOINT").unwrap();
    assert!(endpoint.starts_with("http://127.0.0.1:"));
    let model_port = reqwest::Url::parse(&endpoint).unwrap().port().unwrap();
    let tenant = vw_api_types::id::TenantScope::new("deepagents-window", "validation").unwrap();
    crate::config::update_global_for_tenant(&tenant, json!({
        "providers":{"deepagents-window":{"api":endpoint, "env":["OPENAI_API_KEY"],"models":{"gpt-4o":{"status":"active"}}}},
        "default_provider":"deepagents-window","default_model":"deepagents-window/gpt-4o",
        "memory":{"backend":"none","auto_save":false},
        "acp":{"deepagents":{"command":entry,"args":["--model","openai:gpt-4o","--workspace",workspace],
            "env":{"OPENAI_API_KEY":"local-fixture","OPENAI_BASE_URL":endpoint,"LANGSMITH_TRACING":"false","HOME":workspace}}},
        "agents":{"acp-deepagents":{"enabled":false,"defaults":{
            "permission":{"mode":"fixed","value":{"*":"deny"}},"mcp":{"mode":"fixed","value":[]},"skills":{"mode":"fixed","value":[]}
        }}}
    })).await.unwrap();
    let config = crate::config::get_global_for_tenant(&tenant).await.unwrap();
    let (event_tx, _) = tokio::sync::broadcast::channel(64);
    let state = super::AppState {
        config: Arc::new(parking_lot::Mutex::new(config)),
        provider: Arc::from(
            crate::providers::create_provider_with_url(
                "openai",
                Some("fixture-only"),
                Some(&format!("http://127.0.0.1:{model_port}/v1")),
            )
            .unwrap(),
        ),
        model: "gpt-4o".into(),
        temperature: 0.0,
        mem: Arc::new(crate::memory::NoneMemory::new()),
        auto_save: false,
        pairing: Arc::new(crate::security::pairing::PairingGuard::new(
            true,
            &["deepagents-fixture-token".into()],
        )),
        trust_forwarded_headers: false,
        rate_limiter: Arc::new(super::GatewayRateLimiter::new(1000, 1000, 1000)),
        idempotency_store: Arc::new(super::IdempotencyStore::new(
            std::time::Duration::from_secs(300),
            1000,
        )),
        channels: Default::default(),
        observer: Arc::new(crate::observability::NoopObserver),
        tools_registry: Arc::new(vec![]),
        tools_registry_exec: Arc::new(vec![]),
        multimodal: Default::default(),
        max_iterations: 3,
        event_tx,
        session_query_engines: Default::default(),
        hook_runtime: None,
    };
    let gateway_router =
        super::bootstrap::client_router(state.clone(), Router::new(), Router::new())
            .with_state(state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let gateway = tokio::spawn(async move {
        axum::serve(listener, gateway_router).await.unwrap();
    });
    std::fs::write(
        root.join("ready.json"),
        json!({"gateway_port":address.port(),"workspace":workspace}).to_string(),
    )
    .unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(900), async {
        while !root.join("stop").exists() {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
    })
    .await
    .expect("desktop acceptance must stop its fixture");
    gateway.abort();
    let _ = gateway.await;
}
