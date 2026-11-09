use std::{fs, os::unix::fs::PermissionsExt, time::Duration};

use serde_json::{Value, json};
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    net::TcpListener,
    time::timeout,
};
use vw_api_types::{
    TimestampMs,
    agent_apps_auth::{AuthErrorCode, AuthService},
    apps_usage::{
        UsageCredentialState, UsageQueryData, UsageQueryErrorCode, UsageQueryOutcome,
        UsageQueryResult, UsageSource,
    },
    id::TenantScope,
};

use super::agy::{query_account_at, query_cli_at};
use crate::{apps::discovery::read::LocalReadEnvironment, auth::flow::AuthAccountScope};

const NOW: TimestampMs = TimestampMs(1_893_456_000_000);
const QUERY: &str = "agy-fixture-query";
const ACCOUNT: &str = "abcdefghijklmnopqrstuvwxyz0123456789abcdefg";
const RESET: &str = "2030-01-08T00:00:00Z";
const SECRET_MARKER: &str = "private-upstream-payload";
const SUMMARY: &str = r#"{"groups":[
    {"displayName":"Gemini Models","buckets":[
        {"bucketId":"gemini-weekly","remainingFraction":0.75,"resetTime":"2030-01-08T00:00:00Z"},
        {"bucketId":"gemini-5h","remainingFraction":1},
        {"bucketId":"gemini-5h","remainingFraction":0.5,"resetTime":"2030-01-08T00:00:00Z"}
    ]},
    {"displayName":"Claude and GPT models","buckets":[
        {"bucketId":"3p-weekly","remainingFraction":0.25},
        {"bucketId":"3p-5h","remainingFraction":0},
        {"bucketId":"future-bucket","remainingFraction":1}
    ]}
]}"#;

fn scope(tenant: &TenantScope) -> AuthAccountScope<'_> {
    AuthAccountScope {
        tenant,
        principal: "owner",
        host: "fixture-host",
        service: AuthService::AgyOauth,
    }
}

struct Reply<'a> {
    method: &'a str,
    token: &'a str,
    status: &'a str,
    body: &'a str,
}

async fn replay(listener: &TcpListener, replies: &[Reply<'_>]) {
    for reply in replies {
        let (socket, _) = listener.accept().await.unwrap();
        let mut socket = BufReader::new(socket);
        let mut line = String::new();
        socket.read_line(&mut line).await.unwrap();
        assert_eq!(line.trim_end(), format!("POST /v1internal:{} HTTP/1.1", reply.method));
        let mut headers = std::collections::HashMap::new();
        loop {
            line.clear();
            assert_ne!(socket.read_line(&mut line).await.unwrap(), 0);
            if line == "\r\n" {
                break;
            }
            let (key, value) = line.trim_end().split_once(':').unwrap();
            headers.insert(key.to_ascii_lowercase(), value.trim().to_owned());
        }
        assert_eq!(headers["authorization"], format!("Bearer {}", reply.token));
        assert_eq!(headers["content-type"], "application/json");
        assert!(headers["user-agent"].ends_with(" (antigravity/cli/1.4.6)"));
        let mut body = vec![0; headers["content-length"].parse().unwrap()];
        socket.read_exact(&mut body).await.unwrap();
        let expected = if reply.method == "loadCodeAssist" {
            json!({"metadata":{"ideType":"ANTIGRAVITY"}})
        } else {
            json!({})
        };
        assert_eq!(serde_json::from_slice::<Value>(&body).unwrap(), expected);
        socket.write_all(format!(
            "HTTP/1.1 {}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
            reply.status, reply.body.len(), reply.body,
        ).as_bytes()).await.unwrap();
    }
}

async fn account_query(replies: &[Reply<'_>]) -> (UsageQueryResult, Vec<bool>) {
    let tenant = TenantScope::default();
    let scope = scope(&tenant);
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let mut attempts = Vec::new();
    let query =
        query_account_at(scope, ACCOUNT, &origin, None, NOW, QUERY, |selected, id, refresh| {
            assert!(selected == scope);
            assert_eq!(id, ACCOUNT);
            attempts.push(refresh);
            std::future::ready(Ok(
                if refresh { "fixture-renewed" } else { "fixture-managed" }.to_owned()
            ))
        });
    let (result, ()) =
        timeout(Duration::from_secs(5), async { tokio::join!(query, replay(&listener, replies)) })
            .await
            .unwrap();
    (result.unwrap(), attempts)
}

fn cli_host(path: &std::path::Path, script: &str) -> LocalReadEnvironment {
    let bin = path.join("agy");
    fs::write(&bin, script).unwrap();
    fs::set_permissions(&bin, fs::Permissions::from_mode(0o700)).unwrap();
    LocalReadEnvironment {
        host_id: "fixture-host".into(),
        home: path.into(),
        search_paths: vec![path.into()],
        applications: path.into(),
    }
}

#[tokio::test]
async fn usage_subscriptions_agy_returns_expected_result() {
    let (managed, attempts) = account_query(&[
        Reply {
            method: "retrieveUserQuotaSummary",
            token: "fixture-managed",
            status: "401 Unauthorized",
            body: "{}",
        },
        Reply {
            method: "retrieveUserQuotaSummary",
            token: "fixture-renewed",
            status: "200 OK",
            body: SUMMARY,
        },
    ])
    .await;
    assert_eq!(attempts, [false, true]);

    let home = tempfile::tempdir().unwrap();
    let host = cli_host(
        home.path(),
        r#"#!/bin/sh
[ "$#" = 4 ] && [ "$1" = --print ] && [ "$2" = /usage ] && [ "$3" = --output-format ] &&
[ "$4" = json ] && [ -z "$GEMINI_API_KEY$GOOGLE_GEMINI_BASE_URL" ] && [ "$(pwd -P)" = "$HOME" ] || exit 97
printf '%s' '{"status":"SUCCESS","command":{"data":{"groups":[
  {"buckets":[{"id":"3p-5h","remaining_fraction":0},{"id":"3p-weekly","remaining_fraction":0.25}]},
  {"buckets":[{"id":"gemini-5h","remaining_fraction":0.5,"reset_time":"2030-01-08T00:00:00Z"},
              {"id":"gemini-weekly","remaining_fraction":0.75,"reset_time":"2030-01-08T00:00:00Z"}]}
]}}}'
"#,
    );
    let cli = query_cli_at(&host, NOW, QUERY).await;
    assert_eq!(managed.source, UsageSource::Official("agy_oauth".into()));
    assert_eq!(cli.source, UsageSource::Official("agy".into()));
    assert_eq!(managed.outcome, cli.outcome);
    assert_eq!(managed.queried_at, Some(NOW));
    assert_eq!(cli.queried_at, Some(NOW));
    let UsageQueryOutcome::Success { data: UsageQueryData::OfficialSubscription(data) } =
        managed.outcome
    else {
        panic!("expected official quota")
    };
    assert_eq!(
        data.windows
            .iter()
            .map(|w| (w.name.as_str(), w.utilization, w.resets_at.as_deref()))
            .collect::<Vec<_>>(),
        [
            ("agy_gemini_five_hour", 50.0.try_into().ok(), Some(RESET)),
            ("agy_gemini_weekly", 25.0.try_into().ok(), Some(RESET)),
            ("agy_third_party_five_hour", 100.0.try_into().ok(), None),
            ("agy_third_party_weekly", 75.0.try_into().ok(), None),
        ],
    );
}

#[tokio::test]
async fn usage_subscriptions_agy_rejects_invalid_input() {
    let tenant = TenantScope::default();
    let result = query_account_at(
        scope(&tenant),
        ACCOUNT,
        "http://127.0.0.1:1",
        None,
        NOW,
        QUERY,
        |_, _, refresh| {
            assert!(!refresh);
            std::future::ready(Err(AuthErrorCode::NotFound))
        },
    )
    .await
    .unwrap();
    assert_failure(
        &result,
        UsageCredentialState::NotFound,
        UsageQueryErrorCode::CredentialUnavailable,
    );

    let home = tempfile::tempdir().unwrap();
    let host = cli_host(
        home.path(),
        r#"#!/bin/sh
printf '%s' '{"status":"ERROR","response":"private-upstream-payload"}'
"#,
    );
    let cli = query_cli_at(&host, NOW, QUERY).await;
    assert_failure(&cli, UsageCredentialState::Valid, UsageQueryErrorCode::UpstreamRejected);
}

#[tokio::test]
async fn usage_subscriptions_agy_keeps_ineligibility_distinct_from_expired_login() {
    let (result, attempts) = account_query(&[
        Reply { method: "retrieveUserQuotaSummary", token: "fixture-managed", status: "403 Forbidden", body: "{}" },
        Reply { method: "loadCodeAssist", token: "fixture-managed", status: "200 OK",
            body: r#"{"ineligibleTiers":[{"reasonCode":"RESTRICTED_AGE","reasonMessage":"private-upstream-payload"}]}"# },
    ]).await;
    assert_eq!(attempts, [false]);
    assert_failure(&result, UsageCredentialState::Valid, UsageQueryErrorCode::UpstreamRejected);
    let UsageQueryOutcome::Failure { error, .. } = result.outcome else { unreachable!() };
    assert_eq!(error.message, "Antigravity account is not eligible");
}

#[tokio::test]
async fn usage_subscriptions_agy_rejects_unrecognized_quota() {
    let (result, _) = account_query(&[Reply {
        method: "retrieveUserQuotaSummary", token: "fixture-managed", status: "200 OK",
        body: r#"{"groups":[{"buckets":[{"bucketId":"future-bucket","remainingFraction":0.1},{"bucketId":"gemini-5h"}]}]}"#,
    }]).await;
    assert_failure(&result, UsageCredentialState::Valid, UsageQueryErrorCode::InvalidResponse);
}

fn assert_failure(
    result: &UsageQueryResult,
    state: UsageCredentialState,
    code: UsageQueryErrorCode,
) {
    assert_eq!(result.credential_state, state);
    let UsageQueryOutcome::Failure { error, .. } = &result.outcome else {
        panic!("expected query failure")
    };
    assert_eq!(error.code, code);
    assert_eq!(error.correlation_id, QUERY);
    assert!(!serde_json::to_string(result).unwrap().contains(SECRET_MARKER));
}
