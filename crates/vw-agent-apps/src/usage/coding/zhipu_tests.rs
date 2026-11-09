use super::{client_builder, personal_origin, query_at, query_zhipu};
use crate::usage::detect::CodingPlanProvider::{ZhipuCn, ZhipuGlobal, ZhipuTeam};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::TcpListener,
    time::{Duration, timeout},
};
use vw_api_types::{
    TimestampMs,
    apps_usage::{
        UsageCredentialState, UsageQueryData, UsageQueryErrorCode, UsageQueryKind,
        UsageQueryOutcome, UsageQueryResult, UsageQuotaWindow, UsageSource,
    },
};

const KEY: &str = "fixture-zhipu-key";
const NOW: TimestampMs = TimestampMs(1_893_456_000_000);
const CORRELATION_ID: &str = "fixture-query";
const NO_PLAN: &str = r#"{"code":500,"success":false,"msg":"当前用户不存在coding plan"}"#;

async fn replay(status: &str, body: &str) -> Option<UsageQueryResult> {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len(),
    );
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut stream = BufReader::new(stream);
        let mut line = String::new();
        stream.read_line(&mut line).await.unwrap();
        assert_eq!(line, "GET /api/monitor/usage/quota/limit HTTP/1.1\r\n");
        let mut headers = std::collections::BTreeMap::new();
        loop {
            line.clear();
            assert_ne!(stream.read_line(&mut line).await.unwrap(), 0);
            if line == "\r\n" {
                break;
            }
            let (name, value) = line.split_once(':').unwrap();
            headers.insert(name.to_ascii_lowercase(), value.trim().to_owned());
        }
        for (name, expected) in [
            ("authorization", KEY),
            ("content-type", "application/json"),
            ("accept-language", "en-US,en"),
        ] {
            assert_eq!(headers[name], expected);
        }
        stream.get_mut().write_all(response.as_bytes()).await.unwrap();
    });
    let result = query_at(client_builder().no_proxy(), Ok(&origin), KEY, NOW, CORRELATION_ID).await;
    timeout(Duration::from_secs(2), server).await.unwrap().unwrap();
    result
}

fn windows(result: UsageQueryResult) -> Vec<UsageQuotaWindow> {
    assert_eq!(result.source, UsageSource::Provider("zhipu".into()));
    assert_eq!(result.credential_state, UsageCredentialState::Valid);
    assert_eq!(result.queried_at, Some(NOW));
    let UsageQueryOutcome::Success { data: UsageQueryData::CodingPlan(windows) } = result.outcome
    else {
        panic!("expected Coding Plan windows");
    };
    windows
}

#[tokio::test]
async fn usage_coding_zhipu_returns_expected_result() {
    assert_eq!(personal_origin(ZhipuCn), Ok("https://open.bigmodel.cn"));
    assert_eq!(personal_origin(ZhipuGlobal), Ok("https://api.z.ai"));

    for weekly_number in [7, 1] {
        let body = serde_json::json!({
            "success": true,
            "data": {
                "level": "pro",
                "limits": [
                    {"type":"CREDIT_LIMIT","unit":6,"number":weekly_number,"percentage":42,"nextResetTime":1893459600000_i64},
                    {"type":"tokens_limit","unit":3,"number":5,"percentage":1,"nextResetTime":1893474000250_i64},
                    {"type":"TIME_LIMIT","percentage":99}
                ]
            }
        });
        let windows = windows(replay("200 OK", &body.to_string()).await.unwrap());
        assert_eq!(windows.len(), 2);
        for (window, (name, percent, reset)) in windows.iter().zip([
            ("five_hour", 1.0, "2030-01-01T05:00:00.250+00:00"),
            ("weekly_limit", 42.0, "2030-01-01T01:00:00+00:00"),
        ]) {
            assert_eq!(window.name, name);
            assert_eq!(window.plan_label.as_deref(), Some("pro"));
            assert_eq!(window.utilization.map(f64::from), Some(percent));
            assert_eq!(window.resets_at.as_deref(), Some(reset));
        }
    }
}

#[tokio::test]
async fn usage_coding_zhipu_classifies_legacy_windows() {
    for (limits, expected) in [
        (r#"[{"type":"TOKENS_LIMIT","percentage":12}]"#, vec![("five_hour", 12.0)]),
        (
            r#"[{"type":"TOKENS_LIMIT","percentage":150,"nextResetTime":1893459600000},
                {"type":"TOKENS_LIMIT","percentage":0}]"#,
            vec![("five_hour", 0.0), ("weekly_limit", 150.0)],
        ),
        (
            r#"[{"type":"TOKENS_LIMIT","unit":6,"percentage":25},
                {"type":"TOKENS_LIMIT","percentage":-5,"nextResetTime":1893459600000}]"#,
            vec![("five_hour", -5.0), ("weekly_limit", 25.0)],
        ),
        (
            r#"[{"type":"TOKENS_LIMIT","unit":9,"percentage":25,"nextResetTime":1893474000000},
                {"type":"TOKENS_LIMIT","percentage":10,"nextResetTime":1893459600000}]"#,
            vec![("five_hour", 10.0), ("weekly_limit", 25.0)],
        ),
    ] {
        let body = format!(r#"{{"success":true,"data":{{"limits":{limits}}}}}"#);
        let windows = windows(replay("200 OK", &body).await.unwrap());
        let actual: Vec<_> = windows
            .iter()
            .map(|window| (window.name.as_str(), f64::from(window.utilization.unwrap())))
            .collect();
        assert_eq!(actual, expected);
    }
}

#[tokio::test]
async fn usage_coding_zhipu_distinguishes_absence_from_empty_plan() {
    for body in [
        NO_PLAN,
        r#"{"code":500,"success":false,"msg":"The current user does not have a coding plan"}"#,
    ] {
        assert!(replay("200 OK", body).await.is_none());
    }
    let empty = r#"{"success":true,"data":{"limits":[{"type":"TIME_LIMIT"}]}}"#;
    assert!(windows(replay("200 OK", empty).await.unwrap()).is_empty());

    let unreported = r#"{"success":true,"data":{"limits":[{"type":"TOKENS_LIMIT"}]}}"#;
    let windows = windows(replay("200 OK", unreported).await.unwrap());
    assert_eq!(windows.len(), 1);
    assert_eq!(windows[0].utilization, None);
    assert_eq!(windows[0].resets_at, None);
}

fn assert_failure(
    result: UsageQueryResult,
    expected_code: UsageQueryErrorCode,
    expected_state: UsageCredentialState,
    queried_at: Option<TimestampMs>,
) {
    let UsageQueryOutcome::Failure { kind, error } = result.outcome else {
        panic!("expected a typed Coding Plan failure");
    };
    assert_eq!(kind, UsageQueryKind::CodingPlan);
    assert_eq!(error.code, expected_code);
    assert_eq!(error.correlation_id, CORRELATION_ID);
    assert!(!error.message.contains(KEY));
    assert_eq!(result.credential_state, expected_state);
    assert_eq!(result.queried_at, queried_at);
}

#[tokio::test]
async fn usage_coding_zhipu_rejects_invalid_input() {
    use UsageCredentialState::{Expired, NotFound, Valid};
    use UsageQueryErrorCode::*;

    for (provider, key, code, state) in [
        (ZhipuTeam, KEY, Unsupported, Valid),
        (ZhipuCn, "bad\r\nkey", CredentialUnavailable, NotFound),
        (ZhipuGlobal, "", CredentialUnavailable, NotFound),
    ] {
        let result = query_zhipu(provider, key, NOW, CORRELATION_ID).await.unwrap();
        assert_failure(result, code, state, None);
    }

    for (status, body, code, state) in [
        ("401 Unauthorized", NO_PLAN, UpstreamUnauthorized, Expired),
        ("429 Too Many Requests", NO_PLAN, UpstreamRateLimited, Valid),
        ("500 Internal Server Error", NO_PLAN, UpstreamUnavailable, Valid),
        (
            "200 OK",
            r#"{"code":500,"success":false,"msg":"no coding plan response due to fixture-zhipu-key"}"#,
            UpstreamRejected,
            Valid,
        ),
        ("200 OK", r#"{"success":true,"data":{"limits":{}}}"#, InvalidResponse, Valid),
        (
            "200 OK",
            r#"{"success":true,"data":{"limits":[{"type":"TOKENS_LIMIT","percentage":"invalid"}]}}"#,
            InvalidResponse,
            Valid,
        ),
    ] {
        let result = replay(status, body).await.unwrap();
        assert_failure(result, code, state, Some(NOW));
    }
}
