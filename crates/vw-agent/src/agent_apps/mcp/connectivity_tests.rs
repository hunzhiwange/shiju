use std::{fs, os::unix::fs::PermissionsExt};

use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use serde_json::json;

use vw_api_types::{
    TenantScope,
    mcp_management::{
        McpCatalogScope, McpConfigValue, McpConnectionDefinition, McpConnectionOutcome,
        McpCredentialRef, McpFailureCode,
    },
};

use super::{
    McpCatalogStore,
    catalog_create_tests::{http_draft, with_database},
};
use crate::{
    agent_apps::auth_accounts::{AuthAccountMetadata, AuthAccountScope, AuthAccountStore},
    global_database::run_sync_database,
};

#[test]
fn cc_mcp_t30_stdio_discovers_tools_using_args_env_and_cwd() -> anyhow::Result<()> {
    with_database(|database| {
        let workspace = tempfile::tempdir()?;
        fs::set_permissions(workspace.path(), fs::Permissions::from_mode(0o700))?;
        let workdir = workspace.path().join("fixture-workdir");
        fs::create_dir(&workdir)?;
        let catalog = McpCatalogStore::new(database.clone()).unwrap();
        let accounts =
            AuthAccountStore::new(database, workspace.path()).map_err(anyhow::Error::msg)?;
        let scope = McpCatalogScope::Tenant { scope: TenantScope::new("platform", "company")? };
        let mut draft = http_draft("stdio-check", "unused");
        draft.connection = McpConnectionDefinition::Stdio {
            command: "/bin/sh".into(),
            args: vec!["-c".into(), STDIO_SERVER.into(), "fixture".into(), "private-arg".into()],
            env: [("FIXTURE_ENV".into(), McpConfigValue::Literal { value: "private-env".into() })]
                .into(),
            cwd: Some("fixture-workdir".into()),
        };
        let saved = catalog.create(&scope, draft).unwrap();
        assert_eq!(catalog.list(&scope).unwrap(), vec![saved.clone()]);
        assert!(!workdir.join("started").exists());

        let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build()?;
        let mut stale = saved.record.clone();
        stale.revision += 1;
        assert_eq!(
            runtime.block_on(catalog.check_connection(
                &scope,
                &stale,
                "local-fixture",
                workspace.path(),
                &accounts,
                |_, _| None,
            )),
            McpConnectionOutcome::Failed { code: McpFailureCode::StaleRevision },
        );
        assert!(!workdir.join("started").exists());
        let result = runtime.block_on(catalog.check_connection(
            &scope,
            &saved.record,
            "local-fixture",
            workspace.path(),
            &accounts,
            |_, _| None,
        ));
        assert!(workdir.join("started").exists());
        assert!(workdir.join("stopped").exists());
        assert_eq!(
            result,
            McpConnectionOutcome::Connected { tool_count: Some(1) },
            "the explicit check must report the discovered tool count",
        );
        assert_eq!(
            serde_json::to_value(result)?,
            serde_json::json!({"status":"connected","toolCount":1}),
        );
        Ok(())
    })
}

const STDIO_SERVER: &str = r#"
    [ "$1" = private-arg ] && [ "$FIXTURE_ENV" = private-env ] &&
    [ "${PWD##*/}" = fixture-workdir ] || exit 1
    : > started
    while IFS= read -r line; do
        id=${line#*\"id\":}
        id=${id%%,*}
        id=${id%%\}*}
        case "$line" in
            *'"method":"initialize"'*)
                printf '{"jsonrpc":"2.0","id":%s,"result":{"protocolVersion":"2025-03-26","capabilities":{"tools":{}},"serverInfo":{"name":"fixture","version":"1"}}}\n' "$id"
                ;;
            *'"method":"tools/list"'*)
                printf '{"jsonrpc":"2.0","id":%s,"result":{"tools":[{"name":"private-tool","inputSchema":{"type":"object"}}]}}\n' "$id"
                ;;
        esac
    done
    : > stopped
"#;

#[derive(Clone)]
enum RemoteMode {
    Streamable,
    Sse,
    HandshakeError,
    HandshakeErrorSse,
    DiscoveryError,
    DiscoveryTimeout,
    ForeignSse(String),
    Redirect(String),
}

#[test]
fn cc_mcp_t30_streamable_http_discovers_tools_with_authorized_header() -> anyhow::Result<()> {
    check_remote(RemoteMode::Streamable, McpConnectionOutcome::Connected { tool_count: Some(2) })
}

#[test]
fn cc_mcp_t30_legacy_sse_discovers_tools_with_authorized_header() -> anyhow::Result<()> {
    check_remote(RemoteMode::Sse, McpConnectionOutcome::Connected { tool_count: Some(2) })
}

#[test]
fn cc_mcp_t30_handshake_and_discovery_failures_return_distinct_codes() -> anyhow::Result<()> {
    check_remote(
        RemoteMode::HandshakeError,
        McpConnectionOutcome::Failed { code: McpFailureCode::ConnectionFailed },
    )?;
    check_remote(
        RemoteMode::HandshakeErrorSse,
        McpConnectionOutcome::Failed { code: McpFailureCode::ConnectionFailed },
    )?;
    check_remote(
        RemoteMode::DiscoveryError,
        McpConnectionOutcome::Failed { code: McpFailureCode::ToolDiscoveryFailed },
    )
}

#[test]
fn cc_mcp_t30_stalled_tool_discovery_times_out() -> anyhow::Result<()> {
    check_remote(
        RemoteMode::DiscoveryTimeout,
        McpConnectionOutcome::Failed { code: McpFailureCode::TimedOut },
    )
}

#[test]
fn cc_mcp_t30_foreign_endpoint_and_redirect_never_receive_headers() -> anyhow::Result<()> {
    let foreign = std::net::TcpListener::bind(("127.0.0.1", 0))?;
    foreign.set_nonblocking(true)?;
    let url = format!("http://{}/foreign", foreign.local_addr()?);
    let expected = McpConnectionOutcome::Failed { code: McpFailureCode::ConnectionFailed };
    check_remote(RemoteMode::ForeignSse(url.clone()), expected)?;
    assert_eq!(foreign.accept().unwrap_err().kind(), std::io::ErrorKind::WouldBlock);
    check_remote(RemoteMode::Redirect(url), expected)?;
    assert_eq!(foreign.accept().unwrap_err().kind(), std::io::ErrorKind::WouldBlock);
    Ok(())
}

const ACCESS: &str = "private-access-fixture";
const REFRESH: &str = "private-refresh-fixture";

fn check_remote(mode: RemoteMode, expected: McpConnectionOutcome) -> anyhow::Result<()> {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    use vw_agent_apps::auth::credential_file::{
        AccountCredential, CredentialFileStore, XaiCredential,
    };
    use vw_api_types::{TimestampMs, agent_apps_auth::AuthService};

    with_database(|database| {
        let workspace = tempfile::tempdir()?;
        fs::set_permissions(workspace.path(), fs::Permissions::from_mode(0o700))?;
        let tenant = TenantScope::new("platform", "company")?;
        let scope = McpCatalogScope::Tenant { scope: tenant.clone() };
        let owner = AuthAccountScope {
            tenant: &tenant,
            principal: "oidc:fixture",
            host: "local-fixture",
            service: AuthService::XaiOauth,
        };
        let accounts = AuthAccountStore::new(database.clone(), workspace.path())
            .map_err(anyhow::Error::msg)?;
        let catalog = McpCatalogStore::new(database).unwrap();
        let account_id = "fixture-account";
        accounts
            .insert_metadata(
                &owner,
                account_id,
                &AuthAccountMetadata::new("fixture-identity", TimestampMs(1)).unwrap(),
            )
            .unwrap();
        CredentialFileStore::new(workspace.path())
            .unwrap()
            .lease_xai(owner, account_id, true)
            .unwrap()
            .save(&AccountCredential::XaiOauth(XaiCredential {
                access_token: ACCESS.into(),
                refresh_token: REFRESH.into(),
                expires_at: TimestampMs(i64::MAX),
                requires_reauth: false,
            }))
            .unwrap();
        let reference = McpCredentialRef { service: owner.service, account_id: account_id.into() };

        let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build()?;
        runtime.block_on(async {
            let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0)).await?;
            let url = format!("http://{}/mcp?fixture=private-query", listener.local_addr()?);
            let sse = matches!(mode, RemoteMode::Sse | RemoteMode::ForeignSse(_));
            let hits = Arc::new(AtomicUsize::new(0));
            let (events, _) = tokio::sync::broadcast::channel(8);
            let state = RemoteFixture { mode, events, hits: hits.clone() };
            let routes = axum::Router::new()
                .route("/mcp", axum::routing::get(remote_sse).post(remote_post))
                .route("/messages", axum::routing::post(remote_post))
                .with_state(state);
            let server = tokio::spawn(async move { axum::serve(listener, routes).await });
            let headers = [(
                "Authorization".into(),
                McpConfigValue::Credential { reference: reference.clone() },
            )]
            .into();
            let mut draft = http_draft("remote-check", &url);
            draft.connection = if sse {
                McpConnectionDefinition::Sse { url, headers }
            } else {
                McpConnectionDefinition::StreamableHttp { url, headers }
            };
            let saved = run_sync_database(|| Ok(catalog.create(&scope, draft))).unwrap().unwrap();
            assert_eq!(
                run_sync_database(|| Ok(catalog.list(&scope))).unwrap().unwrap(),
                vec![saved.clone()],
            );

            let denied = catalog
                .check_connection(
                    &scope,
                    &saved.record,
                    owner.host,
                    workspace.path(),
                    &accounts,
                    |_, _| None,
                )
                .await;
            assert_eq!(
                denied,
                McpConnectionOutcome::Failed { code: McpFailureCode::PermissionDenied }
            );
            assert_eq!(hits.load(Ordering::Relaxed), 0);
            let outcome = catalog
                .check_connection(
                    &scope,
                    &saved.record,
                    owner.host,
                    workspace.path(),
                    &accounts,
                    |_, candidate| (candidate == &reference).then_some(owner),
                )
                .await;
            assert_eq!(outcome, expected);
            assert!(hits.load(Ordering::Relaxed) > 0);
            let public = serde_json::to_string(&outcome)?;
            for private in
                [ACCESS, REFRESH, account_id, "private-query", "private-tool", "private-error"]
            {
                assert!(!public.contains(private));
            }
            server.abort();
            assert!(server.await.unwrap_err().is_cancelled());
            Ok(())
        })
    })
}

#[derive(Clone)]
struct RemoteFixture {
    mode: RemoteMode,
    events: tokio::sync::broadcast::Sender<serde_json::Value>,
    hits: std::sync::Arc<std::sync::atomic::AtomicUsize>,
}

impl RemoteFixture {
    fn authorized(&self, headers: &HeaderMap) -> bool {
        self.hits.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        headers
            .get("authorization")
            .is_some_and(|header| header == format!("Bearer {ACCESS}").as_str())
    }
}

async fn remote_sse(State(state): State<RemoteFixture>, headers: HeaderMap) -> Response {
    use axum::response::sse::{Event, Sse};
    use futures_util::StreamExt as _;
    use std::convert::Infallible;

    if !state.authorized(&headers) {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    let endpoint = match &state.mode {
        RemoteMode::Sse => "/messages",
        RemoteMode::ForeignSse(url) => url,
        _ => return StatusCode::METHOD_NOT_ALLOWED.into_response(),
    };
    let initial = Event::default().event("endpoint").data(endpoint);
    let stream = futures_util::stream::unfold(state.events.subscribe(), |mut receiver| async {
        let message = receiver.recv().await.ok()?;
        Some((
            Ok::<_, Infallible>(Event::default().event("message").data(message.to_string())),
            receiver,
        ))
    });
    Sse::new(futures_util::stream::once(async { Ok(initial) }).chain(stream)).into_response()
}

async fn remote_post(
    State(state): State<RemoteFixture>,
    headers: HeaderMap,
    Json(request): Json<serde_json::Value>,
) -> Response {
    if !state.authorized(&headers) {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    if let RemoteMode::Redirect(url) = &state.mode {
        return (StatusCode::TEMPORARY_REDIRECT, [("location", url.as_str())]).into_response();
    }
    let method = request["method"].as_str();
    if method == Some("notifications/initialized") {
        return StatusCode::ACCEPTED.into_response();
    }
    let id = &request["id"];
    let failure = || {
        json!({
            "jsonrpc":"2.0", "id":id,
            "error":{"code":-32603,"message":"private-error","data":{"access":ACCESS}},
        })
    };
    let message = match method {
        Some("initialize") if matches!(state.mode, RemoteMode::HandshakeError) => failure(),
        Some("initialize") if matches!(state.mode, RemoteMode::HandshakeErrorSse) => {
            use axum::response::sse::{Event, Sse};
            use futures_util::{StreamExt as _, stream};
            use std::convert::Infallible;

            let event = Event::default().event("message").data(failure().to_string());
            let stream =
                stream::once(async { Ok::<_, Infallible>(event) }).chain(stream::pending());
            return Sse::new(stream).into_response();
        }
        Some("initialize") => json!({
            "jsonrpc":"2.0", "id":id,
            "result":{
                "protocolVersion":"2025-03-26", "capabilities":{"tools":{}},
                "serverInfo":{"name":"fixture","version":"1"},
            }
        }),
        Some("tools/list") if matches!(state.mode, RemoteMode::DiscoveryError) => failure(),
        Some("tools/list") if matches!(state.mode, RemoteMode::DiscoveryTimeout) => {
            tokio::time::pause();
            tokio::time::advance(std::time::Duration::from_secs(31)).await;
            return std::future::pending().await;
        }
        Some("tools/list") => {
            let first = request["params"]["cursor"].is_null();
            let name = if first { "private-tool-first" } else { "private-tool-last" };
            let mut result = json!({"tools":[{"name":name,"inputSchema":{"type":"object"}}]});
            if first {
                result["nextCursor"] = json!("next-fixture-page");
            }
            json!({"jsonrpc":"2.0", "id":id, "result":result})
        }
        _ => return StatusCode::BAD_REQUEST.into_response(),
    };
    if matches!(state.mode, RemoteMode::Sse) {
        if state.events.send(message).is_err() {
            return StatusCode::SERVICE_UNAVAILABLE.into_response();
        }
        StatusCode::ACCEPTED.into_response()
    } else {
        Json(message).into_response()
    }
}
