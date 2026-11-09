use super::{api_root, client_builder, query_at, query_kimi};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::TcpListener,
};
use vw_api_types::{
    TimestampMs,
    agent_apps_auth::AuthRegion,
    apps_usage::{
        UsageCredentialState, UsageQueryData, UsageQueryErrorCode, UsageQueryKind,
        UsageQueryOutcome, UsageQueryResult, UsageSource,
    },
};

const KEY: &str = "fixture-kimi-key";
const NOW: TimestampMs = TimestampMs(1_893_456_000_000);
const CORRELATION_ID: &str = "fixture-query";

async fn replay(status: &str, body: &str, content_length: usize) -> UsageQueryResult {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let root = format!("http://{}/coding/v1", listener.local_addr().unwrap());
    let location = if status == "302 Found" {
        format!("Location: {root}/unexpected\r\n")
    } else {
        String::new()
    };
    let response = format!(
        "HTTP/1.1 {status}\r\n{location}Content-Length: {content_length}\r\nConnection: close\r\n\r\n{body}"
    );
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut stream = BufReader::new(stream);
        let mut line = String::new();
        stream.read_line(&mut line).await.unwrap();
        assert_eq!(line, "GET /coding/v1/usages HTTP/1.1\r\n");
        let mut authorization = None;
        let mut accept = None;
        loop {
            line.clear();
            assert_ne!(stream.read_line(&mut line).await.unwrap(), 0);
            if line == "\r\n" {
                break;
            }
            let (name, value) = line.split_once(':').unwrap();
            if name.eq_ignore_ascii_case("authorization") {
                authorization = Some(value.trim().to_owned());
            } else if name.eq_ignore_ascii_case("accept") {
                accept = Some(value.trim().to_owned());
            }
        }
        assert_eq!(authorization.as_deref(), Some("Bearer fixture-kimi-key"));
        assert_eq!(accept.as_deref(), Some("application/json"));
        stream.get_mut().write_all(response.as_bytes()).await.unwrap();
    });
    let result = query_at(client_builder().no_proxy(), &root, KEY, NOW, CORRELATION_ID).await;
    server.await.unwrap();
    result
}

fn assert_failure(result: UsageQueryResult, code: UsageQueryErrorCode) {
    let UsageQueryOutcome::Failure { kind, error } = result.outcome else {
        panic!("expected a Coding Plan failure");
    };
    assert_eq!(kind, UsageQueryKind::CodingPlan);
    assert_eq!(error.code, code);
    assert_eq!(error.correlation_id, CORRELATION_ID);
    assert!(!error.message.contains(KEY));
}

#[tokio::test]
async fn usage_coding_kimi_returns_expected_result() {
    assert_eq!(api_root(AuthRegion::MainlandCn), "https://api.kimi.com/coding/v1");
    assert_eq!(api_root(AuthRegion::Global), "https://api.kimi.ai/coding/v1");

    let body = r#"{
        "limits":[
            {"detail":{"limit":"100","remaining":"40","resetTime":1893474000}},
            {"detail":{"limit":80,"remaining":90,"resetTime":1893474000250}},
            {}
        ],
        "usage":{"limit":200,"remaining":"25","resetTime":"2030-01-08T08:00:00+08:00"}
    }"#;
    let result = replay("200 OK", body, body.len()).await;
    assert_eq!(result.source, UsageSource::Provider("kimi".into()));
    assert_eq!(result.queried_at, Some(NOW));
    assert_eq!(result.credential_state, UsageCredentialState::Valid);
    let UsageQueryOutcome::Success { data: UsageQueryData::CodingPlan(windows) } = result.outcome
    else {
        panic!("expected a Coding Plan result");
    };
    let actual: Vec<_> = windows
        .iter()
        .map(|window| {
            (
                window.name.as_str(),
                window.utilization.map(f64::from),
                window.used.map(f64::from),
                window.total.map(f64::from),
                window.remaining.map(f64::from),
                window.resets_at.as_deref(),
            )
        })
        .collect();
    assert_eq!(
        actual,
        [
            (
                "five_hour",
                Some(60.0),
                Some(60.0),
                Some(100.0),
                Some(40.0),
                Some("2030-01-01T05:00:00+00:00")
            ),
            (
                "five_hour",
                Some(0.0),
                Some(0.0),
                Some(80.0),
                Some(90.0),
                Some("2030-01-01T05:00:00.250+00:00")
            ),
            (
                "weekly_limit",
                Some(87.5),
                Some(175.0),
                Some(200.0),
                Some(25.0),
                Some("2030-01-08T08:00:00+08:00")
            ),
        ]
    );
}

#[tokio::test]
async fn usage_coding_kimi_rejects_invalid_input() {
    use UsageQueryErrorCode::*;

    for key in ["", "bad\r\nkey"] {
        let result = query_kimi(AuthRegion::MainlandCn, key, NOW, CORRELATION_ID).await;
        assert_eq!(result.queried_at, None);
        assert_failure(result, CredentialUnavailable);
    }

    for body in [
        "{",
        "[]",
        r#"{"limits":{}}"#,
        r#"{"limits":[{"detail":true}]}"#,
        r#"{"usage":{"limit":"NaN"}}"#,
        r#"{"usage":{"resetTime":"fixture-kimi-key"}}"#,
    ] {
        assert_failure(replay("200 OK", body, body.len()).await, InvalidResponse);
    }
}

#[tokio::test]
async fn usage_coding_kimi_distinguishes_empty_plan_and_expired_credentials() {
    use UsageQueryErrorCode::UpstreamUnauthorized;

    for body in ["{}", r#"{"limits":[],"usage":null}"#] {
        let result = replay("200 OK", body, body.len()).await;
        assert_eq!(result.credential_state, UsageCredentialState::Valid);
        assert_eq!(
            result.outcome,
            UsageQueryOutcome::Success { data: UsageQueryData::CodingPlan(Vec::new()) }
        );
    }

    for status in ["401 Unauthorized", "403 Forbidden"] {
        let result = replay(status, KEY, KEY.len()).await;
        assert_eq!(result.credential_state, UsageCredentialState::Expired);
        assert_eq!(result.queried_at, Some(NOW));
        assert_failure(result, UpstreamUnauthorized);
    }
}

#[tokio::test]
async fn usage_coding_kimi_preserves_missing_and_zero_values() {
    let body = r#"{"limits":[{"detail":{"limit":0,"remaining":0,"resetTime":0}}],"usage":{"remaining":"0","resetTime":-1}}"#;
    let result = replay("200 OK", body, body.len()).await;
    let UsageQueryOutcome::Success { data: UsageQueryData::CodingPlan(windows) } = result.outcome
    else {
        panic!("expected a Coding Plan result");
    };
    assert_eq!(windows.len(), 2);
    assert_eq!(windows[0].utilization.map(f64::from), Some(0.0));
    assert_eq!(windows[0].total.map(f64::from), Some(0.0));
    assert_eq!(windows[0].resets_at, None);
    assert_eq!(windows[1].remaining.map(f64::from), Some(0.0));
    assert_eq!(windows[1].total, None);
    assert_eq!(windows[1].used, None);
    assert_eq!(windows[1].utilization, None);
    assert_eq!(windows[1].resets_at, None);
}

#[tokio::test]
async fn usage_coding_kimi_distinguishes_transport_and_upstream_failures() {
    use UsageQueryErrorCode::*;

    assert_failure(replay("200 OK", "{", 64).await, Transport);
    for (status, code) in [
        ("302 Found", UpstreamRejected),
        ("429 Too Many Requests", UpstreamRateLimited),
        ("503 Service Unavailable", UpstreamUnavailable),
    ] {
        assert_failure(replay(status, KEY, KEY.len()).await, code);
    }
}
