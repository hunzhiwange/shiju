use super::{query_at, query_zenmux};
use crate::usage::coding::zhipu::client_builder;
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

const KEY: &str = "fixture-zenmux-key";
const NOW: TimestampMs = TimestampMs(1_893_456_000_000);
const CORRELATION_ID: &str = "fixture-query";

async fn replay(status: &str, body: &str) -> UsageQueryResult {
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    replay_response(response).await
}

async fn replay_response(response: String) -> UsageQueryResult {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}/custom/usage?period=current", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut stream = BufReader::new(stream);
        let mut line = String::new();
        stream.read_line(&mut line).await.unwrap();
        assert_eq!(line, "GET /custom/usage?period=current HTTP/1.1\r\n");
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
        assert_eq!(authorization.as_deref(), Some("Bearer fixture-zenmux-key"));
        assert_eq!(accept.as_deref(), Some("application/json"));
        stream.get_mut().write_all(response.as_bytes()).await.unwrap();
    });
    let result = query_at(
        client_builder().no_proxy(),
        Ok(endpoint.parse().unwrap()),
        KEY,
        NOW,
        CORRELATION_ID,
    )
    .await;
    timeout(Duration::from_secs(1), server).await.unwrap().unwrap();
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
async fn usage_coding_zenmux_returns_expected_result() {
    let result = replay(
        "200 OK",
        r#"{
            "success":true,
            "data":{
                "plan":{"tier":"pro"},
                "account_status":"active",
                "quota_5_hour":{
                    "usage_percentage":"0.25",
                    "resets_at":"2030-01-01T05:00:00.250+08:00",
                    "used_value_usd":"2.5",
                    "max_value_usd":10
                },
                "quota_7_day":{
                    "usage_percentage":1.125,
                    "resets_at":"2030-01-08T00:00:00Z"
                }
            }
        }"#,
    )
    .await;
    assert_eq!(result.source, UsageSource::Provider("zenmux".into()));
    assert_eq!(result.queried_at, Some(NOW));
    assert_eq!(result.credential_state, UsageCredentialState::Valid);
    assert_eq!(result.credential_message.as_deref(), Some("pro (active)"));
    let UsageQueryOutcome::Success { data: UsageQueryData::CodingPlan(windows) } = result.outcome
    else {
        panic!("expected a Coding Plan result");
    };
    assert_eq!(windows.len(), 2);
    let five_hour = &windows[0];
    assert_eq!(five_hour.name, "five_hour");
    assert_eq!(five_hour.plan_label.as_deref(), Some("ZenMux·PRO"));
    assert_eq!(five_hour.utilization.map(f64::from), Some(25.0));
    assert_eq!(five_hour.used.map(f64::from), Some(2.5));
    assert_eq!(five_hour.total.map(f64::from), Some(10.0));
    assert_eq!(five_hour.remaining.map(f64::from), Some(7.5));
    assert_eq!(five_hour.unit.as_deref(), Some("USD"));
    assert_eq!(five_hour.resets_at.as_deref(), Some("2030-01-01T05:00:00.250+08:00"));
    let weekly = &windows[1];
    assert_eq!(weekly.name, "weekly_limit");
    assert_eq!(weekly.utilization.map(f64::from), Some(112.5));
    assert_eq!(weekly.used, None);
    assert_eq!(weekly.unit, None);
    assert_eq!(weekly.resets_at.as_deref(), Some("2030-01-08T00:00:00Z"));
}

#[tokio::test]
async fn usage_coding_zenmux_rejects_invalid_input() {
    use UsageQueryErrorCode::*;

    for endpoint in [
        "",
        "http://zenmux.ai/usage",
        "https://zenmux.ai.example/usage",
        "https://127.0.0.1/usage",
        "https://user@zenmux.ai/usage",
        "https://zenmux.ai:8443/usage",
        "https://zenmux.ai/usage#fragment",
    ] {
        let result = query_zenmux(endpoint, KEY, NOW, CORRELATION_ID).await;
        assert_eq!(result.queried_at, None);
        assert_failure(result, MissingConfiguration);
    }
    for key in ["", "bad\r\nkey"] {
        let result = query_zenmux("https://zenmux.ai/usage", key, NOW, CORRELATION_ID).await;
        assert_eq!(result.queried_at, None);
        assert_eq!(result.credential_state, UsageCredentialState::NotFound);
        assert_failure(result, CredentialUnavailable);
    }
    for body in [
        "not json",
        r#"{"success":"true"}"#,
        r#"{"success":true}"#,
        r#"{"success":true,"data":{"unknown_window":{"usage_percentage":0.25}}}"#,
        r#"{"success":true,"data":{"quota_5_hour":{}}}"#,
        r#"{"success":true,"data":{"quota_5_hour":{"usage_percentage":"NaN"}}}"#,
        r#"{"success":true,"data":{"quota_7_day":{"usage_percentage":1e308}}}"#,
        r#"{"success":true,"data":{"quota_7_day":{"resets_at":"invalid"}}}"#,
    ] {
        assert_failure(replay("200 OK", body).await, InvalidResponse);
    }
    assert_failure(
        replay("200 OK", r#"{"success":false,"message":"fixture-zenmux-key"}"#).await,
        UpstreamRejected,
    );
}

#[tokio::test]
async fn usage_coding_zenmux_classifies_http_and_transport_failures() {
    use UsageQueryErrorCode::*;

    for status in ["401 Unauthorized", "403 Forbidden"] {
        let result = replay(status, KEY).await;
        assert_eq!(result.credential_state, UsageCredentialState::Expired);
        assert_failure(result, UpstreamUnauthorized);
    }
    for (status, code) in [
        ("429 Too Many Requests", UpstreamRateLimited),
        ("503 Service Unavailable", UpstreamUnavailable),
    ] {
        assert_failure(replay(status, KEY).await, code);
    }
    let redirect =
        "HTTP/1.1 302 Found\r\nLocation: /other\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
    assert_failure(replay_response(redirect.into()).await, UpstreamRejected);
    let truncated = "HTTP/1.1 200 OK\r\nContent-Length: 100\r\nConnection: close\r\n\r\n{";
    assert_failure(replay_response(truncated.into()).await, Transport);
}
