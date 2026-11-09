//! Windowless macOS acceptance through the SDK, HTTP router, and scoped storage.

use std::time::Instant;

use base64::{Engine as _, engine::general_purpose::STANDARD};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    sync::{Semaphore, mpsc},
    task::JoinHandle,
};
use vw_api_types::{
    agent_apps::AppId,
    agent_apps_auth::AuthErrorCode,
    app_providers::{AppProviderWrite, ProviderProtocol},
    apps_usage::{
        UsageQueryData, UsageQueryErrorCode, UsageQueryKind, UsageQueryOutcome, UsageSnapshotQuery,
    },
    id::{ProjectScope, TaskId, TenantScope},
    task::TaskStatus,
};
use vw_gateway_client::{AppsUsageOperation, AppsUsageWriteError, GatewayClient, GatewayEndpoint};

use super::{
    FIXTURE_CA, FIXTURE_CERT, FIXTURE_KEY, Fixture, NativeTransport, OTHER_TOKEN, TOKEN,
    codex_http_fixture, managed_codex_fixture, router_with_transports,
};
use crate::{agent_apps::usage_settings::UsageSettingsScope, app_providers::AppProviderStore};
use serde_json::json;
use std::sync::Arc;

const HOST: &str = "local";
const WALLET_ORIGIN: &str = "https://api.deepseek.com";
const PLAN_ORIGIN: &str = "https://api.kimi.com";

fn tenant() -> TenantScope {
    TenantScope::new("test-platform", "test-company").unwrap()
}

fn selection(provider: &str, kind: UsageQueryKind) -> UsageSnapshotQuery {
    UsageSnapshotQuery {
        project_id: "project-a".into(),
        target_host_id: HOST.into(),
        app_id: AppId::Codex,
        provider_id: provider.into(),
        account_id: None,
        kind: Some(kind),
    }
}

async fn sdk(router: axum::Router) -> (GatewayClient, JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = GatewayEndpoint::new("127.0.0.1", listener.local_addr().unwrap().port())
        .with_tenant_scope(tenant());
    let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    (GatewayClient::new(endpoint).unwrap(), server)
}

fn seed_provider(fixture: &mut Fixture, provider: &str, base_url: &str) {
    AppProviderStore::new(fixture.database.clone())
        .unwrap()
        .save(
            &tenant(),
            AppId::Codex,
            provider,
            &AppProviderWrite {
                name: provider.into(),
                protocol: ProviderProtocol::OpenaiResponses,
                api_key: "fixture-native-key".into(),
                base_url: base_url.into(),
                model: "fixture-model".into(),
                request_options: None,
            },
        )
        .unwrap();
    for principal in fixture.policy["principals"].as_array_mut().unwrap() {
        let grant = &mut principal["grants"][0];
        let resource = json!({
            "project_id": "project-a", "app_id": "codex", "provider_id": provider,
        });
        for action in ["usage_read", "usage_cancel"] {
            grant[action].as_array_mut().unwrap().push(resource.clone());
        }
    }
    fixture.write_policy();
}

fn grant_refresh(
    fixture: &mut Fixture,
    provider: &str,
    origin: &str,
    kind: Option<UsageQueryKind>,
) {
    fixture.policy["principals"][0]["grants"][0]["usage_refresh"].as_array_mut().unwrap().push(
        json!({
            "project_id": "project-a", "app_id": "codex", "provider_id": provider,
            "origin": origin, "kind": kind,
        }),
    );
    fixture.write_policy();
}

async fn terminal(client: &GatewayClient, id: &TaskId) -> AppsUsageOperation {
    tokio::time::timeout(std::time::Duration::from_secs(8), async {
        loop {
            let event = client.apps_usage_operation_events(HOST, id, TOKEN).await.unwrap();
            if event.operation.phase == vw_api_types::agent_apps::AppOperationPhase::Terminal {
                return event;
            }
        }
    })
    .await
    .expect("usage refresh did not reach a terminal state")
}

async fn refresh(client: &GatewayClient, query: &UsageSnapshotQuery) -> AppsUsageOperation {
    let operation = client.apps_usage_refresh(query, TOKEN).await.unwrap();
    let event = terminal(client, &operation.operation_id).await;
    assert_eq!(event.items.len(), 1);
    assert_eq!(Some(event.items[0].kind), query.kind);
    event
}

#[tokio::test]
async fn usage_macos_api_returns_expected_result() {
    let mut fixture = Fixture::new().await;
    let _save = managed_codex_fixture(&mut fixture);
    let (codex_seen, mut codex_requests) = mpsc::unbounded_channel();
    let codex = codex_http_fixture(codex_seen, Arc::new(Semaphore::new(1))).await;
    let native = NativeHttp::start().await;
    let (router, service) =
        router_with_transports(&mut fixture, Some(codex), Some(native.transport.clone()));

    seed_provider(&mut fixture, "wallet", "https://api.deepseek.com/v1");
    seed_provider(&mut fixture, "plan", "https://api.kimi.com/coding/v1");
    grant_refresh(&mut fixture, "wallet", WALLET_ORIGIN, Some(UsageQueryKind::Balance));
    grant_refresh(&mut fixture, "plan", PLAN_ORIGIN, Some(UsageQueryKind::CodingPlan));
    let (client, server) = sdk(router).await;
    let wallet = selection("wallet", UsageQueryKind::Balance);
    let plan = selection("plan", UsageQueryKind::CodingPlan);
    let script = selection("ok", UsageQueryKind::CustomScript);
    let mut codex = selection("official", UsageQueryKind::OfficialSubscription);
    codex.account_id = Some("managed".into());
    for query in [&wallet, &plan, &script, &codex] {
        assert_eq!(refresh(&client, query).await.operation.status, TaskStatus::Completed);
    }
    assert_eq!(codex_requests.try_recv().unwrap(), "Bearer fixture-oauth-original");

    let read = |query| client.apps_usage_snapshot(query, TOKEN);
    let wallet_result = read(&wallet).await.unwrap().snapshot.unwrap();
    assert!(matches!(&wallet_result.result.outcome,
        UsageQueryOutcome::Success { data: UsageQueryData::Balance(wallets) }
        if wallets[0].currency == "CNY" && wallets[0].remaining.map(f64::from) == Some(-1.25)));
    assert!(matches!(read(&plan).await.unwrap().snapshot.unwrap().result.outcome,
        UsageQueryOutcome::Success { data: UsageQueryData::CodingPlan(windows) }
        if windows[0].name == "weekly_limit" && windows[0].remaining.map(f64::from) == Some(7.0)));
    assert!(matches!(read(&script).await.unwrap().snapshot.unwrap().result.outcome,
        UsageQueryOutcome::Success { data: UsageQueryData::CustomScript(packages) }
        if packages[0].remaining.map(f64::from) == Some(17.0)));
    assert!(matches!(read(&codex).await.unwrap().snapshot.unwrap().result.outcome,
        UsageQueryOutcome::Success { data: UsageQueryData::OfficialSubscription(quota) }
        if quota.credits_balance.map(f64::from) == Some(42.5)));

    native.wallet_status.store(429, std::sync::atomic::Ordering::SeqCst);
    assert_eq!(refresh(&client, &wallet).await.operation.status, TaskStatus::Failed);
    let stale = read(&wallet).await.unwrap().snapshot.unwrap();
    assert!(stale.stale);
    assert_eq!(stale.result, wallet_result.result);
    assert!(matches!(stale.refresh_failure.unwrap().outcome,
        UsageQueryOutcome::Failure { error, .. } if error.code == UsageQueryErrorCode::UpstreamRateLimited));
    let project = ProjectScope::new(tenant(), wallet.project_id.clone()).unwrap();
    let expired = service
        .provider_snapshot(
            UsageSettingsScope {
                project: &project,
                principal: "owner",
                target_host: HOST,
                app: wallet.app_id,
                provider: &wallet.provider_id,
                account: None,
            },
            wallet.kind,
            Instant::now() + std::time::Duration::from_secs(600),
        )
        .unwrap()
        .snapshot
        .unwrap();
    assert!(!expired.stale);
    assert!(matches!(expired.result.outcome, UsageQueryOutcome::Failure { .. }));
    server.abort();
}

#[tokio::test]
async fn usage_macos_api_rejects_invalid_input() {
    let mut fixture = Fixture::new().await;
    let mut native = NativeHttp::start().await;
    let (router, _) = router_with_transports(&mut fixture, None, Some(native.transport.clone()));
    seed_provider(&mut fixture, "wallet", "https://api.deepseek.com/v1");
    grant_refresh(&mut fixture, "wallet", WALLET_ORIGIN, None);
    let (client, server) = sdk(router).await;
    let wallet = selection("wallet", UsageQueryKind::Balance);

    let denied = client.apps_usage_refresh(&wallet, TOKEN).await;
    assert!(
        matches!(denied, Err(AppsUsageWriteError::Http {
        status: 403, error: Some(ref error),
    }) if error.code == AuthErrorCode::Forbidden),
        "{denied:?}"
    );
    assert!(native.requests.try_recv().is_err());
    grant_refresh(&mut fixture, "wallet", WALLET_ORIGIN, wallet.kind);
    let hold_response = native.release.acquire().await.unwrap();
    let operation = client.apps_usage_refresh(&wallet, TOKEN).await.unwrap();
    assert_eq!(
        tokio::time::timeout(std::time::Duration::from_secs(5), native.requests.recv())
            .await
            .unwrap()
            .as_deref(),
        Some("/user/balance"),
    );
    let forbidden =
        client.apps_usage_operation_cancel(HOST, &operation.operation_id, OTHER_TOKEN).await;
    assert!(matches!(forbidden, Err(AppsUsageWriteError::Http { status: 404, .. })));
    let cancelled =
        client.apps_usage_operation_cancel(HOST, &operation.operation_id, TOKEN).await.unwrap();
    assert_eq!(cancelled.operation.status, TaskStatus::Cancelled);
    drop(hold_response);
    assert!(client.apps_usage_snapshot(&wallet, TOKEN).await.unwrap().snapshot.is_none());

    assert_eq!(refresh(&client, &wallet).await.operation.status, TaskStatus::Completed);
    assert!(client.apps_usage_snapshot(&wallet, TOKEN).await.unwrap().snapshot.is_some());
    assert!(client.apps_usage_snapshot(&wallet, OTHER_TOKEN).await.unwrap().snapshot.is_none());
    server.abort();
}

struct NativeHttp {
    transport: NativeTransport,
    release: Arc<Semaphore>,
    requests: mpsc::UnboundedReceiver<String>,
    wallet_status: Arc<std::sync::atomic::AtomicU16>,
    server: JoinHandle<()>,
}

impl Drop for NativeHttp {
    fn drop(&mut self) {
        self.server.abort();
    }
}

impl NativeHttp {
    async fn start() -> Self {
        let ca = reqwest::Certificate::from_der(&STANDARD.decode(FIXTURE_CA).unwrap()).unwrap();
        let leaf = STANDARD.decode(FIXTURE_CERT).unwrap();
        let key = STANDARD.decode(FIXTURE_KEY).unwrap();
        let config = rustls::ServerConfig::builder_with_provider(Arc::new(
            rustls::crypto::ring::default_provider(),
        ))
        .with_safe_default_protocol_versions()
        .unwrap()
        .with_no_client_auth()
        .with_single_cert(
            vec![leaf.into()],
            rustls_pki_types::PrivateKeyDer::try_from(key).unwrap(),
        )
        .unwrap();
        let acceptor = tokio_rustls::TlsAcceptor::from(Arc::new(config));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let transport = Arc::new(move || {
            // Restrict this client to the fixture CA. The shared leaf names chatgpt.com.
            reqwest::Client::builder()
                .resolve("api.deepseek.com", addr)
                .resolve("api.kimi.com", addr)
                .tls_certs_only([ca.clone()])
                .tls_danger_accept_invalid_hostnames(true)
        });
        let release = Arc::new(Semaphore::new(1));
        let wallet_status = Arc::new(std::sync::atomic::AtomicU16::new(200));
        let (seen, requests) = mpsc::unbounded_channel();
        let response_release = release.clone();
        let status = wallet_status.clone();
        let server = tokio::spawn(async move {
            let mut connections = tokio::task::JoinSet::new();
            loop {
                tokio::select! {
                    Some(result) = connections.join_next() => result.unwrap(),
                    connection = listener.accept() => {
                        let (socket, _) = connection.unwrap();
                        let acceptor = acceptor.clone();
                        let seen = seen.clone();
                        let release = response_release.clone();
                        let status = status.clone();
                        connections.spawn(async move {
                            let stream = acceptor.accept(socket).await.unwrap();
                            let mut stream = BufReader::new(stream);
                            let mut line = String::new();
                            stream.read_line(&mut line).await.unwrap();
                            let (host, path, status, body) = match line.as_str() {
                                "GET /user/balance HTTP/1.1\r\n" => (
                                    "api.deepseek.com", "/user/balance",
                                    status.load(std::sync::atomic::Ordering::SeqCst),
                                    r#"{"is_available":false,"balance_infos":[{"currency":"CNY","total_balance":"-1.25"}]}"#,
                                ),
                                "GET /coding/v1/usages HTTP/1.1\r\n" => (
                                    "api.kimi.com", "/coding/v1/usages", 200,
                                    r#"{"usage":{"limit":10,"remaining":7}}"#,
                                ),
                                _ => panic!("unexpected native request path or method"),
                            };
                            let mut headers = std::collections::HashMap::new();
                            loop {
                                line.clear();
                                if stream.read_line(&mut line).await.unwrap() == 0 {
                                    return;
                                }
                                if line == "\r\n" {
                                    break;
                                }
                                let (name, value) = line.split_once(':').unwrap();
                                headers.insert(name.to_ascii_lowercase(), value.trim().to_owned());
                            }
                            assert_eq!(headers["authorization"], "Bearer fixture-native-key");
                            assert_eq!(headers["accept"], "application/json");
                            assert_eq!(headers["host"], host);
                            seen.send(path.into()).unwrap();
                            let _permit = release.acquire().await.unwrap();
                            let _ = stream.write_all(format!(
                                "HTTP/1.1 {status} Fixture\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                                body.len(),
                            ).as_bytes()).await;
                            let _ = stream.shutdown().await;
                        });
                    }
                }
            }
        });
        Self { transport, release, requests, wallet_status, server }
    }
}
