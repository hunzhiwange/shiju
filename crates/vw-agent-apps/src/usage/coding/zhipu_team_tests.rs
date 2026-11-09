use std::collections::BTreeMap;

use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::TcpListener,
    time::{Duration, timeout},
};
use vw_api_types::{
    TimestampMs,
    apps_usage::{
        UsageCredentialState, UsageQueryData, UsageQueryErrorCode, UsageQueryKind,
        UsageQueryOutcome, UsageQueryResult, UsageSource,
    },
};

use super::{ZhipuTeamCredentials, query_at, query_zhipu_team};
use crate::usage::{
    coding::zhipu::client_builder,
    detect::{CodingPlanProvider, NativeUsageCapability, ZhipuPlanSelection, detect_native_usage},
};

const BASE_URL: &str = "https://open.bigmodel.cn/api/coding/paas/v4";
const NOW: TimestampMs = TimestampMs(1_893_456_000_000);
const CORRELATION_ID: &str = "fixture-team-query";
const CREDENTIALS: ZhipuTeamCredentials<'_> = ZhipuTeamCredentials {
    api_key: "fixture-team-key",
    organization_id: Some(" fixture-org "),
    project_id: Some(" fixture-project "),
};

fn detected_provider(selection: ZhipuPlanSelection) -> CodingPlanProvider {
    match detect_native_usage(BASE_URL, selection).unwrap() {
        Some(NativeUsageCapability::CodingPlan(provider)) => provider,
        _ => panic!("expected a Coding Plan provider"),
    }
}

async fn assert_no_request(listener: &TcpListener) {
    assert!(timeout(Duration::from_millis(20), listener.accept()).await.is_err());
}

async fn replay(status: &str, body: &str) -> UsageQueryResult {
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
        assert_eq!(line, "GET /api/monitor/usage/quota/limit?type=2 HTTP/1.1\r\n");

        let mut headers = BTreeMap::new();
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
            ("authorization", "fixture-team-key"),
            ("bigmodel-organization", "fixture-org"),
            ("bigmodel-project", "fixture-project"),
            ("content-type", "application/json"),
            ("accept-language", "en-US,en"),
        ] {
            assert_eq!(headers[name], expected);
        }
        stream.get_mut().write_all(response.as_bytes()).await.unwrap();
        listener
    });

    let result = query_at(
        client_builder().no_proxy(),
        &origin,
        detected_provider(ZhipuPlanSelection::Team),
        CREDENTIALS,
        NOW,
        CORRELATION_ID,
    )
    .await;
    let listener = timeout(Duration::from_secs(2), server).await.unwrap().unwrap();
    assert_no_request(&listener).await;
    result
}

#[tokio::test]
async fn usage_coding_zhipu_team_returns_expected_result() {
    let result = replay(
        "200 OK",
        r#"{"success":true,"data":{"level":"max","limits":[
            {"type":"TOKENS_LIMIT","unit":6,"percentage":5,"nextResetTime":1893459600000},
            {"type":"TOKENS_LIMIT","unit":3,"percentage":26,"nextResetTime":1893474000250}
        ]}}"#,
    )
    .await;

    assert_eq!(result.source, UsageSource::Provider("zhipu_team".into()));
    assert_eq!(result.queried_at, Some(NOW));
    assert_eq!(result.credential_state, UsageCredentialState::Valid);
    let UsageQueryOutcome::Success { data: UsageQueryData::CodingPlan(windows) } = result.outcome
    else {
        panic!("expected team quota windows");
    };
    assert_eq!(windows.len(), 2);
    for (window, (name, utilization, reset)) in windows.iter().zip([
        ("five_hour", 26.0, "2030-01-01T05:00:00.250+00:00"),
        ("weekly_limit", 5.0, "2030-01-01T01:00:00+00:00"),
    ]) {
        assert_eq!(window.name, name);
        assert_eq!(window.plan_label.as_deref(), Some("max"));
        assert_eq!(window.utilization.map(f64::from), Some(utilization));
        assert_eq!(window.resets_at.as_deref(), Some(reset));
    }
}

fn assert_failure(
    result: UsageQueryResult,
    code: UsageQueryErrorCode,
    credential_state: UsageCredentialState,
    queried_at: Option<TimestampMs>,
) {
    assert_eq!(result.source, UsageSource::Provider("zhipu_team".into()));
    assert_eq!(result.queried_at, queried_at);
    assert_eq!(result.credential_state, credential_state);
    let UsageQueryOutcome::Failure { kind: UsageQueryKind::CodingPlan, error } = result.outcome
    else {
        panic!("expected a team quota failure");
    };
    assert_eq!(error.code, code);
    assert_eq!(error.correlation_id, CORRELATION_ID);
    assert!(!error.message.contains("fixture-team-key"));
}

#[tokio::test]
async fn usage_coding_zhipu_team_rejects_invalid_input() {
    use UsageCredentialState::{NotFound, Valid};
    use UsageQueryErrorCode::{CredentialUnavailable, MissingConfiguration, Unsupported};

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let auto = detected_provider(ZhipuPlanSelection::Auto);
    let team = detected_provider(ZhipuPlanSelection::Team);

    let result = query_zhipu_team(auto, CREDENTIALS, NOW, CORRELATION_ID).await;
    assert_failure(result, Unsupported, Valid, None);
    for (provider, credentials, code, state) in [
        (auto, CREDENTIALS, Unsupported, Valid),
        (
            team,
            ZhipuTeamCredentials { organization_id: None, ..CREDENTIALS },
            MissingConfiguration,
            NotFound,
        ),
        (
            team,
            ZhipuTeamCredentials { project_id: None, ..CREDENTIALS },
            MissingConfiguration,
            NotFound,
        ),
        (
            team,
            ZhipuTeamCredentials { organization_id: Some("  "), ..CREDENTIALS },
            MissingConfiguration,
            NotFound,
        ),
        (
            team,
            ZhipuTeamCredentials { project_id: Some("line\r\nbreak"), ..CREDENTIALS },
            MissingConfiguration,
            NotFound,
        ),
        (
            team,
            ZhipuTeamCredentials { api_key: " ", ..CREDENTIALS },
            CredentialUnavailable,
            NotFound,
        ),
    ] {
        let result = query_at(
            client_builder().no_proxy(),
            &origin,
            provider,
            credentials,
            NOW,
            CORRELATION_ID,
        )
        .await;
        assert_failure(result, code, state, None);
    }
    assert_no_request(&listener).await;
}

#[tokio::test]
async fn usage_coding_zhipu_team_keeps_upstream_failures_in_team_scope() {
    use UsageCredentialState::{Expired, Valid};
    use UsageQueryErrorCode::{UpstreamRejected, UpstreamUnauthorized};

    let no_plan = r#"{"code":500,"success":false,"msg":"当前用户不存在coding plan"}"#;
    assert_failure(replay("200 OK", no_plan).await, UpstreamRejected, Valid, Some(NOW));
    assert_failure(
        replay("403 Forbidden", no_plan).await,
        UpstreamUnauthorized,
        Expired,
        Some(NOW),
    );
}
