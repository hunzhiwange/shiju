use super::agent_apps_auth_read;
use crate::{
    agent_apps::auth_accounts::{AuthAccountMetadata, AuthAccountScope, AuthAccountStore},
    global_database::{GlobalDatabase, run_sync_database, test_support::PostgresTestDatabase},
};
use axum::{
    Extension,
    http::{StatusCode, header},
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, net::SocketAddr, os::unix::fs::PermissionsExt, path::Path};
use tokio::{net::TcpListener, task::JoinHandle};
use vw_api_types::{TimestampMs, agent_apps_auth::AuthService, id::TenantScope};

const OWNER: &str = "fixture-owner-token-aaaaaaaaaaaa";
const OTHER: &str = "fixture-other-token-bbbbbbbbbbbb";
const READER: &str = "fixture-reader-token-cccccccccccc";
const BASIC: &str = "Basic dGVzdDpzZWNyZXQ=";
const FLOWS: &str = "/v1/agent-apps/auth/flows";
const CLAUDE: &str = "host_id=host&service=claude_official";
const CODEX: &str = "host_id=host&service=codex_oauth";
const LINK: &str = "https://claude.com/cai/oauth/authorize?code=true&state=fixture";
const PASTE: &str = "fixture_valid_code#fixture";

struct Gateway {
    addr: SocketAddr,
    client: reqwest::Client,
    server: JoinHandle<()>,
}

impl Gateway {
    async fn start(home: &Path, database: GlobalDatabase) -> anyhow::Result<Self> {
        let root = home.join("credentials");
        fs::create_dir(&root)?;
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700))?;
        let policy = home.join("policy.json");
        let principal = |id, token: &str, start: bool| {
            let grants = ["host", "remote", "offline"].map(|host| {
                let mut grant = json!({"platform_id":"p","company_id":"c","host_id":host,
                                       "auth_account_read":["claude_official","codex_oauth"]});
                if start {
                    grant["auth_flow_start"] = json!([
                        "claude_official",
                        "codex_oauth",
                        "kimi_oauth",
                        "github_copilot",
                        "xai_oauth"
                    ]);
                }
                if id == "owner" && host == "host" {
                    grant["auth_browser_local"] = true.into();
                }
                grant
            });
            json!({"id":id, "token_sha256":hex::encode(Sha256::digest(token)), "grants":grants})
        };
        let execution_home =
            std::path::PathBuf::from(std::env::var_os("HOME").expect("test execution HOME"));
        let hosts =
            [("host", "local", true), ("remote", "remote", true), ("offline", "local", false)].map(
                |(id, binding, online)| {
                    json!({"id":id, "binding":binding, "online":online,
                "home":execution_home, "applications":home, "search_paths":[home], "credential_root":root})
                },
            );
        fs::write(
            &policy,
            serde_json::to_vec(&json!({
                "principals":[principal("owner", OWNER, true),principal("other", OTHER, true),principal("reader", READER, false)],
                "hosts":hosts,
            }))?,
        )?;
        fs::set_permissions(&policy, fs::Permissions::from_mode(0o600))?;

        run_sync_database(|| {
            let store =
                AuthAccountStore::new(database.clone(), &root).map_err(anyhow::Error::msg)?;
            let tenant = TenantScope::new("p", "c")?;
            let scope = AuthAccountScope {
                tenant: &tenant,
                principal: "other",
                host: "host",
                service: AuthService::CodexOauth,
            };
            store
                .insert_metadata(
                    &scope,
                    "other-account",
                    &AuthAccountMetadata::new("fixture", TimestampMs(1)).unwrap(),
                )
                .unwrap();
            let mut kimi = AuthAccountMetadata::new("fixture-kimi", TimestampMs(1)).unwrap();
            kimi.region = Some(vw_api_types::agent_apps_auth::AuthRegion::MainlandCn);
            store
                .insert_metadata(
                    &AuthAccountScope {
                        principal: "owner",
                        service: AuthService::KimiOauth,
                        ..scope
                    },
                    "kimi-account",
                    &kimi,
                )
                .unwrap();
            Ok(())
        })?;
        let program = home.join("claude");
        fs::write(
            &program,
            format!(
                r#"#!/bin/sh
set -eu
test "$*" = 'auth login --claudeai'
test "$PWD" = "$CLAUDE_CONFIG_DIR"
"$BROWSER" '{LINK}'
IFS= read -r code
test "$code" = '{PASTE}'
umask 077
printf '%s' '{{"claudeAiOauth":{{"refreshToken":"fixture-private-refresh","accessToken":"fixture-private-access"}}}}' > .credentials.json
printf '%s' '{{"oauthAccount":{{"accountUuid":"fixture-claude-user","organizationUuid":"fixture-org"}}}}' > .claude.json
printf delivered > '{}'
"#,
                home.join("delivered").display()
            ),
        )?;
        fs::set_permissions(&program, fs::Permissions::from_mode(0o700))?;

        let router = crate::gateway::router::build_router_with_auth(
            vec![],
            agent_apps_auth_read::router(Some("secret"), "test", Some(policy)),
        )
        .layer(Extension(database));
        let listener = TcpListener::bind(("127.0.0.1", 0)).await?;
        let addr = listener.local_addr()?;
        let server = tokio::spawn(async move {
            axum::serve(listener, router.into_make_service_with_connect_info::<SocketAddr>())
                .await
                .unwrap();
        });
        let client = reqwest::Client::builder()
            .no_proxy()
            .timeout(std::time::Duration::from_secs(10))
            .build()?;
        Ok(Self { addr, client, server })
    }

    async fn post(
        &self,
        path: &str,
        query: &str,
        token: &str,
        body: Value,
    ) -> anyhow::Result<(StatusCode, Value)> {
        let response = self
            .client
            .post(format!("http://{}{path}?{query}", self.addr))
            .header(header::AUTHORIZATION, BASIC)
            .header("x-platform-id", "p")
            .header("x-company-id", "c")
            .header("x-agent-apps-token", token)
            .json(&body)
            .send()
            .await?;
        let status = response.status();
        if path.starts_with(FLOWS) {
            assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
        }
        Ok((status, serde_json::from_slice(&response.bytes().await?).unwrap_or(Value::Null)))
    }
}

impl Drop for Gateway {
    fn drop(&mut self) {
        self.server.abort();
    }
}

#[test]
fn auth6_api_start_commits_expected_scoped_result() -> anyhow::Result<()> {
    let home = tempfile::tempdir()?;
    let database = GlobalDatabase::for_home(home.path())?;
    tokio::runtime::Runtime::new()?.block_on(check_start(home.path(), database))
}

#[test]
fn auth6_api_start_commits_expected_scoped_result_postgres() -> anyhow::Result<()> {
    let home = tempfile::tempdir()?;
    let database = PostgresTestDatabase::from_environment()?;
    tokio::runtime::Runtime::new()?.block_on(check_start(home.path(), database.database().clone()))
}

async fn check_start(home: &Path, database: GlobalDatabase) -> anyhow::Result<()> {
    let gateway = Gateway::start(home, database).await?;
    let (status, body) = gateway.post(FLOWS, CLAUDE, OWNER, json!({"method":"copy_link"})).await?;
    assert_eq!(status, StatusCode::CREATED, "authorized Claude start");
    let flow = body["flow_id"].as_str().unwrap();
    assert!(uuid::Uuid::parse_str(flow).is_ok());
    assert_eq!(body["verification_url"], LINK);
    assert_eq!(body["can_paste"], true);
    assert_eq!(body["state"], "pending");
    assert_eq!(body["retry_after_seconds"], 1);
    assert!(body["expires_at"].as_i64().unwrap() > 0);
    assert_eq!(body.as_object().unwrap().len(), 6);
    let path = format!("{FLOWS}/{flow}/paste");
    let payload = json!({"pasted": PASTE});
    let (status, body) = gateway.post(&path, CLAUDE, OTHER, payload.clone()).await?;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["code"], "not_found");
    assert_eq!(gateway.post(&path, CLAUDE, OWNER, payload).await?.0, StatusCode::ACCEPTED);
    tokio::time::timeout(std::time::Duration::from_secs(3), async {
        while !home.join("delivered").is_file() {
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
    })
    .await?;
    assert_eq!(fs::read_to_string(home.join("delivered"))?, "delivered");
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            let response = gateway
                .client
                .get(format!("http://{}/v1/agent-apps/auth/accounts?{CLAUDE}", gateway.addr))
                .header(header::AUTHORIZATION, BASIC)
                .header("x-platform-id", "p")
                .header("x-company-id", "c")
                .header("x-agent-apps-token", OWNER)
                .send()
                .await?;
            assert_eq!(response.status(), StatusCode::OK);
            let raw = response.text().await?;
            assert!(!raw.contains("fixture-private"));
            let body: Value = serde_json::from_str(&raw)?;
            if let Some(account) = body["accounts"].as_array().unwrap().first() {
                assert_eq!(account["credential_state"], "ready");
                assert_eq!(account["service"], "claude_official");
                assert_ne!(account["account_id"], flow);
                assert_eq!(
                    gateway.post(&path, CLAUDE, OWNER, json!({"pasted":PASTE})).await?.0,
                    StatusCode::CONFLICT
                );
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(30)).await;
        }
        Ok::<_, anyhow::Error>(())
    })
    .await??;
    Ok(())
}

#[test]
fn auth6_api_start_rejects_invalid_or_stale_input() -> anyhow::Result<()> {
    let home = tempfile::tempdir()?;
    let database = GlobalDatabase::for_home(home.path())?;
    tokio::runtime::Runtime::new()?.block_on(check_rejections(home.path(), database))
}

#[test]
fn auth6_api_start_rejects_invalid_or_stale_input_postgres() -> anyhow::Result<()> {
    let home = tempfile::tempdir()?;
    let database = PostgresTestDatabase::from_environment()?;
    tokio::runtime::Runtime::new()?
        .block_on(check_rejections(home.path(), database.database().clone()))
}

async fn check_rejections(home: &Path, database: GlobalDatabase) -> anyhow::Result<()> {
    let gateway = Gateway::start(home, database).await?;
    for (query, token, request, status, code) in [
        (CLAUDE, READER, json!({"method":"copy_link"}), 403, "forbidden"),
        (CLAUDE, OWNER, json!({"method":"device_code"}), 501, "unsupported"),
        (CLAUDE, OWNER, json!({"method":"unknown"}), 400, "invalid_input"),
        (CLAUDE, OWNER, json!({"method":"browser","program":"secret"}), 400, "invalid_input"),
        ("host_id=host&service=unknown", OWNER, json!({"method":"browser"}), 400, "invalid_input"),
        (
            "host_id=host&service=kimi_oauth",
            OWNER,
            json!({"method":"device_code"}),
            400,
            "invalid_input",
        ),
        (
            "host_id=host&service=github_copilot",
            OWNER,
            json!({"method":"device_code","github_domain":"https://secret.example/path"}),
            400,
            "invalid_input",
        ),
        (
            CODEX,
            OWNER,
            json!({"method":"device_code","target_account_id":"other-account"}),
            404,
            "not_found",
        ),
        (
            "host_id=remote&service=claude_official",
            OWNER,
            json!({"method":"copy_link"}),
            501,
            "execution_channel_unavailable",
        ),
        (
            "host_id=offline&service=claude_official",
            OWNER,
            json!({"method":"copy_link"}),
            503,
            "execution_host_unavailable",
        ),
        (CODEX, OTHER, json!({"method":"browser"}), 409, "callback_unreachable"),
        (
            "host_id=host&service=kimi_oauth",
            OWNER,
            json!({"method":"device_code","target_account_id":"kimi-account","region":"global"}),
            400,
            "invalid_input",
        ),
    ] {
        let (actual, body) = gateway.post(FLOWS, query, token, request).await?;
        assert_eq!(actual.as_u16(), status, "{code}");
        assert_eq!(body["code"], code);
        assert!(!body.to_string().contains("secret"));
    }
    let path = format!("{FLOWS}/{}/paste", uuid::Uuid::new_v4());
    let (status, body) = gateway.post(&path, CLAUDE, OWNER, json!({"pasted":PASTE})).await?;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(!body.to_string().contains(PASTE));
    assert!(!home.join("delivered").exists());
    Ok(())
}
