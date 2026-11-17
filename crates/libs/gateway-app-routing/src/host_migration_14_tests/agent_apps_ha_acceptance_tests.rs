//! Controlled macOS acceptance: a standalone gateway owns routing without Iced.
use crate::host_migration_7_tests::fixture::{HOST, MANAGER, Lifecycle, write_policy};
use {app_provider_services::AppProviderStore, agent_database::GlobalDatabase};
use axum::{Json, Router, routing::post};
use reqwest::{Client, Method, StatusCode};
use serde_json::{Value, json};
use std::{fs, os::unix::fs::PermissionsExt, sync::{Arc, atomic::{AtomicBool, AtomicUsize, Ordering}}, time::Duration};
use vw_api_app_identity::AppId::Claude;
use vw_api_app_failover::FailoverPolicy;
use vw_api_app_failover::FailoverQueueQuery;
use vw_api_app_provider_base::claude::ClaudeConfig;
use vw_api_app_provider_base::claude::ClaudeProfile;
use vw_api_transport::id::TenantScope;

struct Servers(Vec<tokio::task::JoinHandle<()>>);
impl Drop for Servers {
    fn drop(&mut self) { for task in &self.0 { task.abort(); } }
}

#[tokio::test]
async fn ha_macos_headless_routing_acceptance() -> anyhow::Result<()> {
    const CHILD: &str = "SHIJU_HA_ACCEPTANCE_CHILD";
    if std::env::var_os(CHILD).is_none() {
        let root = tempfile::tempdir()?;
        let output = std::process::Command::new(std::env::current_exe()?)
            .args(["--exact", "host_migration_14_tests::agent_apps_ha_acceptance_tests::ha_macos_headless_routing_acceptance", "--nocapture"])
            .env(CHILD, "1").env("VIBEWINDOW_TEST_ROOT", root.path())
            .env_remove("VIBEWINDOW_TEST_HOME").output()?;
        eprint!("{}{}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));
        assert!(output.status.success(), "isolated acceptance failed: {}", output.status);
        return Ok(());
    }
    let mut servers = Servers(Vec::new());
    let failures = Arc::new(AtomicBool::new(false));
    let hits = Arc::new(AtomicUsize::new(0));
    let connections = Arc::new(AtomicUsize::new(0));
    let upstream = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let endpoint = format!("http://{}", upstream.local_addr()?);
    let (record, fail) = (hits.clone(), failures.clone());
    servers.0.push(tokio::spawn(async move {
        axum::serve(upstream, Router::new().route("/v1/messages", post(
            move |headers: axum::http::HeaderMap, Json(body): Json<Value>| {
                let (record, fail) = (record.clone(), fail.clone());
                async move {
                    assert_eq!(headers["x-api-key"], "backup-key");
                    assert_eq!(body["model"], "fixture");
                    record.fetch_add(1, Ordering::SeqCst);
                    if fail.load(Ordering::SeqCst) {
                        (StatusCode::BAD_GATEWAY, Json(json!({"error":"controlled failure"})))
                    } else {
                        (StatusCode::OK, Json(json!({"id":"fixture","type":"message",
                            "role":"assistant","model":"fixture","content":[{"type":"text","text":"ok"}],
                            "stop_reason":"end_turn","usage":{"input_tokens":1,"output_tokens":1}})))
                    }
                }
            }
        ))).await.unwrap();
    }));
    let broken = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let unavailable = format!("https://{}", broken.local_addr()?);
    let counted = connections.clone();
    servers.0.push(tokio::spawn(async move {
        use tokio::io::AsyncWriteExt;
        loop {
            let (mut socket, _) = broken.accept().await.unwrap();
            counted.fetch_add(1, Ordering::SeqCst);
            let _ = socket.write_all(b"HTTP/1.1 503 Service Unavailable\r\n\r\n").await;
            let _ = socket.shutdown().await;
        }
    }));
    let home = agent_paths::paths().home.clone();
    use sha2::{Digest, Sha256};
    let root = agent_paths::paths().data.join("apps").join(hex::encode(
        Sha256::digest(serde_json::to_vec(&("p", "owner"))?),
    ));
    fs::create_dir_all(&root)?;
    fs::set_permissions(&root, fs::Permissions::from_mode(0o700))?;
    fs::create_dir(home.as_path().join(".claude"))?;
    let policy = home.as_path().join("access.json");
    write_policy(&policy, home.as_path(), 0);
    let mut grants: Value = serde_json::from_slice(&fs::read(&policy)?)?;
    grants["principals"][0]["grants"][0]["routing_apps"] = json!([
        {"project_id":"project","app_id":"claude"}
    ]);
    grants["principals"][0]["grants"][0]["provider_config_write"] = json!(true);
    fs::write(&policy, grants.to_string())?;
    let db = GlobalDatabase::for_home(home.as_path())?;
    let tenant = TenantScope::new("p", "owner")?;
    for (id, base_url) in [("current", unavailable), ("backup", endpoint)] {
        AppProviderStore::new(db.clone()).map_err(anyhow::Error::msg)?.claude_save(
            &tenant, id, &ClaudeProfile {
                name: id.into(),
                config: ClaudeConfig::ThirdParty {
                    base_url, api_key: format!("{id}-key"), api_key_header: true,
                },
                model: Some("fixture".into()),
            }, None, true,
        ).map_err(anyhow::Error::msg)?;
    }
    // Establish the same real direct-provider transaction used by the native
    // mode owner before asking the HTTP API to publish a routing mode.
    {
        use vw_agent_apps::providers::live::{
            commit::{ExecutionHostId, PendingTarget, ProviderTransactions},
            write_guard::lock_app,
        };
        let transactions =
            ProviderTransactions::new(&root, ExecutionHostId::new(HOST).unwrap()).unwrap();
        let mut guard = lock_app(Claude, &home.as_path().join(".claude")).unwrap();
        transactions
            .commit(
                &tenant,
                &mut guard,
                &[],
                PendingTarget { pointer: Some("current".into()), ..Default::default() },
            )
            .unwrap();
    }
    // Compose the same mode publication endpoint and host adapter as the native
    // gateway. The lifecycle-only fixture does not register provider APIs.
    let routing = Arc::new(app_routing_services::RoutingListener::with_database(db.clone()));
    let mode = gateway_app_providers::mode_apply::router_with_policy(Some(policy.clone()))
        .layer(axum::middleware::from_fn(|mut request: axum::extract::Request, next: axum::middleware::Next| async move {
            let control = request.extensions().get::<crate::server::lifecycle_api::RoutingControl>().cloned();
            assert!(control.is_some());
            request.extensions_mut().insert(gateway_app_providers::mode_apply::ModeHost(Arc::new(
                crate::server::provider_mode_host::Host(control, true),
            )));
            next.run(request).await
        }));
    let app = Router::new().nest("/v1", crate::server::router()
        .merge(crate::server::agent_apps_high_availability::router()).merge(mode));
    let app = crate::server::lifecycle_api::configure_with_policy(
        app, &routing, policy.clone(), db.clone(),
        crate::server::lifecycle_api::RoutingPaths {
            home: home.as_path().to_owned(), config: home.as_path().join(".config/shiju"),
            state: home.as_path().join(".local/state/shiju"),
        },
        |database, policy, tenant, host| Lifecycle(agent_gateway::aggregation_lifecycle::AggregationLifecycle { database, policy, tenant, host }),
    ).await.layer(axum::Extension(db.clone()))
        .layer(axum::middleware::from_fn(|mut request: axum::extract::Request, next: axum::middleware::Next| async move {
            let tenant = TenantScope::new(
                request.headers()["x-platform-id"].to_str().unwrap(),
                request.headers()["x-company-id"].to_str().unwrap(),
            ).unwrap();
            request.extensions_mut().insert(tenant);
            next.run(request).await
        }));
    let socket = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let port = socket.local_addr()?.port();
    servers.0.push(tokio::spawn(async move { axum::serve(socket, app).await.unwrap(); }));
    let client = Client::builder().no_proxy().timeout(Duration::from_secs(15)).build()?;
    let sdk = gateway_app_failover::GatewayAppFailoverClient::new(gateway_http::GatewayTransport::new(
        gateway_endpoint::GatewayEndpoint::new("127.0.0.1", port)
            .with_tenant_scope(tenant.clone()),
    ).map_err(anyhow::Error::msg)?);
    let sdk_scope = FailoverQueueQuery {
        project_id: "project".into(), target_host_id: HOST.into(), app_id: Claude,
    };
    let empty = sdk.agent_apps_failover_queue(&sdk_scope, Some(MANAGER)).await?;
    assert!(empty.entries.is_empty());
    assert!(!empty.enabled);
    let request = |method, path: &str| {
        client.request(method, format!("http://127.0.0.1:{port}/v1/{path}"))
            .header("x-platform-id", "p").header("x-company-id", "owner")
            .header("x-agent-apps-token", MANAGER)
    };
    let ha = |method, resource: &str| {
        request(method, &format!("agent-apps/high-availability/{resource}"))
            .query(&[("projectId", "project"), ("targetHostId", HOST), ("appId", "claude")])
    };
    assert_eq!(request(Method::POST, &format!("app-routing/hosts/{HOST}/start"))
        .json(&json!({})).send().await?.status(), StatusCode::OK);
    let applied = request(Method::POST, "app-providers/claude/mode/apply").json(&json!({
        "targetHostId":HOST,"projectId":"project","action":{"kind":"proxy","providerId":"current"}
    })).send().await?;
    assert_eq!(applied.status(), StatusCode::OK);
    let mut version = 0;
    for id in ["current", "backup"] {
        let response = ha(Method::POST, "queue/members")
            .json(&json!({"expectedVersion":version,"providerId":id})).send().await?;
        assert_eq!(response.status(), StatusCode::OK);
        version = response.json::<Value>().await?["version"].as_u64().unwrap();
    }
    let response = ha(Method::GET, "policy").send().await?;
    assert_eq!(response.status(), StatusCode::OK);
    let mut policy: FailoverPolicy = response.json().await?;
    policy.retry_timeout.max_retries = 2;
    policy.circuit_breaker.failure_threshold = 1;
    policy.circuit_breaker.success_threshold = 1;
    policy.circuit_breaker.min_requests = 5;
    policy.circuit_breaker.cooldown_secs = 300;
    assert_eq!(ha(Method::PUT, "policy").json(&policy).send().await?.status(), StatusCode::OK);
    assert_eq!(ha(Method::PUT, "enabled").json(&json!({"expectedVersion":version,"enabled":true}))
        .send().await?.status(), StatusCode::OK);
    let enabled = sdk.agent_apps_failover_queue(&sdk_scope, Some(MANAGER)).await?;
    assert!(enabled.enabled);
    assert_eq!(enabled.entries.len(), 2);
    let config_bytes = fs::read(home.as_path().join(".claude/settings.json"))?;
    let config: Value = serde_json::from_slice(&config_bytes)?;
    let token = config["env"]["ANTHROPIC_AUTH_TOKEN"].as_str().unwrap();
    let url = config["env"]["ANTHROPIC_BASE_URL"].as_str().unwrap();
    let send = || client.post(format!("{}/v1/messages", url.trim_end_matches('/')))
        .header("x-api-key", token).header("x-provider-retries", "100")
        .json(&json!({"model":"fixture","messages":[{"role":"user","content":"hello"}]})).send();
    let response = send().await?;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.json::<Value>().await?["content"][0]["text"], "ok");
    assert_eq!(connections.load(Ordering::SeqCst), 1);
    assert_eq!(hits.load(Ordering::SeqCst), 1, "single budget must not multiply attempts");
    let route = ha(Method::GET, "route").send().await?;
    assert_eq!(route.status(), StatusCode::OK);
    assert_eq!(route.json::<Value>().await?["actualProviderId"], "backup");
    failures.store(true, Ordering::SeqCst);
    assert!(!send().await?.status().is_success());
    assert_eq!(hits.load(Ordering::SeqCst), 2);
    for id in ["current", "backup"] {
        let health = ha(Method::GET, "health").query(&[("providerId", id)]).send().await?;
        assert_eq!(health.status(), StatusCode::OK);
        assert_eq!(health.json::<Value>().await?["state"], "open");
    }
    assert!(!send().await?.status().is_success());
    assert_eq!(hits.load(Ordering::SeqCst), 2, "all-open circuits must reject before sending");
    assert_eq!(connections.load(Ordering::SeqCst), 1);
    // Change the real policy to expire cooldown; the next production request
    // must acquire the half-open probe and close the circuit on success.
    failures.store(false, Ordering::SeqCst);
    policy.circuit_breaker.cooldown_secs = 0;
    assert_eq!(ha(Method::PUT, "policy").json(&policy).send().await?.status(), StatusCode::OK);
    let response = send().await?;
    assert_eq!(response.status(), StatusCode::OK);
    let _: Value = response.json().await?;
    assert_eq!(hits.load(Ordering::SeqCst), 3);
    let health = ha(Method::GET, "health").query(&[("providerId", "backup")]).send().await?;
    assert_eq!(health.status(), StatusCode::OK);
    assert_eq!(health.json::<Value>().await?["state"], "closed");
    assert_eq!(fs::read(home.as_path().join(".claude/settings.json"))?, config_bytes);
    let response = ha(Method::GET, "queue").send().await?;
    assert_eq!(response.status(), StatusCode::OK);
    let queue = response.json::<Value>().await?;
    let response = ha(Method::PUT, "enabled")
        .json(&json!({"expectedVersion":queue["version"],"enabled":false})).send().await?;
    assert_eq!(response.status(), StatusCode::OK);
    let disabled = response.json::<Value>().await?;
    assert_eq!(disabled["enabled"], false);
    assert_eq!(disabled["entries"], queue["entries"]);
    routing.stop().await?;
    eprintln!("controlled headless acceptance: OS={}, arch={}, version={}, backup_http={}, primary_connections={}",
        std::env::consts::OS, std::env::consts::ARCH, env!("CARGO_PKG_VERSION"),
        hits.load(Ordering::SeqCst), connections.load(Ordering::SeqCst));
    for task in servers.0.drain(..) {
        task.abort();
        assert!(task.await.unwrap_err().is_cancelled());
    }
    Ok(())
}
