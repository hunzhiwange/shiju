//! macOS acceptance through the production HTTP router and SDK, without a desktop.
use crate::{
    agent_apps::mcp::McpCatalogStore,
    app_providers::AppProviderStore,
    gateway::{
        agent_apps_diagnostic_jobs::DiagnosticJobs, agent_apps_failover_runtime::FailoverRuntime,
    },
    global_database::GlobalDatabase,
};
use axum::{Extension, Router};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    os::unix::fs::PermissionsExt,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    net::TcpListener,
};
use vw_agent_apps::{
    high_availability::{
        circuit_breaker::AttemptOutcome,
        health::{HealthAccount, HealthKey},
    },
    mcp::catalog::McpCatalogDraft,
};
use vw_api_types::{
    agent_apps::AppId,
    agent_apps_diagnostics::*,
    agent_apps_high_availability::CircuitBreakerPolicy,
    app_providers::{AppProviderWrite, ProviderProtocol},
    id::{ProjectScope, TenantScope},
    mcp_management::{McpCatalogScope, McpConnectionDefinition, McpRecordId},
};
use vw_gateway_client::{AgentAppsSettingsError, GatewayClient, GatewayEndpoint};

const TOKEN: &str = "fixture-macos-diagnostic-member-00001";

struct Servers(Vec<tokio::task::JoinHandle<()>>);
impl Drop for Servers {
    fn drop(&mut self) {
        for task in &self.0 {
            task.abort();
        }
    }
}

async fn terminal(
    client: &GatewayClient,
    scope: &DiagnosticScopeQuery,
    job: &DiagnosticJobIdentity,
) -> DiagnosticJobSnapshot {
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let snapshot = client.diagnostic_status(scope, &job.task_id, TOKEN).await.unwrap();
            if !matches!(
                snapshot.state,
                DiagnosticJobState::Queued {}
                    | DiagnosticJobState::Running {}
                    | DiagnosticJobState::Cancelling {}
            ) {
                return snapshot;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("diagnostic never released its resources")
}

fn rejected(error: AgentAppsSettingsError, status: u16) {
    let code = if status == 404 {
        "not_found"
    } else if status == 401 {
        "unauthenticated"
    } else {
        "forbidden"
    };
    assert!(
        matches!(&error, AgentAppsSettingsError::Http { status: actual, body } if *actual == status && body["error"]["code"] == code),
        "{error:?}"
    );
}

#[tokio::test]
async fn diagnostic_macos_jobs_acceptance() -> anyhow::Result<()> {
    let root = tempfile::tempdir()?;
    std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700))?;
    let database = GlobalDatabase::for_home(root.path())?;
    let tenant = TenantScope::new("platform", "company")?;
    let project = ProjectScope::new(tenant.clone(), "project".into())?;
    let scope = DiagnosticScopeQuery {
        project_id: "project".into(),
        target_host_id: "host".into(),
        app_id: AppId::Claude,
    };
    let hang = Arc::new(AtomicBool::new(false));
    let requests = Arc::new(AtomicUsize::new(0));
    let calls = Arc::new(AtomicUsize::new(0));
    let (entered, mut arrivals) = tokio::sync::mpsc::unbounded_channel();
    let (closed, mut disconnects) = tokio::sync::mpsc::unbounded_channel();
    let upstream = TcpListener::bind("127.0.0.1:0").await?;
    let url = format!("http://{}/v1", upstream.local_addr()?);
    let mut servers = Servers(Vec::new());
    let (hanging, counted, tool_calls) = (hang.clone(), requests.clone(), calls.clone());
    servers.0.push(tokio::spawn(async move {
        let mut peers = tokio::task::JoinSet::new();
        loop {
            tokio::select! {
                accepted = upstream.accept() => {
                    let (socket, _) = accepted.unwrap();
                    let (entered, closed, hang, requests, calls) = (entered.clone(), closed.clone(), hanging.clone(), counted.clone(), tool_calls.clone());
                    peers.spawn(async move {
                        let mut socket = BufReader::new(socket);
                        let mut line = String::new();
                        socket.read_line(&mut line).await.unwrap();
                        let method = line.split_whitespace().next().unwrap().to_owned();
                        let mut length = 0;
                        loop {
                            line.clear();
                            socket.read_line(&mut line).await.unwrap();
                            if line == "\r\n" { break; }
                            if let Some(value) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                                length = value.trim().parse::<usize>().unwrap();
                            }
                        }
                        let mut bytes = vec![0; length];
                        socket.read_exact(&mut bytes).await.unwrap();
                        let body: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
                        if body["method"] == "tools/call" { calls.fetch_add(1, Ordering::SeqCst); }
                        requests.fetch_add(1, Ordering::SeqCst);
                        if hang.load(Ordering::SeqCst) {
                            entered.send(()).unwrap();
                            assert_eq!(socket.read(&mut [0]).await.unwrap(), 0, "cancel must close the actual TCP request");
                            closed.send(()).unwrap();
                            return;
                        }
                        let (status, content_type, payload) = if body["method"] == "initialize" {
                            (200, "application/json", json!({"jsonrpc":"2.0","id":body["id"],"result":{"protocolVersion":"2025-03-26","capabilities":{},"serverInfo":{"name":"controlled-macos","version":"1"}}}).to_string())
                        } else if body["method"].is_string() {
                            (202, "application/json", String::new())
                        } else if body["stream"] == true {
                            (200, "text/event-stream", crate::model_relay::stream_observer::tests::samples().into_iter().find(|(protocol, _, _)| *protocol == "anthropic").unwrap().2.to_owned())
                        } else if method == "POST" {
                            assert!(body.get("tools").is_none(), "model probes cannot carry tools");
                            (200, "application/json", json!({"type":"message","role":"assistant","model":"fixture-model","stop_reason":"end_turn","content":[{"type":"text","text":"OK"}]}).to_string())
                        } else { (200, "text/plain", String::new()) };
                        socket.write_all(format!("HTTP/1.1 {status} Fixture\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{payload}", payload.len()).as_bytes()).await.unwrap();
                    });
                }
                result = peers.join_next(), if !peers.is_empty() => { result.unwrap().unwrap(); }
            }
        }
    }));
    AppProviderStore::new(database.clone())
        .map_err(anyhow::Error::msg)?
        .save(
            &tenant,
            AppId::Claude,
            "provider",
            &AppProviderWrite {
                name: "Controlled macOS".into(),
                protocol: ProviderProtocol::Anthropic,
                api_key: "fixture-key".into(),
                base_url: url.clone(),
                model: "fixture-model".into(),
                request_options: None,
            },
        )
        .map_err(anyhow::Error::msg)?;
    let catalog = McpCatalogStore::new(database.clone()).unwrap();
    let mut permits = Vec::new();
    for mode in ["http", "normal", "cancel"] {
        let connection = if mode == "http" {
            McpConnectionDefinition::StreamableHttp {
                url: format!("{url}/mcp"),
                headers: Default::default(),
            }
        } else {
            McpConnectionDefinition::Stdio {
                command: "/bin/sh".into(),
                args: vec!["-c".into(), STDIO.into(), "fixture".into(), mode.into()],
                env: Default::default(),
                cwd: None,
            }
        };
        let digest = hex::encode(Sha256::digest(serde_json::to_vec(&connection)?));
        let stored = catalog
            .create(
                &McpCatalogScope::Project { scope: project.clone() },
                McpCatalogDraft {
                    id: mode.into(),
                    name: None,
                    description: None,
                    homepage: None,
                    documentation: None,
                    tags: vec![],
                    connection,
                },
            )
            .unwrap();
        let mut permit = json!({"project_id":"project","app_id":"claude","record_id":mode,"connection_revision":stored.connection_revision});
        if mode == "http" {
            permit["endpoint_sha256"] =
                json!(hex::encode(Sha256::digest(format!("{url}/mcp").as_bytes())));
        } else {
            permit["stdio_sha256"] = json!(digest);
            permit["stdio_workspace"] = json!(root.path());
        }
        permits.push(permit);
    }
    let policy_file = root.path().join("access.json");
    std::fs::write(&policy_file, json!({"principals":[{"id":"member","token_sha256":hex::encode(Sha256::digest(TOKEN)),"grants":[{"platform_id":"platform","company_id":"company","host_id":"host","provider_config_read":true,"routing_manage":true,"routing_apps":[{"project_id":"project","app_id":"claude"}],"mcp_probe":permits}]}],"hosts":[{"id":"host","binding":"local","online":true,"home":root.path(),"applications":root.path(),"search_paths":[],"credential_root":root.path()}]}).to_string())?;
    std::fs::set_permissions(&policy_file, std::fs::Permissions::from_mode(0o600))?;
    let jobs = DiagnosticJobs::open(root.path()).unwrap();
    let runtime = Arc::new(FailoverRuntime::default());
    let health_key = HealthKey {
        project,
        target_host_id: "host".into(),
        app_id: AppId::Claude,
        provider_id: "provider".into(),
        account: HealthAccount::Unbound,
    };
    let policy = CircuitBreakerPolicy {
        failure_threshold: 1,
        success_threshold: 1,
        cooldown_secs: 3600,
        error_rate_threshold: 0.5,
        min_requests: 1,
    };
    let observed_at = Instant::now();
    let circuit = runtime.health.circuit(&health_key, observed_at).unwrap();
    circuit
        .allow_owned_request(&policy, observed_at)
        .unwrap()
        .record(AttemptOutcome::ProviderFailure, observed_at);
    let before = runtime.health.snapshot(&health_key, &policy, observed_at);
    let router = Router::new()
        .nest("/v1", super::agent_apps_diagnostics::router_with_policy(Some(policy_file)))
        .layer(Extension(tenant.clone()))
        .layer(Extension(database))
        .layer(Extension(jobs.clone()))
        .layer(Extension(runtime.clone()));
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let client = GatewayClient::new(
        GatewayEndpoint::new("127.0.0.1", listener.local_addr()?.port()).with_tenant_scope(tenant),
    )
    .map_err(anyhow::Error::msg)?;
    servers.0.push(tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    }));
    let candidates = client
        .diagnostic_endpoint_candidates(
            &scope,
            &DiagnosticEndpointCandidatesQuery {
                provider_id: "provider".into(),
                draft_urls: vec![],
                preset_name: None,
            },
            TOKEN,
        )
        .await?;
    let endpoint_id = candidates
        .candidates
        .iter()
        .find(|candidate| candidate.display_url == url)
        .expect("saved endpoint must be discoverable through the SDK")
        .id
        .clone();
    let mut probes = vec![
        DiagnosticStartRequest::Reachability { provider_id: "provider".into() },
        DiagnosticStartRequest::Speed {
            provider_id: "provider".into(),
            endpoint_ids: vec![endpoint_id],
        },
        DiagnosticStartRequest::Model {
            provider_id: "provider".into(),
            model_id: "fixture-model".into(),
            protocol: None,
        },
        DiagnosticStartRequest::Streaming {
            provider_id: "provider".into(),
            model_id: "fixture-model".into(),
        },
        DiagnosticStartRequest::Mcp { server_ids: vec![McpRecordId("http".into())] },
        DiagnosticStartRequest::Mcp { server_ids: vec![McpRecordId("normal".into())] },
    ];
    let foreign = DiagnosticScopeQuery { project_id: "foreign".into(), ..scope.clone() };
    for probe in &probes {
        for denied in [
            foreign.clone(),
            DiagnosticScopeQuery { target_host_id: "foreign".into(), ..scope.clone() },
            DiagnosticScopeQuery { app_id: AppId::Codex, ..scope.clone() },
        ] {
            rejected(client.diagnostic_start(&denied, probe, TOKEN).await.unwrap_err(), 403);
        }
        rejected(client.diagnostic_start(&scope, probe, "invalid-token").await.unwrap_err(), 401);
    }
    assert_eq!(requests.load(Ordering::SeqCst), 0);
    assert!(!root.path().join("normal.pid").exists());
    let mut completed = Vec::new();
    for probe in &probes {
        let start_count = requests.load(Ordering::SeqCst);
        let job = client.diagnostic_start(&scope, probe, TOKEN).await?;
        let done = terminal(&client, &scope, &job).await;
        assert_eq!(done.state, DiagnosticJobState::Succeeded {}, "{probe:?}: {done:?}");
        assert_eq!(done.progress, DiagnosticProgress { completed: 1, total: Some(1) });
        let result = serde_json::to_value(done.results.as_ref().unwrap())?;
        assert_eq!(result[0]["report"]["outcome"]["status"], "succeeded", "{result}");
        assert_eq!(client.diagnostic_cancel(&scope, &job.task_id, TOKEN).await?, done);
        if !matches!(probe, DiagnosticStartRequest::Mcp { .. }) {
            assert_eq!(
                requests.load(Ordering::SeqCst) - start_count,
                if matches!(probe, DiagnosticStartRequest::Speed { .. }) { 2 } else { 1 }
            );
        }
        eprintln!("controlled {probe:?}: succeeded, completed=1/1");
        completed.push(job);
    }
    probes[5] = DiagnosticStartRequest::Mcp { server_ids: vec![McpRecordId("cancel".into())] };
    hang.store(true, Ordering::SeqCst);
    for (index, probe) in probes.iter().enumerate() {
        let job = client.diagnostic_start(&scope, probe, TOKEN).await?;
        let pids = if index == 5 {
            Some(
                tokio::time::timeout(Duration::from_secs(5), async {
                    loop {
                        let read = |suffix| {
                            std::fs::read_to_string(root.path().join(format!("cancel.{suffix}")))
                                .ok()
                                .and_then(|s| s.trim().parse::<i32>().ok())
                        };
                        if let (Some(pid), Some(child)) = (read("pid"), read("child"))
                            && root
                                .path()
                                .join("cancel.requests")
                                .metadata()
                                .is_ok_and(|m| m.len() > 0)
                        {
                            break (pid, child);
                        }
                        tokio::time::sleep(Duration::from_millis(10)).await;
                    }
                })
                .await?,
            )
        } else {
            tokio::time::timeout(Duration::from_secs(5), arrivals.recv()).await?.unwrap();
            None
        };
        let running = client.diagnostic_status(&scope, &job.task_id, TOKEN).await?;
        assert_eq!(running.state, DiagnosticJobState::Running {});
        assert_eq!(running.progress.completed, 0);
        rejected(client.diagnostic_cancel(&foreign, &job.task_id, TOKEN).await.unwrap_err(), 403);
        if let Some((pid, child)) = pids {
            // SAFETY: signal zero observes only the fixture's recorded process IDs.
            assert!(unsafe { libc::kill(pid, 0) == 0 && libc::kill(child, 0) == 0 });
            eprintln!("controlled stdio before cancel: leader={pid}, child={child}, alive");
        }
        let cancelling = client.diagnostic_cancel(&scope, &job.task_id, TOKEN).await?;
        assert!(matches!(
            cancelling.state,
            DiagnosticJobState::Cancelling {} | DiagnosticJobState::Cancelled {}
        ));
        let done = terminal(&client, &scope, &job).await;
        assert_eq!(done.state, DiagnosticJobState::Cancelled {});
        assert_eq!(done.progress.completed, 0);
        assert_eq!(done.results, Some(vec![]));
        assert_eq!(client.diagnostic_cancel(&scope, &job.task_id, TOKEN).await?, done);
        if let Some((pid, child)) = pids {
            tokio::time::timeout(Duration::from_secs(5), async {
                // SAFETY: signal zero observes only the fixture's recorded process IDs.
                while unsafe { libc::kill(pid, 0) == 0 || libc::kill(child, 0) == 0 } {
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
            })
            .await?;
            eprintln!("controlled stdio after cancel: leader={pid}, child={child}, exited");
        } else {
            tokio::time::timeout(Duration::from_secs(5), disconnects.recv()).await?.unwrap();
        }
        let checkpoint: Value =
            serde_json::from_slice(&std::fs::read(root.path().join("diagnostics.json"))?)?;
        assert!(
            checkpoint
                .as_array()
                .unwrap()
                .iter()
                .any(|entry| entry["identity"]["taskId"] == job.task_id.as_ref()
                    && entry["state"]["status"] == "cancelled")
        );
        eprintln!(
            "controlled {probe:?}: running -> {:?} -> cancelled, cleanup observed",
            cancelling.state
        );
    }
    // All three production host permits must be reusable after actual cleanup.
    let mut active = Vec::new();
    for _ in 0..3 {
        active.push(client.diagnostic_start(&scope, &probes[0], TOKEN).await?);
    }
    for _ in 0..3 {
        tokio::time::timeout(Duration::from_secs(5), arrivals.recv()).await?.unwrap();
    }
    let queued = client.diagnostic_start(&scope, &probes[0], TOKEN).await?;
    assert_eq!(
        client.diagnostic_status(&scope, &queued.task_id, TOKEN).await?.state,
        DiagnosticJobState::Queued {}
    );
    client.diagnostic_cancel(&scope, &queued.task_id, TOKEN).await?;
    assert_eq!(terminal(&client, &scope, &queued).await.state, DiagnosticJobState::Cancelled {});
    for job in active {
        client.diagnostic_cancel(&scope, &job.task_id, TOKEN).await?;
        assert_eq!(terminal(&client, &scope, &job).await.state, DiagnosticJobState::Cancelled {});
        tokio::time::timeout(Duration::from_secs(5), disconnects.recv()).await?.unwrap();
    }
    assert!(arrivals.try_recv().is_err(), "queued cancellation must not issue a request");
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    for mode in ["normal", "cancel"] {
        assert!(
            !std::fs::read_to_string(root.path().join(format!("{mode}.requests")))?
                .contains("tools/call")
        );
    }
    assert_eq!(
        runtime.health.snapshot(&health_key, &policy, observed_at),
        before,
        "diagnostics cannot reset or mutate production health"
    );
    assert!(!runtime.health.is_available(&health_key, &policy, observed_at));
    tokio::time::pause();
    tokio::time::advance(Duration::from_secs(601)).await;
    tokio::time::resume();
    rejected(
        client.diagnostic_status(&scope, &completed[0].task_id, TOKEN).await.unwrap_err(),
        404,
    );
    rejected(
        client.diagnostic_cancel(&scope, &completed[0].task_id, TOKEN).await.unwrap_err(),
        404,
    );
    eprintln!(
        "controlled acceptance: six successes, six active cancellations, queued cancellation, three permits reusable, tools/call=0, health unchanged, expired GET/POST=404"
    );
    for task in servers.0.drain(..) {
        task.abort();
        assert!(task.await.unwrap_err().is_cancelled());
    }
    Ok(())
}

const STDIO: &str = r#"
printf '%s\n' "$$" > "$1.pid"
/bin/sleep 120 &
printf '%s\n' "$!" > "$1.child"
while IFS= read -r line; do
    printf '%s\n' "$line" >> "$1.requests"
    case "$line" in
        *'"method":"initialize"'*)
            if [ "$1" = cancel ]; then wait; fi
            id=${line#*\"id\":}
            id=${id%%,*}
            id=${id%%\}*}
            printf '{"jsonrpc":"2.0","id":%s,"result":{"protocolVersion":"2025-03-26","capabilities":{},"serverInfo":{"name":"controlled-stdio","version":"1"}}}\n' "$id"
            ;;
    esac
done
wait
"#;

/// Also exercise bootstrap-owned extensions in the real standalone gateway process.
#[tokio::test]
async fn diagnostic_macos_jobs_acceptance_gateway() -> anyhow::Result<()> {
    use super::app_routing::lifecycle_api_tests::{
        HOST, MANAGER, client, spawn_gateway, write_policy,
    };
    let root = tempfile::tempdir()?;
    let credentials = root.path().join("credentials");
    std::fs::create_dir(&credentials)?;
    std::fs::set_permissions(&credentials, std::fs::Permissions::from_mode(0o700))?;
    let policy = root.path().join("access.json");
    write_policy(&policy, root.path(), 0);
    let mut access: Value = serde_json::from_slice(&std::fs::read(&policy)?)?;
    access["principals"][0]["grants"][0]["provider_config_read"] = json!(true);
    access["principals"][0]["grants"][0]["routing_apps"] =
        json!([{"project_id":"project","app_id":"claude"}]);
    std::fs::write(&policy, access.to_string())?;
    let upstream = TcpListener::bind("127.0.0.1:0").await?;
    AppProviderStore::new(GlobalDatabase::for_home(root.path())?)
        .map_err(anyhow::Error::msg)?
        .save(
            &TenantScope::new("p", "owner")?,
            AppId::Claude,
            "provider",
            &AppProviderWrite {
                name: "Controlled standalone".into(),
                protocol: ProviderProtocol::Anthropic,
                api_key: "fixture-key".into(),
                base_url: format!("http://{}/v1", upstream.local_addr()?),
                model: "fixture-model".into(),
                request_options: None,
            },
        )
        .map_err(anyhow::Error::msg)?;
    let (gateway, port) = spawn_gateway(root.path(), &policy).await;
    let sdk = client(port, "owner");
    let scope = DiagnosticScopeQuery {
        project_id: "project".into(),
        target_host_id: HOST.into(),
        app_id: AppId::Claude,
    };
    let job = sdk
        .diagnostic_start(
            &scope,
            &DiagnosticStartRequest::Reachability { provider_id: "provider".into() },
            MANAGER,
        )
        .await?;
    let (mut socket, _) = tokio::time::timeout(Duration::from_secs(5), upstream.accept()).await??;
    let mut headers = Vec::new();
    tokio::time::timeout(Duration::from_secs(5), async {
        while !headers.ends_with(b"\r\n\r\n") {
            headers.push(socket.read_u8().await.unwrap());
        }
    })
    .await?;
    assert!(headers.starts_with(b"GET /v1 HTTP/1.1"));
    assert_eq!(
        sdk.diagnostic_status(&scope, &job.task_id, MANAGER).await?.state,
        DiagnosticJobState::Running {}
    );
    sdk.diagnostic_cancel(&scope, &job.task_id, MANAGER).await?;
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let done = sdk.diagnostic_status(&scope, &job.task_id, MANAGER).await.unwrap();
            if done.state == (DiagnosticJobState::Cancelled {}) {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await?;
    assert_eq!(tokio::time::timeout(Duration::from_secs(5), socket.read(&mut [0])).await??, 0);
    drop(gateway); // GatewayProcess::drop kills and waits for the standalone child.
    eprintln!(
        "standalone gateway: SDK POST=202, GET running=200, cancel=200, cancelled=200, actual TCP EOF, gateway reaped"
    );
    Ok(())
}
