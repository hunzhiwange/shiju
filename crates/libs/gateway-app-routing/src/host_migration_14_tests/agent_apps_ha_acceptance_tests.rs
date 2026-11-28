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
    exercise_failover(false, "host_migration_14_tests::agent_apps_ha_acceptance_tests::ha_macos_headless_routing_acceptance").await
}

pub(crate) async fn exercise_failover(local: bool, test_name: &str) -> anyhow::Result<()> {
    let host = if local { vw_api_transport::LOCAL_HOST_ID } else { HOST };
    const CHILD: &str = "SHIJU_HA_ACCEPTANCE_CHILD";
    if std::env::var_os(CHILD).is_none() {
        let root = tempfile::tempdir()?;
        let output = std::process::Command::new(std::env::current_exe()?)
            .args(["--exact", test_name, "--nocapture"])
            .env(CHILD, "1").env("VIBEWINDOW_TEST_ROOT", root.path())
            .env_remove("VIBEWINDOW_TEST_HOME").env_remove("SHIJU_AGENT_APPS_ACCESS_FILE").output()?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        eprint!("{stdout}{}", String::from_utf8_lossy(&output.stderr));
        assert!(stdout.contains("running 1 test"), "isolated acceptance did not run");
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
    // Keep an independent routing database to catch handlers that reopen the
    // ambient database instead of using the listener's authorized control.
    let database_home = if local { home.join("routing-home") } else { home.clone() };
    let db = GlobalDatabase::for_home(&database_home)?;
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
            ProviderTransactions::new(&root, ExecutionHostId::new(host).unwrap()).unwrap();
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
    let routing = Arc::new(app_routing_services::RoutingListener::with_database(db.clone()));
    let app = Router::new().nest(
        "/v1",
        crate::server::router().merge(crate::server::agent_apps_high_availability::router()),
    );
    let paths = crate::server::lifecycle_api::RoutingPaths {
        home: database_home, config: home.as_path().join(".config/shiju"),
        state: home.as_path().join(".local/state/shiju"),
    };
    let lifecycle = |database, policy, tenant, host| Lifecycle(
        agent_gateway::aggregation_lifecycle::AggregationLifecycle { database, policy, tenant, host }
    );
    let app = if local {
        crate::server::lifecycle_api::configure(app, &routing, paths, true, lifecycle).await
    } else {
        crate::server::lifecycle_api::configure_with_policy(
            app, &routing, policy, db.clone(), paths, lifecycle,
        ).await.layer(axum::Extension(db.clone()))
    };
    let app = app
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
        project_id: "project".into(), target_host_id: host.into(), app_id: Claude,
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
            .query(&[("projectId", "project"), ("targetHostId", host), ("appId", "claude")])
    };
    if local {
        let response = request(Method::PUT, &format!("app-routing/hosts/{host}/config"))
            .json(&json!({"enabled": true, "listenAddress": "127.0.0.1",
                "listenPort": 0, "loggingEnabled": false})).send().await?;
        assert_eq!(response.status(), StatusCode::OK, "listener config: {}", response.text().await?);
    }
    assert_eq!(request(Method::POST, &format!("app-routing/hosts/{host}/start"))
        .json(&json!({})).send().await?.status(), StatusCode::OK);
    let applied = request(Method::POST, &format!("app-routing/hosts/{host}/projects/project/apps/claude/mode"))
        .json(&json!({"action":{"kind":"proxy","providerId":"current"}})).send().await?;
    assert_eq!(applied.status(), StatusCode::OK, "attach: {}", applied.text().await?);
    let candidates = ha(Method::GET, "candidates").send().await?.error_for_status()?
        .json::<Value>().await?;
    assert_eq!(candidates["candidates"].as_array().unwrap().len(), 2, "candidate response: {candidates}");
    let mut version = 0;
    for id in ["backup", "current"] {
        let response = ha(Method::POST, "queue/members")
            .json(&json!({"expectedVersion":version,"providerId":id})).send().await?;
        assert_eq!(response.status(), StatusCode::OK);
        version = response.json::<Value>().await?["version"].as_u64().unwrap();
    }
    let reordered = ha(Method::PUT, "queue/order")
        .json(&json!({"expectedVersion":version,"providerIds":["current","backup"]}))
        .send().await?.error_for_status()?.json::<Value>().await?;
    version = reordered["version"].as_u64().unwrap();
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
    assert_eq!(send().await?.status(), StatusCode::BAD_GATEWAY);
    assert_eq!(hits.load(Ordering::SeqCst), 2);
    for id in ["current", "backup"] {
        let health = ha(Method::GET, "health").query(&[("providerId", id)]).send().await?;
        assert_eq!(health.status(), StatusCode::OK);
        assert_eq!(health.json::<Value>().await?["state"], "open");
    }
    assert_eq!(send().await?.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(hits.load(Ordering::SeqCst), 2, "all-open circuits must reject before sending");
    assert_eq!(connections.load(Ordering::SeqCst), 1);
    // Change the real policy to expire cooldown; the next production request
    // must acquire the half-open probe. In local mode, require two successes
    // so both the half-open state and subsequent recovery are visible over HTTP.
    failures.store(false, Ordering::SeqCst);
    policy.circuit_breaker.cooldown_secs = 0;
    if local {
        policy.circuit_breaker.success_threshold = 2;
    }
    assert_eq!(ha(Method::PUT, "policy").json(&policy).send().await?.status(), StatusCode::OK);
    let response = send().await?;
    assert_eq!(response.status(), StatusCode::OK);
    let _: Value = response.json().await?;
    assert_eq!(hits.load(Ordering::SeqCst), 3);
    let health = ha(Method::GET, "health").query(&[("providerId", "backup")]).send().await?;
    assert_eq!(health.status(), StatusCode::OK);
    assert_eq!(health.json::<Value>().await?["state"], if local { "half_open" } else { "closed" });
    if local {
        let other_project = FailoverQueueQuery { project_id: "other-project".into(), ..sdk_scope.clone() };
        let other_queue = sdk.agent_apps_failover_queue(&other_project, Some(MANAGER)).await?;
        assert!(!other_queue.enabled);
        assert!(other_queue.entries.is_empty());
        policy.circuit_breaker.cooldown_secs = 300;
        assert_eq!(ha(Method::PUT, "policy").json(&policy).send().await?.status(), StatusCode::OK);
        let _: Value = send().await?.error_for_status()?.json().await?;
        let recovered = ha(Method::GET, "health").query(&[("providerId", "backup")])
            .send().await?.error_for_status()?.json::<Value>().await?;
        assert_eq!(recovered["state"], "closed");
        let before = connections.load(Ordering::SeqCst);
        let reset = ha(Method::POST, "health/reset").query(&[("providerId", "current")])
            .send().await?.error_for_status()?.json::<Value>().await?;
        assert_eq!(reset["state"], "unknown");
        assert_eq!(send().await?.status(), StatusCode::OK);
        assert_eq!(connections.load(Ordering::SeqCst), before + 1,
            "reset must make the higher-priority provider eligible for the next attempt");
        failures.store(true, Ordering::SeqCst);
        assert_eq!(send().await?.status(), StatusCode::BAD_GATEWAY);
        failures.store(false, Ordering::SeqCst);
        let reset = ha(Method::POST, "health/reset").query(&[("providerId", "backup")])
            .send().await?.error_for_status()?.json::<Value>().await?;
        assert_eq!(reset["state"], "unknown");
        assert_eq!(send().await?.status(), StatusCode::OK);
        assert_eq!(hits.load(Ordering::SeqCst), 7);
    }
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
    let removed = ha(Method::DELETE, "queue/members")
        .json(&json!({"expectedVersion":disabled["version"],"providerId":"backup"}))
        .send().await?.error_for_status()?.json::<Value>().await?;
    assert_eq!(removed["entries"].as_array().unwrap().len(), 1);
    assert_eq!(removed["entries"][0]["providerId"], "current");
    routing.stop().await?;
    eprintln!("controlled headless acceptance: OS={}, arch={}, version={}, backup_http={}, primary_connections={}",
        std::env::consts::OS, std::env::consts::ARCH, product_version::VERSION,
        hits.load(Ordering::SeqCst), connections.load(Ordering::SeqCst));
    for task in servers.0.drain(..) {
        task.abort();
        assert!(task.await.unwrap_err().is_cancelled());
    }
    Ok(())
}
