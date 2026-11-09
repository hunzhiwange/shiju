use std::{fs, path::Path, time::Duration};

use serde_json::{Value, json};
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    net::TcpListener,
    time::timeout,
};
use vw_api_types::{
    TimestampMs,
    apps_usage::{
        UsageCredentialState, UsageQueryData, UsageQueryErrorCode, UsageQueryOutcome,
        UsageQueryResult, UsageSource, UsageSubscription,
    },
};

use super::gemini::{GeminiOAuth, query_at};
use crate::usage::coding::zhipu::client_builder;

const NOW: TimestampMs = TimestampMs(1_893_456_000_000);
const LOAD: &str = "/v1internal:loadCodeAssist";
const QUOTA: &str = "/v1internal:retrieveUserQuota";

struct Exchange {
    path: &'static str,
    bearer: Option<&'static str>,
    status: &'static str,
    response: &'static str,
}

async fn replay(credentials: Option<&str>, exchanges: &[Exchange]) -> UsageQueryResult {
    replay_with_oauth(credentials, exchanges, Some(("fixture-client-id", "fixture-client-secret")))
        .await
}

async fn replay_with_oauth(
    credentials: Option<&str>,
    exchanges: &[Exchange],
    client_credentials: Option<(&str, &str)>,
) -> UsageQueryResult {
    let home = tempfile::tempdir().unwrap();
    let directory = home.path().join(".gemini");
    fs::create_dir(&directory).unwrap();
    if let Some(credentials) = credentials {
        fs::write(directory.join("oauth_creds.json"), credentials).unwrap();
    }
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let server = async {
        for exchange in exchanges {
            let (stream, _) = listener.accept().await.unwrap();
            let mut stream = BufReader::new(stream);
            let mut line = String::new();
            stream.read_line(&mut line).await.unwrap();
            assert_eq!(line, format!("POST {} HTTP/1.1\r\n", exchange.path));

            let mut headers = std::collections::HashMap::new();
            loop {
                line.clear();
                assert_ne!(stream.read_line(&mut line).await.unwrap(), 0);
                if line == "\r\n" {
                    break;
                }
                let (name, value) = line.split_once(':').unwrap();
                headers.insert(name.to_ascii_lowercase(), value.trim().to_owned());
            }
            assert_eq!(
                headers.get("authorization").cloned(),
                exchange.bearer.map(|token| format!("Bearer {token}"))
            );
            let mut body = vec![0; headers["content-length"].parse().unwrap()];
            stream.read_exact(&mut body).await.unwrap();
            if exchange.path == "/token" {
                assert_eq!(headers["content-type"], "application/x-www-form-urlencoded");
                let form: std::collections::HashMap<_, _> = reqwest::Url::parse(&format!(
                    "http://fixture/?{}",
                    String::from_utf8(body).unwrap()
                ))
                .unwrap()
                .query_pairs()
                .into_owned()
                .collect();
                assert_eq!(form["grant_type"], "refresh_token");
                assert_eq!(form["refresh_token"], "fixture-refresh");
                assert_eq!(form["client_id"], "fixture-client-id");
                assert_eq!(form["client_secret"], "fixture-client-secret");
            } else {
                assert_eq!(headers["content-type"], "application/json");
                let body: Value = serde_json::from_slice(&body).unwrap();
                match exchange.path {
                    LOAD => assert_eq!(
                        body,
                        json!({"metadata":{"ideType":"GEMINI_CLI","pluginType":"GEMINI"}})
                    ),
                    QUOTA => assert_eq!(body, json!({"project":"fixture-project"})),
                    _ => unreachable!(),
                }
            }
            let response = format!(
                "HTTP/1.1 {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                exchange.status,
                exchange.response.len(),
                exchange.response,
            );
            stream.get_mut().write_all(response.as_bytes()).await.unwrap();
        }
    };
    let endpoint = format!("{origin}/token");
    let query = query_at(
        home.path(),
        Some(Path::new("/fixture/home-without-keychain-access")),
        client_builder().no_proxy(),
        &origin,
        GeminiOAuth { token_url: &endpoint, client_credentials },
        NOW,
        "fixture-query",
    );
    let ((), result) =
        timeout(Duration::from_secs(3), async { tokio::join!(server, query) }).await.unwrap();
    assert!(timeout(Duration::from_millis(20), listener.accept()).await.is_err());
    result
}

fn subscription(result: UsageQueryResult) -> UsageSubscription {
    assert_eq!(result.source, UsageSource::Official("gemini".into()));
    assert_eq!(result.queried_at, Some(NOW));
    assert_eq!(result.credential_state, UsageCredentialState::Valid);
    let UsageQueryOutcome::Success { data: UsageQueryData::OfficialSubscription(data) } =
        result.outcome
    else {
        panic!("expected Gemini subscription quota");
    };
    data
}

fn assert_failure(
    result: UsageQueryResult,
    state: UsageCredentialState,
    code: UsageQueryErrorCode,
    queried_at: Option<TimestampMs>,
) {
    assert_eq!(result.credential_state, state);
    assert_eq!(result.queried_at, queried_at);
    let UsageQueryOutcome::Failure { error, .. } = result.outcome else {
        panic!("expected Gemini query failure");
    };
    assert_eq!(error.code, code);
    assert_eq!(error.correlation_id, "fixture-query");
    assert!(!error.message.contains("fixture-private"));
}

#[tokio::test]
async fn usage_subscriptions_gemini_returns_expected_result() {
    let result = replay(
        Some(r#"{"access_token":"fixture-old","refresh_token":"fixture-refresh","expiry_date":1893456000000}"#),
        &[
            Exchange {
                path: "/token",
                bearer: None,
                status: "200 OK",
                response: r#"{"access_token":"fixture-current"}"#,
            },
            Exchange {
                path: LOAD,
                bearer: Some("fixture-current"),
                status: "200 OK",
                response: r#"{"cloudaicompanionProject":{"projectId":"fixture-project"}}"#,
            },
            Exchange {
                path: QUOTA,
                bearer: Some("fixture-current"),
                status: "200 OK",
                response: r#"{"buckets":[
                    {"modelId":"gemini-2.5-flash-lite","remainingFraction":1.0,"resetTime":"2030-01-02T00:00:00Z"},
                    {"modelId":"gemini-2.5-pro","remainingFraction":0.75,"resetTime":"2030-01-03T00:00:00Z"},
                    {"modelId":"gemini-3-pro","remainingFraction":0,"resetTime":"2030-01-04T00:00:00Z"},
                    {"modelId":"gemini-2.5-flash","remainingFraction":0.5}
                ]}"#,
            },
        ],
    )
    .await;
    let data = subscription(result);
    let windows: Vec<_> = data
        .windows
        .iter()
        .map(|window| {
            (window.name.as_str(), window.utilization.map(f64::from), window.resets_at.as_deref())
        })
        .collect();
    assert_eq!(
        windows,
        [
            ("gemini_pro", Some(100.0), Some("2030-01-04T00:00:00Z")),
            ("gemini_flash", Some(50.0), None),
            ("gemini_flash_lite", Some(0.0), Some("2030-01-02T00:00:00Z")),
        ]
    );
}

#[tokio::test]
async fn usage_subscriptions_gemini_rejects_invalid_input() {
    use UsageCredentialState::{NotFound, ParseError, Valid};
    use UsageQueryErrorCode::{CredentialUnavailable, InvalidResponse};

    assert_failure(replay(None, &[]).await, NotFound, CredentialUnavailable, None);
    assert_failure(
        replay(Some(r#"{"access_token":42}"#), &[]).await,
        ParseError,
        CredentialUnavailable,
        None,
    );
    let result = replay(
        Some(r#"{"access_token":"fixture-current"}"#),
        &[
            Exchange {
                path: LOAD,
                bearer: Some("fixture-current"),
                status: "200 OK",
                response: r#"{"cloudaicompanionProject":"fixture-project"}"#,
            },
            Exchange {
                path: QUOTA,
                bearer: Some("fixture-current"),
                status: "200 OK",
                response: r#"{"error":"fixture-private"}"#,
            },
        ],
    )
    .await;
    assert_failure(result, Valid, InvalidResponse, Some(NOW));
}

#[tokio::test]
async fn usage_subscriptions_gemini_preserves_unknown_usage_and_reset() {
    let result = replay_with_oauth(
        Some(r#"{"token":{"accessToken":"fixture-current","expiresAt":1893456000001}}"#),
        &[
            Exchange {
                path: LOAD,
                bearer: Some("fixture-current"),
                status: "200 OK",
                response: r#"{"cloudaicompanionProject":{"id":"fixture-project"}}"#,
            },
            Exchange {
                path: QUOTA,
                bearer: Some("fixture-current"),
                status: "200 OK",
                response: r#"{"buckets":[
                    {"modelId":"gemini-2.5-pro","remainingFraction":0.75,"resetTime":"2030-01-03T00:00:00Z"},
                    {"modelId":"gemini-3-pro","remainingFraction":0.5},
                    {"modelId":"gemini-2.5-flash","resetTime":"2030-01-02T00:00:00Z"}
                ]}"#,
            },
        ],
        None,
    )
    .await;
    let data = subscription(result);
    assert_eq!(data.windows[0].utilization.map(f64::from), Some(50.0));
    assert_eq!(data.windows[0].resets_at, None);
    assert_eq!(data.windows[1].utilization, None);
    assert_eq!(data.windows[1].resets_at.as_deref(), Some("2030-01-02T00:00:00Z"));
}

#[tokio::test]
async fn usage_subscriptions_gemini_reports_refresh_failure_without_using_stale_token() {
    let result = replay(
        Some(r#"{"access_token":"fixture-old","refresh_token":"fixture-refresh","expiry_date":1893456000000}"#),
        &[Exchange {
            path: "/token",
            bearer: None,
            status: "503 Service Unavailable",
            response: r#"{"error":"fixture-private"}"#,
        }],
    )
    .await;
    assert_failure(
        result,
        UsageCredentialState::RefreshPending,
        UsageQueryErrorCode::UpstreamUnavailable,
        None,
    );
    assert_failure(
        replay(Some(r#"{"access_token":"fixture-old","expiry_date":1893456000000}"#), &[]).await,
        UsageCredentialState::Expired,
        UsageQueryErrorCode::CredentialUnavailable,
        None,
    );
}

#[tokio::test]
async fn usage_subscriptions_gemini_reports_missing_refresh_configuration() {
    let result = replay_with_oauth(
        Some(r#"{"access_token":"fixture-old","refresh_token":"fixture-refresh","expiry_date":1893456000000}"#),
        &[],
        None,
    )
    .await;
    assert_failure(
        result,
        UsageCredentialState::RefreshPending,
        UsageQueryErrorCode::MissingConfiguration,
        None,
    );
}

#[tokio::test]
async fn usage_subscriptions_gemini_reports_upstream_auth_rejection() {
    let result = replay(
        Some(r#"{"access_token":"fixture-current"}"#),
        &[Exchange {
            path: LOAD,
            bearer: Some("fixture-current"),
            status: "403 Forbidden",
            response: r#"{"error":"fixture-private"}"#,
        }],
    )
    .await;
    assert_failure(
        result,
        UsageCredentialState::Expired,
        UsageQueryErrorCode::UpstreamUnauthorized,
        Some(NOW),
    );
}
