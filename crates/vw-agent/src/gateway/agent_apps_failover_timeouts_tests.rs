use super::{
    agent_apps_high_availability_service::set_enabled,
    agent_apps_high_availability_store::FailoverQueueStore,
};
use crate::{
    app_providers::AppProviderStore,
    app_routing::{RoutingListener, provider_publication},
    config::SystemConfigStore,
    global_database::GlobalDatabase,
};
use axum::http::StatusCode;
use serde_json::{Value, json};
use std::{fs, os::unix::fs::PermissionsExt, time::Duration};
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    time::{sleep, timeout},
};
use vw_agent_apps::providers::{
    failover::{ProviderAuth, ProviderSnapshot},
    live::{commit::*, write_guard::lock_app},
};
use vw_api_types::{
    agent_apps::AppId,
    agent_apps_high_availability::{FailoverQueueScope, FailoverQueueVersion},
    app_providers::{
        AppProviderSummary,
        claude::{ClaudeConfig, ClaudeProfile},
        mode_apply::ModeApplyAction,
    },
    id::{ProjectScope, TenantScope},
};

#[tokio::test]
async fn ha_first_byte_timeout_stops_silent_upstream() -> anyhow::Result<()> {
    for mode in ["headers", "keepalive", "valid"] {
        let retries = 2;
        let home = tempfile::tempdir()?;
        let root = home.path().join("credentials");
        let native = home.path().join(".claude");
        fs::create_dir(&root)?;
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700))?;
        fs::create_dir(&native)?;
        let database = GlobalDatabase::for_home(home.path())?;
        let scope = FailoverQueueScope {
            project: ProjectScope::new(
                TenantScope::new("p", "owner")?,
                format!("ha-first-{}", uuid::Uuid::new_v4()).into(),
            )?,
            target_host_id: "host".into(),
            app_id: AppId::Claude,
        };
        let tenant = scope.project.tenant_scope();
        SystemConfigStore::new(database.clone())?.upsert(
            tenant,
            &json!({"reliability":{"provider_retries":retries,"provider_backoff_ms":0}})
                .to_string(),
        )?;
        let mut policy = super::agent_apps_failover_runtime::policy(database.clone(), &scope)?;
        policy.retry_timeout.max_retries = retries as u8;
        policy.circuit_breaker.failure_threshold = 1;
        super::settings_store::GatewaySettingsStore::new(database.clone())?.upsert(
            tenant,
            super::agent_apps_high_availability_store::NAMESPACE,
            &super::agent_apps_failover_runtime::policy_key(&scope)?,
            &serde_json::to_string(&policy)?,
        )?;
        let tx = ProviderTransactions::new(&root, ExecutionHostId::new("host").unwrap()).unwrap();
        let mut guard = lock_app(AppId::Claude, &native).unwrap();
        tx.commit(
            tenant,
            &mut guard,
            &[],
            PendingTarget { pointer: Some("alpha".into()), ..Default::default() },
        )
        .unwrap();
        drop(guard);
        let upstream = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let endpoint = format!("http://{}", upstream.local_addr()?);
        let backup = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let backup_endpoint = format!("http://{}", backup.local_addr()?);
        let store = AppProviderStore::new(database.clone()).map_err(anyhow::Error::msg)?;
        for (id, base_url) in [("alpha", endpoint), ("beta", backup_endpoint)] {
            store
                .claude_save(
                    tenant,
                    id,
                    &ClaudeProfile {
                        name: id.into(),
                        config: ClaudeConfig::ThirdParty {
                            base_url,
                            api_key: format!("{id}-key"),
                            api_key_header: true,
                        },
                        model: Some("fixture".into()),
                    },
                    None,
                    true,
                )
                .map_err(anyhow::Error::msg)?;
        }
        let routing = RoutingListener::default();
        let address = routing.start(0)?;
        provider_publication::apply(
            database.clone(),
            &routing,
            home.path(),
            &root,
            &scope,
            &ModeApplyAction::Proxy { provider_id: "alpha".into() },
        )
        .unwrap();
        let queues = FailoverQueueStore::new(database.clone())?;
        let providers = ["alpha", "beta"].map(|id| ProviderSnapshot {
            provider: AppProviderSummary {
                id: id.into(),
                app_id: AppId::Claude,
                name: id.into(),
                configured: true,
            },
            auth: ProviderAuth::ThirdParty,
        });
        let saved = queues.update(&scope, FailoverQueueVersion(0), |queue, _| {
            for id in ["alpha", "beta"] {
                queue.add(id, &providers).map_err(|e| anyhow::anyhow!("{e:?}"))?;
            }
            Ok(())
        })?;
        let mut saved = set_enabled(
            database.clone(),
            &routing,
            home.path(),
            &root,
            &scope,
            saved.version,
            true,
        )?;
        crate::app_routing::config_apply::RoutingConfigStore::new(
            GlobalDatabase::for_home(&crate::global::paths().home)?,
            tenant.clone(),
            "host".into(),
        )?
        .save_app_timeouts(
            &scope.project,
            AppId::Claude,
            crate::app_routing::config_apply::AppTimeouts {
                first_byte_timeout_secs: 1,
                ..Default::default()
            },
        )?;
        let config: Value = serde_json::from_slice(&fs::read(native.join("settings.json"))?)?;
        let token = config["env"]["ANTHROPIC_AUTH_TOKEN"].as_str().unwrap();
        let client = reqwest::Client::builder()
            .no_proxy()
            .timeout(std::time::Duration::from_secs(10))
            .build()?;
        let request = || {
            client.post(format!("http://{address}/v1/messages"))
            .header("x-api-key", token)
            .json(&json!({"model":"fixture","stream":true,"messages":[{"role":"user","content":"hello"}]}))
            .send()
        };
        let first = tokio::spawn(request());
        let (socket, _) = timeout(Duration::from_secs(4), upstream.accept()).await??;
        let mut socket = BufReader::new(socket);
        let mut head = String::new();
        while !head.ends_with("\r\n\r\n") {
            assert_ne!(socket.read_line(&mut head).await?, 0);
        }
        assert!(head.to_ascii_lowercase().contains("x-api-key: alpha-key"));
        let len: usize = head
            .lines()
            .find_map(|line| {
                line.to_ascii_lowercase().strip_prefix("content-length: ").map(str::to_owned)
            })
            .unwrap()
            .parse()?;
        let mut payload = vec![0; len];
        socket.read_exact(&mut payload).await?;
        assert_eq!(serde_json::from_slice::<Value>(&payload)?["stream"], true);
        socket.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nTransfer-Encoding: chunked\r\n\r\n").await?;
        if mode == "valid" {
            let sample = crate::model_relay::stream_observer::tests::samples()
                .into_iter()
                .find(|(kind, _, _)| *kind == "anthropic")
                .unwrap()
                .2;
            chunk(&mut socket, sample.as_bytes()).await?;
            let response = timeout(Duration::from_secs(4), first).await???;
            assert_eq!(response.status(), StatusCode::OK);
            assert!(response.text().await?.contains("message_stop"));
        } else {
            tokio::pin!(first);
            let response = timeout(Duration::from_secs(4), async {
                loop {
                    tokio::select! {
                        result = &mut first => break result.unwrap().unwrap(),
                        () = sleep(Duration::from_millis(50)), if mode == "keepalive" => {
                            if chunk(&mut socket, b": keepalive\n\nevent: ping\ndata: {\"type\":\"ping\"}\n\n").await.is_err() {
                                break first.await.unwrap().unwrap();
                            }
                        }
                    }
                }
            }).await?;
            assert_eq!(response.status(), StatusCode::GATEWAY_TIMEOUT, "{mode}");
            let mut byte = [0];
            let closed = timeout(Duration::from_secs(4), socket.read(&mut byte)).await?;
            assert!(
                matches!(closed, Ok(0))
                    || matches!(&closed, Err(error) if error.kind() == std::io::ErrorKind::ConnectionReset),
                "silent upstream retained"
            );
            assert!(
                timeout(Duration::from_millis(100), backup.accept()).await.is_err(),
                "a sent request with unknown execution must never replay to backup"
            );
            assert_eq!(queues.get(&scope)?, saved);
            // Remove the eligible backup explicitly in the fixture, so a new
            // request can reveal whether the timed-out primary was opened.
            saved = queues.update(&scope, saved.version, |queue, _| {
                queue.remove("beta").map_err(|error| anyhow::anyhow!("{error:?}"))?;
                Ok(())
            })?;
            let second = timeout(Duration::from_secs(4), request()).await??;
            assert_eq!(second.status(), StatusCode::SERVICE_UNAVAILABLE);
            // The timed-out primary must remain unavailable during cooldown.
            // With a missing timeout outcome, the primary is incorrectly reused.
            assert!(
                timeout(Duration::from_millis(100), upstream.accept()).await.is_err(),
                "first-byte timeout must count as provider failure"
            );
        }
        assert!(
            timeout(Duration::from_millis(100), backup.accept()).await.is_err(),
            "a sent request with unknown execution must never replay to backup"
        );
        assert_eq!(queues.get(&scope)?, saved);
        routing.stop().await?;
        eprintln!(
            "mode={mode}: one real HTTP send; no replay; queue changes only by explicit fixture mutation; listener stopped"
        );
    }
    Ok(())
}

async fn chunk(socket: &mut BufReader<tokio::net::TcpStream>, bytes: &[u8]) -> std::io::Result<()> {
    socket.write_all(format!("{:x}\r\n", bytes.len()).as_bytes()).await?;
    socket.write_all(bytes).await?;
    socket.write_all(b"\r\n").await
}
