use std::{cell::Cell, time::Duration};

use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::TcpListener,
    time::timeout,
};
use vw_api_types::{
    TimestampMs,
    apps_usage::{
        UsageCredentialState, UsageQueryData, UsageQueryErrorCode, UsageQueryKind,
        UsageQueryOutcome, UsageQueryResult, UsageQuotaWindow, UsageSource,
    },
};

use super::volcengine::{query_at, query_volcengine};
use super::zhipu::client_builder;

const BASE_URL: &str = "https://ark.cn-shanghai.volces.com/api/coding/v3";
const ACCESS_KEY: &str = "fixture-access-key";
const SECRET_KEY: &str = "fixture-secret-key";
const NOW: TimestampMs = TimestampMs(1_893_456_000_000);
const CORRELATION_ID: &str = "fixture-query";
const AGENT: &str = "GetAFPUsage";
const CODING: &str = "GetCodingPlanUsage";

async fn replay(responses: &'static [(&str, &str, &str)]) -> UsageQueryResult {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        for (action, status, body) in responses {
            let (stream, _) = listener.accept().await.unwrap();
            let mut stream = BufReader::new(stream);
            let mut line = String::new();
            stream.read_line(&mut line).await.unwrap();
            assert_eq!(
                line,
                format!(
                    "POST /?Action={action}&Region=cn-shanghai&Version=2024-01-01 HTTP/1.1\r\n"
                )
            );
            let mut headers = reqwest::header::HeaderMap::new();
            loop {
                line.clear();
                assert_ne!(stream.read_line(&mut line).await.unwrap(), 0);
                if line == "\r\n" {
                    break;
                }
                let (name, value) = line.split_once(':').unwrap();
                headers.insert(
                    reqwest::header::HeaderName::from_bytes(name.as_bytes()).unwrap(),
                    value.trim().parse().unwrap(),
                );
            }
            assert_eq!(headers["host"], "open.volcengineapi.com");
            assert_eq!(headers["content-type"], "application/json; charset=utf-8");
            assert_eq!(headers["x-date"], "20300101T000000Z");
            assert_eq!(
                headers["x-content-sha256"],
                "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
            );
            let authorization = headers["authorization"].to_str().unwrap();
            assert!(authorization.starts_with(
                "HMAC-SHA256 Credential=fixture-access-key/20300101/cn-shanghai/ark/request, SignedHeaders=host;x-date;x-content-sha256;content-type, Signature="
            ));
            assert!(!authorization.contains(SECRET_KEY));
            assert!(headers.get("content-length").is_none_or(|length| length == "0"));
            let length = if *body == "truncated" { 100 } else { body.len() };
            let response = format!(
                "HTTP/1.1 {status}\r\nContent-Length: {length}\r\nConnection: close\r\n\r\n{body}"
            );
            stream.get_mut().write_all(response.as_bytes()).await.unwrap();
        }
    });

    let client = client_builder().no_proxy().build().unwrap();
    let attempts = Cell::new(0);
    let query =
        query_at(BASE_URL, ACCESS_KEY, SECRET_KEY, NOW, CORRELATION_ID, async |mut request| {
            attempts.set(attempts.get() + 1);
            assert_eq!(request.url().scheme(), "https");
            assert_eq!(request.url().host_str(), Some("open.volcengineapi.com"));
            assert!(request.headers()["authorization"].is_sensitive());
            let url = request.url_mut();
            url.set_scheme("http").unwrap();
            url.set_ip_host(address.ip()).unwrap();
            url.set_port(Some(address.port())).unwrap();
            client.execute(request).await
        });
    let result = timeout(Duration::from_secs(5), query).await.unwrap();
    assert_eq!(attempts.get(), responses.len());
    timeout(Duration::from_secs(1), server).await.unwrap().unwrap();
    result
}

fn windows(result: UsageQueryResult) -> Vec<UsageQuotaWindow> {
    assert_eq!(result.source, UsageSource::Provider("volcengine".into()));
    assert_eq!(result.queried_at, Some(NOW));
    assert_eq!(result.credential_state, UsageCredentialState::Valid);
    let UsageQueryOutcome::Success { data: UsageQueryData::CodingPlan(windows) } = result.outcome
    else {
        panic!("expected quota windows, got {:?}", result.outcome);
    };
    windows
}

fn assert_failure(result: UsageQueryResult, code: UsageQueryErrorCode) {
    let UsageQueryOutcome::Failure { kind, error } = result.outcome else {
        panic!("expected a Coding Plan failure");
    };
    assert_eq!(kind, UsageQueryKind::CodingPlan);
    assert_eq!(error.code, code);
    assert_eq!(error.correlation_id, CORRELATION_ID);
    let public = format!("{error:?} {:?}", result.credential_message);
    for private in [ACCESS_KEY, SECRET_KEY, "fixture-private-body"] {
        assert!(!public.contains(private));
    }
}

#[tokio::test]
async fn usage_coding_volcengine_returns_expected_result() {
    let windows = windows(
        replay(&[(
            AGENT,
            "200 OK",
            r#"{"Result":{"PlanType":"Large",
            "AFPFiveHour":{"Quota":"50","Used":"12.5","ResetTime":1893456000250},
            "AFPDaily":{"Quota":100,"Used":25},
            "AFPWeekly":{"Quota":0,"Used":0},
            "AFPMonthly":{"Quota":100,"Used":125}}}"#,
        )])
        .await,
    );
    assert_eq!(windows.len(), 2);
    let first = &windows[0];
    assert_eq!(first.name, "five_hour");
    assert_eq!(first.plan_label.as_deref(), Some("Agent Plan Large"));
    assert_eq!(first.utilization.map(f64::from), Some(25.0));
    assert_eq!(first.used.map(f64::from), Some(12.5));
    assert_eq!(first.total.map(f64::from), Some(50.0));
    assert_eq!(first.remaining.map(f64::from), Some(37.5));
    assert_eq!(first.unit.as_deref(), Some("AFP"));
    assert_eq!(first.resets_at.as_deref(), Some("2030-01-01T00:00:00.250Z"));
    assert_eq!(windows[1].name, "monthly");
    assert_eq!(windows[1].utilization.map(f64::from), Some(125.0));
}

#[tokio::test]
async fn usage_coding_volcengine_rejects_invalid_input() {
    use UsageQueryErrorCode::*;

    for (key, secret) in [("", SECRET_KEY), (ACCESS_KEY, ""), ("ak\ninvalid", SECRET_KEY)] {
        let result = query_volcengine(BASE_URL, key, secret, NOW, CORRELATION_ID).await;
        assert_eq!(result.queried_at, None);
        assert_eq!(result.credential_state, UsageCredentialState::NotFound);
        assert_failure(result, CredentialUnavailable);
    }
    for result in [
        replay(&[(AGENT, "400 Bad Request", r#"{"ResponseMetadata":{"Error":{"Code":"InvalidAuthorization","Message":"fixture-secret-key"}}}"#)]).await,
        replay(&[(AGENT, "200 OK", r#"{"Error":{"Code":"SignatureDoesNotMatch","Message":"fixture-private-body"}}"#)]).await,
        replay(&[(AGENT, "403 Forbidden", "fixture-private-body")]).await,
    ] {
        assert_eq!(result.queried_at, Some(NOW));
        assert_eq!(result.credential_state, UsageCredentialState::Expired);
        assert_failure(result, UpstreamUnauthorized);
    }
}

#[tokio::test]
async fn usage_coding_volcengine_falls_back_and_preserves_window_times() {
    const QUOTA: &str = r#"{"Result":{"QuotaUsage":[
        {"Level":"session","Percent":0,"ResetTimestamp":-1},
        {"Level":"weekly","Percent":"1.672568","ResetTimestamp":1893456000},
        {"Level":"monthly","Percent":0.5,"ResetTimestamp":"1893456000250"},
        {"Level":"daily","Percent":30}]}}"#;
    for responses in [
        &[
            (AGENT, "200 OK", r#"{"Result":{"AFPFiveHour":{"Quota":0}}}"#),
            (CODING, "200 OK", QUOTA),
        ][..],
        &[
            (
                AGENT,
                "400 Bad Request",
                r#"{"Error":{"Code":"InvalidParameter.Action","Message":"Unauthorized fixture-private-body"}}"#,
            ),
            (CODING, "200 OK", QUOTA),
        ],
    ] {
        let windows = windows(replay(responses).await);
        assert_eq!(windows.len(), 3);
        assert_eq!(windows[0].name, "five_hour");
        assert_eq!(windows[0].plan_label.as_deref(), Some("Coding Plan"));
        assert_eq!(windows[0].utilization.map(f64::from), Some(0.0));
        assert_eq!(windows[0].resets_at, None);
        assert_eq!(windows[1].name, "weekly_limit");
        assert_eq!(windows[1].utilization.map(f64::from), Some(1.672568));
        assert_eq!(windows[1].resets_at.as_deref(), Some("2030-01-01T00:00:00Z"));
        assert_eq!(windows[1].total, None);
        assert_eq!(windows[2].resets_at.as_deref(), Some("2030-01-01T00:00:00.250Z"));
    }
}

#[tokio::test]
async fn usage_coding_volcengine_distinguishes_empty_plans_from_transport_failure() {
    use UsageQueryErrorCode::*;

    let empty = replay(&[
        (AGENT, "200 OK", r#"{"Result":{}}"#),
        (CODING, "200 OK", r#"{"Result":{"QuotaUsage":[]}}"#),
    ])
    .await;
    assert_eq!(empty.credential_state, UsageCredentialState::Valid);
    assert_failure(empty, UpstreamRejected);
    assert_failure(replay(&[(AGENT, "200 OK", "truncated")]).await, Transport);
}
