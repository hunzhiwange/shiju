use super::{
    copilot::CopilotAuth,
    copilot_tests::oauth_server,
    copilot_token::CopilotSession,
    credential_file::{AccountCredential, CredentialFileStore},
    flow::AuthAccountScope,
};
use reqwest::{Request, Url, header};
use serde_json::{Value, json};
use std::{cell::Cell, collections::VecDeque, os::unix::fs::PermissionsExt};
use tokio::task::JoinHandle;
use vw_api_types::{
    agent_apps_auth::{AuthErrorCode, AuthService},
    id::TenantScope,
};

fn scope(tenant: &TenantScope) -> AuthAccountScope<'_> {
    AuthAccountScope {
        tenant,
        host: "host",
        principal: "oidc:owner",
        service: AuthService::GithubCopilot,
    }
}

fn store(root: &tempfile::TempDir) -> CredentialFileStore {
    std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    CredentialFileStore::new(root.path()).unwrap()
}

fn save(store: &CredentialFileStore, scope: AuthAccountScope<'_>, id: &str, oauth: &str) {
    store
        .lease_copilot(scope, id, true)
        .unwrap()
        .save(&AccountCredential::CopilotToken(oauth.to_owned()))
        .unwrap();
}

struct Reply {
    url: Url,
    request: JoinHandle<String>,
    upstream: String,
    oauth: &'static str,
}

impl Reply {
    async fn new(upstream: &str, oauth: &'static str, status: &str, body: Value) -> Self {
        let (mut url, request) = oauth_server(status, "", body).await;
        url.set_path(Url::parse(upstream).unwrap().path());
        Self { url, request, upstream: upstream.to_owned(), oauth }
    }

    fn route(&self, outbound: &mut Request) {
        assert_eq!(outbound.url().as_str(), self.upstream);
        assert_eq!(outbound.headers()[header::AUTHORIZATION], format!("token {}", self.oauth));
        assert!(outbound.headers()[header::AUTHORIZATION].is_sensitive());
        *outbound.url_mut() = self.url.clone();
    }

    async fn verify(self) {
        let request = self.request.await.unwrap();
        assert!(request.starts_with(&format!("GET {} HTTP/1.1\r\n", self.url.path())));
        assert!(request.contains(&format!("\r\nauthorization: token {}\r\n", self.oauth)));
        assert!(request.contains("\r\neditor-version: vscode/1.110.1\r\n"));
        assert!(request.contains("\r\neditor-plugin-version: copilot-chat/0.38.2\r\n"));
        if self.url.path().ends_with("/user") {
            assert!(request.contains("\r\nx-github-api-version: 2025-10-01\r\n"));
        }
    }
}

const TOKEN: &str = "https://api.github.com/copilot_internal/v2/token";
const USAGE: &str = "https://api.github.com/copilot_internal/user";

async fn public_replies(
    oauth: &'static str,
    token: &str,
    expiry: i64,
    endpoint: &str,
) -> [Reply; 2] {
    [
        Reply::new(TOKEN, oauth, "200 OK", json!({"token": token, "expires_at": expiry})).await,
        Reply::new(USAGE, oauth, "200 OK", json!({"endpoints": {"api": endpoint}})).await,
    ]
}

fn assert_session(result: Result<CopilotSession, AuthErrorCode>, token: &str, endpoint: &str) {
    assert!(result.is_ok(), "scoped Copilot session rejected: {:?}", result.as_ref().err());
    let session = result.ok().unwrap();
    assert_eq!(session.authorization, format!("Bearer {token}"));
    assert!(session.authorization.is_sensitive());
    assert_eq!(session.endpoint.as_str(), endpoint);
}

#[tokio::test]
async fn auth6_copilot_token_commits_expected_scoped_result() {
    let root = tempfile::tempdir().unwrap();
    let store = store(&root);
    let tenant = TenantScope::new("platform", "company").unwrap();
    let scope = scope(&tenant);
    let foreign = AuthAccountScope { principal: "oidc:other", ..scope };
    let auth = CopilotAuth::new().unwrap();
    let now = Cell::new(1_000);
    let business = "https://api.business.githubcopilot.com/";
    let enterprise = "https://api.enterprise.githubcopilot.com/";
    save(&store, scope, "first", "oauth-a");
    save(&store, scope, "second", "oauth-b");
    // Use the same fixture identity and OAuth bytes to make a cache-key scope omission observable.
    save(&store, foreign, "first", "oauth-a");

    for (owner, id, oauth, token, endpoint) in [
        (scope, "first", "oauth-a", "session-a", business),
        (scope, "second", "oauth-b", "session-b", enterprise),
        (foreign, "first", "oauth-a", "session-foreign", enterprise),
    ] {
        let mut pending = VecDeque::from(public_replies(oauth, token, 1_300, endpoint).await);
        let mut sent = Vec::new();
        let fetch = auth.session_with(
            &store,
            owner,
            id,
            "github.com",
            || Ok(now.get()),
            |outbound| {
                let reply = pending.pop_front().unwrap();
                reply.route(outbound);
                sent.push(reply);
            },
        );
        let cache = async {
            auth.session_with(
                &store,
                owner,
                id,
                "github.com",
                || Ok(now.get()),
                |_| {
                    panic!("same scoped account made a duplicate exchange");
                },
            )
            .await
        };
        let (first, second) = tokio::join!(fetch, cache);
        assert_session(first, token, endpoint);
        assert_session(second, token, endpoint);
        assert!(pending.is_empty());
        for reply in sent {
            reply.verify().await;
        }
    }

    let ghes = "corp.example.com:8443";
    let reply = Reply::new(
        "https://corp.example.com:8443/api/v3/copilot_internal/user",
        "oauth-a",
        "200 OK",
        json!({}),
    )
    .await;
    let result = auth
        .session_with(
            &store,
            scope,
            "first",
            ghes,
            || Ok(now.get()),
            |outbound| {
                reply.route(outbound);
            },
        )
        .await;
    assert_session(result, "oauth-a", "https://copilot-api.corp.example.com:8443/");
    reply.verify().await;
    let result = auth
        .session_with(
            &store,
            scope,
            "first",
            "github.com",
            || Ok(now.get()),
            |_| {
                panic!("Enterprise resolution overwrote the public domain cache");
            },
        )
        .await;
    assert_session(result, "session-a", business);

    save(&store, scope, "first", "oauth-rotated");
    for (time, token, expiry) in
        [(1_001, "session-rotated", 1_100), (1_040, "session-renewed", 1_400)]
    {
        now.set(time);
        let [exchange, discovery] = public_replies("oauth-rotated", token, expiry, business).await;
        let result = auth
            .session_with(
                &store,
                scope,
                "first",
                "github.com",
                || Ok(now.get()),
                |outbound| {
                    if outbound.url().path().ends_with("/token") {
                        exchange.route(outbound);
                    } else {
                        discovery.route(outbound);
                    }
                },
            )
            .await;
        assert_session(result, token, business);
        exchange.verify().await;
        discovery.verify().await;
    }
}

#[tokio::test]
async fn auth6_copilot_token_rejects_invalid_or_stale_input() {
    let root = tempfile::tempdir().unwrap();
    let store = store(&root);
    let tenant = TenantScope::new("platform", "company").unwrap();
    let scope = scope(&tenant);
    let auth = CopilotAuth::new().unwrap();
    let now = || Ok(1_000);

    let invalid = auth
        .session_with(&store, scope, "first", "github.com@other.example", now, |_| {
            panic!("invalid authority reached the network");
        })
        .await;
    assert_eq!(invalid.err(), Some(AuthErrorCode::InvalidInput));

    save(&store, scope, "first", "oauth-a");
    let foreign = AuthAccountScope { host: "other-host", ..scope };
    let missing = auth
        .session_with(&store, foreign, "first", "github.com", now, |_| {
            panic!("foreign scope used the stored OAuth token");
        })
        .await;
    assert_eq!(missing.err(), Some(AuthErrorCode::StorageUnavailable));

    for (status, body, expected) in [
        (
            "200 OK",
            json!({"token": "stale-session", "expires_at": 1_000}),
            AuthErrorCode::UpstreamFailure,
        ),
        (
            "200 OK",
            json!({"token": "oauth-a", "expires_at": 1_300}),
            AuthErrorCode::UpstreamFailure,
        ),
        ("401 Unauthorized", json!({"message": "oauth-a"}), AuthErrorCode::ReauthRequired),
        ("403 Forbidden", json!({"message": "oauth-a"}), AuthErrorCode::AuthorizationDenied),
    ] {
        let exchange = Reply::new(TOKEN, "oauth-a", status, body).await;
        let result = auth
            .session_with(&store, scope, "first", "github.com", now, |outbound| {
                exchange.route(outbound)
            })
            .await;
        assert_eq!(result.err(), Some(expected));
        exchange.verify().await;
    }

    for endpoint in [
        "http://api.githubcopilot.com/",
        "https://api.githubcopilot.com.evil.example/",
        "https://api.githubcopilot.com@evil.example/",
        "https://api.githubcopilot.com/path",
        "https://api.githubcopilot.com/?query=secret",
        "https://api.githubcopilot.com:8443/",
    ] {
        let [exchange, discovery] = public_replies("oauth-a", "session-a", 1_300, endpoint).await;
        let result = auth
            .session_with(&store, scope, "first", "github.com", now, |outbound| {
                if outbound.url().path().ends_with("/token") {
                    exchange.route(outbound);
                } else {
                    discovery.route(outbound);
                }
            })
            .await;
        assert_eq!(result.err(), Some(AuthErrorCode::UpstreamFailure));
        exchange.verify().await;
        discovery.verify().await;
    }

    let reply = Reply::new(
        "https://corp.example.com/api/v3/copilot_internal/user",
        "oauth-a",
        "200 OK",
        json!({"endpoints": {"api": "https://copilot-api.other.example.com/"}}),
    )
    .await;
    let result = auth
        .session_with(&store, scope, "first", "corp.example.com", now, |outbound| {
            reply.route(outbound)
        })
        .await;
    assert_eq!(result.err(), Some(AuthErrorCode::UpstreamFailure));
    reply.verify().await;
}
