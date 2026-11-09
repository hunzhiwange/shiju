use std::time::Duration;

use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::TcpListener,
    time::timeout,
};
use vw_api_types::{
    TimestampMs,
    apps_usage::{
        UsageCredentialState, UsageQueryData, UsageQueryErrorCode, UsageQueryKind,
        UsageQueryOutcome, UsageQueryResult, UsageSource,
    },
};

use super::{client_builder, query_at, query_opencode_go};

const KEY: &str = "fixture-opencode-go-key";
const NOW: TimestampMs = TimestampMs(1_893_456_000_000);
const ID: &str = "fixture-query";
const CURRENT: &str = "https://opencode.ai/inference/go/anthropic";

async fn replay(base_url: &str, status: &str, body: &str) -> UsageQueryResult {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut stream = BufReader::new(stream);
        let mut line = String::new();
        stream.read_line(&mut line).await.unwrap();
        assert_eq!(line, "GET /zen/go/v1/usage HTTP/1.1\r\n");

        let mut authorization = None;
        let mut accept = None;
        loop {
            line.clear();
            assert_ne!(stream.read_line(&mut line).await.unwrap(), 0);
            if line == "\r\n" {
                break;
            }
            let (name, value) = line.split_once(':').unwrap();
            match name.to_ascii_lowercase().as_str() {
                "authorization" => authorization = Some(value.trim().to_owned()),
                "accept" => accept = Some(value.trim().to_owned()),
                "x-api-key" => panic!("the quota endpoint uses Bearer authentication"),
                _ => {}
            }
        }
        assert_eq!(authorization.as_deref(), Some("Bearer fixture-opencode-go-key"));
        assert_eq!(accept.as_deref(), Some("application/json"));
        stream.get_mut().write_all(response.as_bytes()).await.unwrap();
    });
    let result = query_at(client_builder().no_proxy(), &origin, base_url, KEY, NOW, ID).await;
    timeout(Duration::from_secs(1), server).await.unwrap().unwrap();
    result
}

fn assert_failure(result: &UsageQueryResult, expected: UsageQueryErrorCode) {
    let UsageQueryOutcome::Failure { kind, error } = &result.outcome else {
        panic!("expected a Coding Plan failure");
    };
    assert_eq!(*kind, UsageQueryKind::CodingPlan);
    assert_eq!(error.code, expected);
    assert_eq!(error.correlation_id, ID);
    assert!(!error.message.contains(KEY));
}

#[tokio::test]
async fn usage_coding_opencode_go_returns_expected_result() {
    for base_url in [
        CURRENT,
        "https://opencode.ai/inference/go/v1",
        "https://opencode.ai/zen/go",
        "https://opencode.ai/zen/go/v1",
    ] {
        let result = replay(
            base_url,
            "200 OK",
            r#"{"usage":{
                "rolling":{"status":"ok","percent":37,"resetsAt":"2030-01-01T05:00:00.250Z"},
                "weekly":{"status":"ok","percent":"0","resetsAt":"2030-01-08T00:00:00Z"},
                "monthly":{"status":"rate-limited","percent":100,"resetsAt":"2030-02-01T00:00:00Z"}
            }}"#,
        )
        .await;
        assert_eq!(result.source, UsageSource::Provider("opencode_go".into()));
        assert_eq!(result.credential_state, UsageCredentialState::Valid);
        assert_eq!(result.queried_at, Some(NOW));
        let UsageQueryOutcome::Success { data: UsageQueryData::CodingPlan(windows) } =
            result.outcome
        else {
            panic!("expected OpenCode Go quota windows");
        };
        assert_eq!(windows.len(), 3);
        for (window, (name, percent, reset)) in windows.iter().zip([
            ("five_hour", 37.0, Some("2030-01-01T05:00:00.250Z")),
            ("weekly_limit", 0.0, None),
            ("monthly", 100.0, Some("2030-02-01T00:00:00Z")),
        ]) {
            assert_eq!(window.name, name);
            assert_eq!(window.utilization.map(f64::from), Some(percent));
            assert_eq!(window.unit.as_deref(), Some("USD"));
            assert_eq!((window.used, window.total, window.remaining), (None, None, None));
            assert_eq!(window.resets_at.as_deref(), reset);
        }
    }
}

#[tokio::test]
async fn usage_coding_opencode_go_rejects_invalid_input() {
    use UsageQueryErrorCode::*;

    for url in [
        "https://opencode.ai/zen/v1",
        "https://opencode.ai/inference/v1",
        "https://opencode.ai/zen/gopher",
        "https://opencode.ai.example/inference/go",
    ] {
        let result = query_at(client_builder(), "http://127.0.0.1:0", url, KEY, NOW, ID).await;
        assert_failure(&result, Unsupported);
        assert_eq!(result.queried_at, None);
    }
    for key in ["", "bad\r\nkey"] {
        let result = query_opencode_go(CURRENT, key, NOW, ID).await;
        assert_failure(&result, CredentialUnavailable);
        assert_eq!(result.credential_state, UsageCredentialState::NotFound);
        assert_eq!(result.queried_at, None);
    }
}

#[tokio::test]
async fn usage_coding_opencode_go_distinguishes_entitlement_and_authentication() {
    use UsageQueryErrorCode::*;

    for (status, expected, state) in [
        ("401 Unauthorized", UpstreamUnauthorized, UsageCredentialState::Expired),
        ("403 Forbidden", UpstreamRejected, UsageCredentialState::Valid),
        ("429 Too Many Requests", UpstreamRateLimited, UsageCredentialState::Valid),
        ("503 Service Unavailable", UpstreamUnavailable, UsageCredentialState::Valid),
    ] {
        let result = replay(CURRENT, status, KEY).await;
        assert_failure(&result, expected);
        assert_eq!(result.credential_state, state);
        assert_eq!(result.queried_at, Some(NOW));
    }
}

#[tokio::test]
async fn usage_coding_opencode_go_keeps_valid_windows_in_partial_responses() {
    let result = replay(
        CURRENT,
        "200 OK",
        r#"{"usage":{
            "rolling":{"percent":"NaN"},
            "weekly":{"percent":"12","resetsAt":"invalid"},
            "monthly":null
        }}"#,
    )
    .await;
    let UsageQueryOutcome::Success { data: UsageQueryData::CodingPlan(windows) } = result.outcome
    else {
        panic!("expected a valid weekly window");
    };
    assert_eq!(windows.len(), 1);
    assert_eq!(windows[0].name, "weekly_limit");
    assert_eq!(windows[0].utilization.map(f64::from), Some(12.0));
    assert_eq!(windows[0].resets_at, None);

    let obsolete = r#"{"rollingUsage":{"usagePercent":37,"resetInSec":3600}}"#;
    let result = replay(CURRENT, "200 OK", obsolete).await;
    assert_failure(&result, UsageQueryErrorCode::InvalidResponse);
}
