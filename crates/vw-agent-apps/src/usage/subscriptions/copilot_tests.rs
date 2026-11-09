use super::copilot::query_at;
use crate::{
    auth::{
        copilot::CopilotAuth,
        copilot_tests::oauth_server,
        credential_file::{AccountCredential, CredentialFileStore},
        flow::AuthAccountScope,
    },
    usage::binding::{
        UnboundUsageAccount, UsageAccountSelection, UsageCredentialRef, resolve_usage_credential,
    },
};
use reqwest::{Request, Url, header};
use serde_json::{Value, json};
use std::os::unix::fs::PermissionsExt;
use vw_api_types::{
    TimestampMs,
    agent_apps::AppId,
    agent_apps_auth::{AuthErrorCode, AuthService},
    apps_usage::{
        UsageCredentialState, UsageQueryData, UsageQueryErrorCode, UsageQueryOutcome, UsageSource,
    },
    id::TenantScope,
};

const NOW: TimestampMs = TimestampMs(1_800_000_000_000);

fn scope(tenant: &TenantScope) -> AuthAccountScope<'_> {
    AuthAccountScope {
        tenant,
        principal: "oidc:owner",
        host: "mac",
        service: AuthService::GithubCopilot,
    }
}

fn store(root: &tempfile::TempDir) -> CredentialFileStore {
    std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    CredentialFileStore::new(root.path()).unwrap()
}

fn save(store: &CredentialFileStore, scope: AuthAccountScope<'_>, id: &str, token: &str) {
    store
        .lease_copilot(scope, id, true)
        .unwrap()
        .save(&AccountCredential::CopilotToken(token.to_owned()))
        .unwrap();
}

fn resolve<'a>(
    scope: AuthAccountScope<'a>,
    explicit: Option<&str>,
    default: Option<&str>,
) -> Result<UsageCredentialRef<'a>, AuthErrorCode> {
    resolve_usage_credential(
        scope,
        scope,
        UsageAccountSelection {
            app: AppId::ClaudeDesktop,
            account_id: explicit,
            bound_account_id: None,
            unbound: UnboundUsageAccount::ManagedDefault,
        },
        |owner| {
            assert!(scope == owner);
            assert!(explicit.is_none());
            Ok(default.map(str::to_owned))
        },
        |owner, id| {
            assert!(scope == owner);
            if matches!(id, "explicit" | "default") { Ok(()) } else { Err(AuthErrorCode::NotFound) }
        },
    )
}

fn quota(plan: &str, reset: &str, entitlement: i64, remaining: i64) -> Value {
    json!({
        "copilot_plan": plan,
        "quota_reset_date": reset,
        "quota_snapshots": {
            "chat": {"entitlement": 90, "remaining": 1},
            "premium_interactions": {"entitlement": entitlement, "remaining": remaining}
        },
        "endpoints": {"api": "https://unused.example.test"}
    })
}

fn route(request: &mut Request, origin: Url, expected_url: &str) {
    assert_eq!(request.url().as_str(), expected_url);
    assert!(request.headers()[header::AUTHORIZATION].is_sensitive());
    let mut target = origin;
    target.set_path(request.url().path());
    *request.url_mut() = target;
}

#[tokio::test]
async fn usage_subscriptions_copilot_returns_expected_result() {
    let root = tempfile::tempdir().unwrap();
    let store = store(&root);
    let tenant = TenantScope::default();
    let scope = scope(&tenant);
    let adapter = CopilotAuth::new().unwrap();
    save(&store, scope, "explicit", "fixture-token-explicit");
    save(&store, scope, "default", "fixture-token-default");
    save(
        &store,
        AuthAccountScope { principal: "oidc:other", ..scope },
        "default",
        "fixture-token-neighbor",
    );

    for (id, authority, host, path, token, plan, reset, total, remaining, percent) in [
        (
            Some("explicit"),
            "github.com",
            "api.github.com",
            "/copilot_internal/user",
            "fixture-token-explicit",
            "individual",
            "2027-02-01",
            100,
            75,
            25.0,
        ),
        (
            None,
            "enterprise.example.test:8443",
            "enterprise.example.test:8443",
            "/api/v3/copilot_internal/user",
            "fixture-token-default",
            "business",
            "2027-03-01T00:00:00Z",
            0,
            0,
            0.0,
        ),
    ] {
        let account = resolve(scope, id, Some("default")).unwrap();
        let (url, received) =
            oauth_server("200 OK", "", quota(plan, reset, total, remaining)).await;
        let result = query_at(&adapter, &store, &account, authority, NOW, "quota-1", |request| {
            route(request, url, &format!("https://{host}{path}"));
        })
        .await
        .unwrap();

        let sent = received.await.unwrap();
        assert!(sent.starts_with(&format!("GET {path} HTTP/1.1\r\n")));
        assert!(sent.contains(&format!("\r\nauthorization: token {token}\r\n")));
        assert!(sent.contains("\r\ncontent-type: application/json\r\n"));
        assert!(sent.contains("\r\neditor-version: vscode/1.110.1\r\n"));
        assert!(sent.contains("\r\neditor-plugin-version: copilot-chat/0.38.2\r\n"));
        assert!(sent.contains("\r\nuser-agent: GitHubCopilotChat/0.38.2\r\n"));
        assert!(sent.contains("\r\nx-github-api-version: 2025-10-01\r\n"));

        assert_eq!(result.source, UsageSource::Official("github_copilot".into()));
        assert_eq!(result.queried_at, Some(NOW));
        assert_eq!(result.credential_state, UsageCredentialState::Valid);
        let UsageQueryOutcome::Success { data: UsageQueryData::OfficialSubscription(quota) } =
            result.outcome
        else {
            panic!("expected Copilot quota")
        };
        assert_eq!(quota.subscription_type.as_deref(), Some(plan));
        let [premium] = quota.windows.as_slice() else { panic!("expected only premium quota") };
        assert_eq!(premium.name, "premium");
        assert_eq!(premium.resets_at.as_deref(), Some(reset));
        assert_eq!(premium.utilization.map(f64::from), Some(percent));
        assert_eq!(premium.total.map(f64::from), Some(total as f64));
        assert_eq!(premium.remaining.map(f64::from), Some(remaining as f64));
    }
}

#[tokio::test]
async fn usage_subscriptions_copilot_rejects_invalid_input() {
    let root = tempfile::tempdir().unwrap();
    let store = store(&root);
    let tenant = TenantScope::default();
    let scope = scope(&tenant);
    let adapter = CopilotAuth::new().unwrap();
    save(&store, scope, "default", "fixture-token-default");
    save(&store, AuthAccountScope { host: "other-mac", ..scope }, "explicit", "fixture-neighbor");

    assert_eq!(
        resolve(scope, Some("deleted"), Some("default")).err(),
        Some(AuthErrorCode::NotFound)
    );
    assert_eq!(resolve(scope, None, None).err(), Some(AuthErrorCode::NotFound));
    let explicit = resolve(scope, Some("explicit"), Some("default")).unwrap();
    let missing = query_at(&adapter, &store, &explicit, "github.com", NOW, "quota-2", |_| {
        panic!("must not send a request with another account's token");
    })
    .await
    .unwrap();
    assert_eq!(missing.queried_at, None);
    assert_eq!(missing.credential_state, UsageCredentialState::NotFound);

    let account = resolve(scope, None, Some("default")).unwrap();
    let invalid = query_at(&adapter, &store, &account, "github.com/path", NOW, "quota-2", |_| {
        panic!("must not send a request to an invalid authority");
    })
    .await;
    assert_eq!(invalid.err(), Some(AuthErrorCode::InvalidInput));

    for (status, body, code, credential) in [
        (
            "401 Unauthorized",
            json!({"error": "fixture-private-response"}),
            UsageQueryErrorCode::UpstreamUnauthorized,
            UsageCredentialState::Expired,
        ),
        (
            "403 Forbidden",
            json!({"error": "fixture-private-response"}),
            UsageQueryErrorCode::UpstreamRejected,
            UsageCredentialState::Valid,
        ),
        (
            "200 OK",
            json!({"quota_snapshots": {"chat": "fixture-private-response"}}),
            UsageQueryErrorCode::InvalidResponse,
            UsageCredentialState::Valid,
        ),
        (
            "200 OK",
            quota("fixture-token-default", "2027-02-01", 100, 75),
            UsageQueryErrorCode::InvalidResponse,
            UsageCredentialState::Valid,
        ),
    ] {
        let (url, received) = oauth_server(status, "", body).await;
        let result =
            query_at(&adapter, &store, &account, "github.com", NOW, "quota-2", |request| {
                route(request, url, "https://api.github.com/copilot_internal/user");
            })
            .await
            .unwrap();
        received.await.unwrap();
        assert_eq!(result.credential_state, credential);
        assert_eq!(result.queried_at, Some(NOW));
        let UsageQueryOutcome::Failure { error, .. } = result.outcome else {
            panic!("expected a failed quota query");
        };
        assert_eq!(error.code, code);
        assert_eq!(error.correlation_id, "quota-2");
        assert!(!error.message.contains("fixture-private-response"));
        assert!(!error.message.contains("fixture-token-default"));
    }
}
