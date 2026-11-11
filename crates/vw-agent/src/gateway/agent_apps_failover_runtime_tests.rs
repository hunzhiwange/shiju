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
use axum::{Json, Router, http::StatusCode, routing::post};
use serde_json::{Value, json};
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
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
async fn ha_runtime_uses_single_retry_owner() -> anyhow::Result<()> {
    for (retries, all_unavailable) in [(0, false), (2, false), (0, true), (2, true)] {
        let home = tempfile::tempdir()?;
        let root = home.path().join("credentials");
        let native = home.path().join(".claude");
        fs::create_dir(&root)?;
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700))?;
        fs::create_dir(&native)?;
        let database = GlobalDatabase::for_home(home.path())?;
        let scope = FailoverQueueScope {
            project: ProjectScope::new(TenantScope::new("p", "owner")?, "project".into())?,
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
        let hits = Arc::new(AtomicUsize::new(0));
        let record = hits.clone();
        let upstream = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let endpoint = format!("http://{}", upstream.local_addr()?);
        let server = tokio::spawn(async move {
            axum::serve(
                upstream,
                Router::new().route(
                    "/v1/messages",
                    post(move |headers: axum::http::HeaderMap, Json(body): Json<Value>| {
                        let hits = record.clone();
                        async move {
                            assert_eq!(headers["x-api-key"], "beta-key");
                            assert_eq!(body["model"], "fixture");
                            hits.fetch_add(1, Ordering::SeqCst);
                            (StatusCode::BAD_GATEWAY, Json(json!({"error":"controlled failure"})))
                        }
                    }),
                ),
            )
            .await
            .unwrap();
        });
        // These real sockets fail during TLS, before an HTTP request can be sent.
        let (unavailable, connections, failed_server) = failing_tls().await?;
        let (failed_backup, backup_connections, failed_backup_server) = failing_tls().await?;
        let endpoint = if all_unavailable { failed_backup } else { endpoint };
        let store = AppProviderStore::new(database.clone()).map_err(anyhow::Error::msg)?;
        for (id, base_url) in [("alpha", unavailable), ("beta", endpoint)] {
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
        let saved =
            set_enabled(database, &routing, home.path(), &root, &scope, saved.version, true)?;
        let config: Value = serde_json::from_slice(&fs::read(native.join("settings.json"))?)?;
        let token = config["env"]["ANTHROPIC_AUTH_TOKEN"].as_str().unwrap();
        let client = reqwest::Client::builder()
            .no_proxy()
            .timeout(std::time::Duration::from_secs(10))
            .build()?;
        let identity = crate::model_relay::RelayIdentity {
            tenant: tenant.clone(),
            project: "project".into(),
            session: "session".into(),
            provider: "alpha".into(),
            model: "fixture".into(),
            generation: 1,
        };
        let relay = crate::model_relay::RelayBinding::new(
            identity.clone(),
            std::time::Duration::from_secs(60),
        )?;
        assert!(!relay.accepts(token, &identity));
        assert_eq!(
            client
                .post(format!("http://{address}/v1/messages"))
                .header("x-api-key", relay.bearer_token())
                .json(&json!({"model":"fixture"}))
                .send()
                .await?
                .status(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(connections.load(Ordering::SeqCst), 0);
        let response = client
            .post(format!("http://{address}/v1/messages"))
            .header("x-api-key", token)
            .header("x-retry-owner", "session")
            .header("x-provider-retries", "100")
            .json(&json!({"model":"fixture","messages":[{"role":"user","content":"hello"}]}))
            .send()
            .await?;
        assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
        let observed = hits.load(Ordering::SeqCst);
        assert!(observed <= retries + 1, "total sends multiplied: {observed}");
        assert_eq!(queues.get(&scope)?, saved);
        let network_attempts = observed
            + connections.load(Ordering::SeqCst)
            + backup_connections.load(Ordering::SeqCst);
        assert!(network_attempts <= retries + 1, "network attempts multiplied: {network_attempts}");
        assert_eq!(
            network_attempts,
            if all_unavailable { retries + 1 } else { 1 + usize::from(retries > 0) }
        );
        eprintln!(
            "max_retries={retries} provider_retries={retries} all_unavailable={all_unavailable} network_attempts={network_attempts} provider_http_requests={observed}"
        );
        routing.stop().await?;
        for task in [failed_server, failed_backup_server] {
            task.abort();
            assert!(task.await.unwrap_err().is_cancelled());
        }
        server.abort();
        assert!(server.await.unwrap_err().is_cancelled());
        assert_eq!(
            observed,
            usize::from(retries > 0 && !all_unavailable),
            "trusted connect failure must select the second provider when budget permits"
        );
    }
    Ok(())
}

async fn failing_tls() -> anyhow::Result<(String, Arc<AtomicUsize>, tokio::task::JoinHandle<()>)> {
    use tokio::io::AsyncWriteExt;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let url = format!("https://{}", listener.local_addr()?);
    let connections = Arc::new(AtomicUsize::new(0));
    let record = connections.clone();
    let server = tokio::spawn(async move {
        loop {
            let (mut socket, _) = listener.accept().await.unwrap();
            record.fetch_add(1, Ordering::SeqCst);
            let _ = socket.write_all(b"HTTP/1.1 503 Service Unavailable\r\n\r\n").await;
            let _ = socket.shutdown().await;
        }
    });
    Ok((url, connections, server))
}
