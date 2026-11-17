use super::{agent_apps_auth_read, agent_apps_auth_start::Runtime};
use crate::{
    agent_apps::auth_accounts::{AuthAccountMetadata, AuthAccountScope, AuthAccountStore},
    global_database::{GlobalDatabase, run_sync_database},
};
use axum::{
    Extension,
    http::{StatusCode, header},
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, net::SocketAddr, os::unix::fs::PermissionsExt, path::Path, sync::Arc};
use tokio::{net::TcpListener, task::JoinHandle};
use {
    vw_api_transport::TimestampMs,
    vw_api_app_auth::AuthService,
    vw_api_transport::id::TenantScope,
};

pub(super) const OWNER: &str = "fixture-owner-token-aaaaaaaaaaaa";
pub(super) const OTHER: &str = "fixture-other-token-bbbbbbbbbbbb";
pub(super) const READER: &str = "fixture-reader-token-cccccccccccc";
pub(super) const BASIC: &str = "Basic dGVzdDpzZWNyZXQ=";
pub(super) const FLOWS: &str = "/v1/agent-apps/auth/flows";
pub(super) const CLAUDE: &str = "host_id=host&service=claude_official";
pub(super) const CODEX: &str = "host_id=host&service=codex_oauth";
const LINK: &str = "https://claude.com/cai/oauth/authorize?code=true&state=fixture";
pub(super) const PASTE: &str = "fixture_valid_code#fixture";

pub(super) struct Gateway {
    pub(super) addr: SocketAddr,
    pub(super) client: reqwest::Client,
    server: JoinHandle<()>,
}

impl Gateway {
    pub(super) async fn start(home: &Path, database: GlobalDatabase) -> anyhow::Result<Self> {
        Self::start_with_runtime(home, database, Arc::new(Runtime::default())).await
    }

    pub(super) async fn start_with_runtime(
        home: &Path,
        database: GlobalDatabase,
        runtime: Arc<Runtime>,
    ) -> anyhow::Result<Self> {
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
            kimi.region = Some(vw_api_app_auth::AuthRegion::MainlandCn);
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
            agent_apps_auth_read::router_with_runtime(
                Some("secret"),
                "test",
                Some(policy),
                runtime,
            ),
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

    pub(super) async fn post(
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

#[tokio::test]
async fn host_auth_start_uses_real_auth_wiring() -> anyhow::Result<()> {
    let home = tempfile::tempdir()?;
    let gateway = Gateway::start(home.path(), GlobalDatabase::for_home(home.path())?).await?;
    let (status, body) = gateway.post(FLOWS, CLAUDE, READER, json!({"method":"copy_link"})).await?;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["code"], "forbidden");
    Ok(())
}
