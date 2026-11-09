use super::{minimax_origin, query_at, query_minimax};
use crate::usage::detect::{
    CodingPlanProvider, NativeUsageCapability, ZhipuPlanSelection, detect_native_usage,
};
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
        UsageQueryOutcome, UsageQueryResult, UsageQuotaWindow, UsageSource,
    },
};

const KEY: &str = "fixture-minimax-key";
const NOW: TimestampMs = TimestampMs(1_893_456_000_000);
const CORRELATION_ID: &str = "fixture-query";

async fn replay(status: &str, body: &str) -> UsageQueryResult {
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
        assert_eq!(line, "GET /v1/api/openplatform/coding_plan/remains HTTP/1.1\r\n");
        let mut authorization = None;
        let mut content_type = None;
        loop {
            line.clear();
            assert_ne!(stream.read_line(&mut line).await.unwrap(), 0);
            if line == "\r\n" {
                break;
            }
            let (name, value) = line.split_once(':').unwrap();
            if name.eq_ignore_ascii_case("authorization") {
                authorization = Some(value.trim().to_owned());
            } else if name.eq_ignore_ascii_case("content-type") {
                content_type = Some(value.trim().to_owned());
            }
        }
        assert_eq!(authorization.as_deref(), Some("Bearer fixture-minimax-key"));
        assert_eq!(content_type.as_deref(), Some("application/json"));
        stream.get_mut().write_all(response.as_bytes()).await.unwrap();
    });
    let result =
        query_at(super::client_builder().no_proxy(), Ok(&origin), KEY, NOW, CORRELATION_ID).await;
    timeout(Duration::from_secs(1), server).await.unwrap().unwrap();
    result
}

fn windows(result: UsageQueryResult) -> Vec<UsageQuotaWindow> {
    assert_eq!(result.source, UsageSource::Provider("minimax".into()));
    assert_eq!(result.queried_at, Some(NOW));
    assert_eq!(result.credential_state, UsageCredentialState::Valid);
    let UsageQueryOutcome::Success { data: UsageQueryData::CodingPlan(windows) } = result.outcome
    else {
        panic!("expected a Coding Plan result");
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
    assert!(!error.message.contains(KEY));
}

#[tokio::test]
async fn usage_coding_minimax_returns_expected_result() {
    for (inference_url, quota_origin) in [
        ("https://api.minimax.cn/anthropic", "https://api.minimaxi.com"),
        ("https://api.minimaxi.com/v1", "https://api.minimaxi.com"),
        ("https://api.minimax.io/v1", "https://api.minimax.io"),
    ] {
        let Some(NativeUsageCapability::CodingPlan(provider)) =
            detect_native_usage(inference_url, ZhipuPlanSelection::Auto).unwrap()
        else {
            panic!("expected a Coding Plan provider");
        };
        assert_eq!(minimax_origin(provider), Ok(quota_origin));
    }

    let result = replay(
        "200 OK",
        r#"{
            "base_resp":{"status_code":0},
            "model_remains":[
                {"model_name":"video","current_interval_remaining_percent":50},
                {
                    "model_name":"general",
                    "current_interval_remaining_percent":98,
                    "current_interval_total_count":1500,
                    "current_interval_usage_count":1470,
                    "end_time":1893474000250,
                    "current_weekly_status":2,
                    "current_weekly_remaining_percent":0,
                    "weekly_end_time":1894089600000
                }
            ]
        }"#,
    )
    .await;
    let windows = windows(result);
    assert_eq!(windows.len(), 2);
    let five_hour = &windows[0];
    assert_eq!(five_hour.name, "five_hour");
    assert_eq!(five_hour.utilization.map(f64::from), Some(2.0));
    assert_eq!(five_hour.total.map(f64::from), Some(1500.0));
    assert_eq!(five_hour.used.map(f64::from), Some(30.0));
    assert_eq!(five_hour.remaining.map(f64::from), Some(1470.0));
    assert_eq!(five_hour.unit.as_deref(), Some("requests"));
    assert_eq!(five_hour.resets_at.as_deref(), Some("2030-01-01T05:00:00.250+00:00"));
    let weekly = &windows[1];
    assert_eq!(weekly.name, "weekly_limit");
    assert_eq!(weekly.utilization.map(f64::from), Some(100.0));
    assert_eq!(weekly.total, None);
    assert_eq!(weekly.used, None);
    assert_eq!(weekly.remaining, None);
    assert_eq!(weekly.resets_at.as_deref(), Some("2030-01-08T08:00:00+00:00"));
}

#[tokio::test]
async fn usage_coding_minimax_rejects_invalid_input() {
    use UsageQueryErrorCode::*;

    for key in ["", "bad\r\nkey"] {
        let result = query_minimax(CodingPlanProvider::MiniMaxCn, key, NOW, CORRELATION_ID).await;
        assert_eq!(result.queried_at, None);
        assert_eq!(result.credential_state, UsageCredentialState::NotFound);
        assert_failure(result, CredentialUnavailable);
    }
    assert_failure(
        query_minimax(CodingPlanProvider::ZhipuCn, KEY, NOW, CORRELATION_ID).await,
        Unsupported,
    );
    for body in [
        r#"{}"#,
        r#"{"base_resp":{},"model_remains":[]}"#,
        r#"{"model_remains":[{"model_name":"video","current_interval_remaining_percent":100}]}"#,
        r#"{"model_remains":[{"model_name":"general","end_time":1893474000250}]}"#,
        r#"{"model_remains":[{"model_name":"general","current_interval_remaining_percent":"98"}]}"#,
    ] {
        assert_failure(replay("200 OK", body).await, InvalidResponse);
    }
    assert_failure(
        replay("200 OK", r#"{"base_resp":{"status_code":17,"status_msg":"fixture-minimax-key"}}"#)
            .await,
        UpstreamRejected,
    );
    for status in ["401 Unauthorized", "403 Forbidden"] {
        let result = replay(status, KEY).await;
        assert_eq!(result.credential_state, UsageCredentialState::Expired);
        assert_failure(result, UpstreamUnauthorized);
    }
}

#[tokio::test]
async fn usage_coding_minimax_preserves_partial_quota() {
    for weekly_status in [1, 3] {
        let body = format!(
            r#"{{"model_remains":[{{
                "model_name":"general",
                "current_interval_remaining_percent":-5,
                "current_interval_total_count":0,
                "current_interval_usage_count":0,
                "current_weekly_status":{weekly_status},
                "current_weekly_remaining_percent":{}
            }}]}}"#,
            if weekly_status == 1 { "null" } else { "100" },
        );
        let windows = windows(replay("200 OK", &body).await);
        assert_eq!(windows.len(), 1);
        assert_eq!(windows[0].utilization.map(f64::from), Some(105.0));
        assert_eq!(windows[0].total, None);
        assert_eq!(windows[0].remaining, None);
        assert_eq!(windows[0].used, None);
        assert_eq!(windows[0].resets_at, None);
    }
}
