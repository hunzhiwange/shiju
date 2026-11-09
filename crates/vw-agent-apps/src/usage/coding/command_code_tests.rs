use std::time::Duration;

use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::TcpListener,
    time::timeout,
};
use vw_api_types::{
    TimestampMs,
    apps_usage::{UsageCredentialState, UsageQueryData, UsageQueryErrorCode, UsageQueryOutcome},
};

use super::{CommandCodeUsage, client_builder, query_at, query_command_code};

const CLAUDE: &str = "https://api.commandcode.ai/provider";
const CODEX: &str = "https://api.commandcode.ai/provider/v1";
const KEY: &str = "fixture-command-code-key";
const NOW: TimestampMs = TimestampMs(1_893_456_000_000);
const ID: &str = "fixture-query";
const WHOAMI: (&str, &str, &str) =
    ("/alpha/whoami?limits=1", "200 OK", r#"{"org":{"id":"org-a & b"}}"#);
const CREDITS: (&str, &str, &str) = (
    "/alpha/billing/credits?orgId=org-a+%26+b",
    "200 OK",
    r#"{"credits":{"monthlyCredits":12.5,"purchasedCredits":"3","freeCredits":0.5},
        "windowLimits":{"limited":true,
            "fiveHour":{"used":"4","cap":16,"resetAt":1893474000},
            "weekly":{"used":12,"cap":40,"resetAt":0}}}"#,
);
const SUBSCRIPTION: (&str, &str, &str) = (
    "/alpha/billing/subscriptions?orgId=org-a+%26+b",
    "200 OK",
    r#"{"data":{"planId":"individual-goat","currentPeriodStart":"2030-01-01T00:00:00Z",
        "currentPeriodEnd":"2030-02-01T00:00:00Z"}}"#,
);
const SUMMARY_PATH: &str = "/alpha/usage/summary?since=2030-01-01T00%3A00%3A00Z&orgId=org-a+%26+b";

async fn replay(base: &str, exchanges: &[(&str, &str, &str)]) -> CommandCodeUsage {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let server = async {
        for &(path, status, body) in exchanges {
            let (stream, _) = listener.accept().await.unwrap();
            let mut stream = BufReader::new(stream);
            let mut line = String::new();
            stream.read_line(&mut line).await.unwrap();
            assert_eq!(line, format!("GET {path} HTTP/1.1\r\n"));

            let mut bearer = None;
            let mut accept = None;
            loop {
                line.clear();
                assert_ne!(stream.read_line(&mut line).await.unwrap(), 0);
                if line == "\r\n" {
                    break;
                }
                let (name, value) = line.split_once(':').unwrap();
                match name.to_ascii_lowercase().as_str() {
                    "authorization" => bearer = Some(value.trim().to_owned()),
                    "accept" => accept = Some(value.trim().to_owned()),
                    _ => {}
                }
            }
            assert_eq!(bearer, Some(format!("Bearer {KEY}")));
            assert_eq!(accept.as_deref(), Some("application/json"));
            let response = format!(
                "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            stream.get_mut().write_all(response.as_bytes()).await.unwrap();
        }
    };
    let query = query_at(client_builder().no_proxy(), &origin, base, KEY, NOW, ID);
    let ((), result) =
        timeout(Duration::from_secs(3), async { tokio::join!(server, query) }).await.unwrap();
    result
}

#[tokio::test]
async fn usage_coding_command_code_returns_expected_result() {
    for base in [CLAUDE, CODEX] {
        let result = replay(
            base,
            &[WHOAMI, CREDITS, SUBSCRIPTION, (SUMMARY_PATH, "200 OK", r#"{"totalCost":8}"#)],
        )
        .await;
        assert_eq!(result.balance.credential_state, UsageCredentialState::Valid);
        assert_eq!(result.coding_plan.queried_at, Some(NOW));
        assert!(result.window_errors.is_empty());
        let UsageQueryOutcome::Success { data: UsageQueryData::Balance(wallets) } =
            result.balance.outcome
        else {
            panic!("expected a credit balance");
        };
        assert_eq!(wallets.len(), 1);
        assert_eq!(wallets[0].currency, "USD");
        assert_eq!(wallets[0].remaining.map(f64::from), Some(16.0));

        let UsageQueryOutcome::Success { data: UsageQueryData::CodingPlan(windows) } =
            result.coding_plan.outcome
        else {
            panic!("expected quota windows");
        };
        assert_eq!(windows.len(), 3);
        for (window, (name, percent, reset)) in windows.iter().zip([
            ("five_hour", 25.0, Some("2030-01-01T05:00:00Z")),
            ("weekly_limit", 30.0, None),
            ("monthly", 8.0 / 24.0 * 100.0, Some("2030-02-01T00:00:00Z")),
        ]) {
            assert_eq!(window.name, name);
            assert_eq!(window.plan_label.as_deref(), Some("individual-goat"));
            assert!((f64::from(window.utilization.unwrap()) - percent).abs() < 1e-9);
            assert_eq!(window.resets_at.as_deref(), reset);
        }
        assert_eq!(windows[0].used.map(f64::from), Some(4.0));
        assert_eq!(windows[0].total.map(f64::from), Some(16.0));
        assert_eq!(windows[0].remaining.map(f64::from), Some(12.0));
    }
}

#[tokio::test]
async fn usage_coding_command_code_rejects_invalid_input() {
    for (base, key, code) in [
        ("https://api.commandcode.ai.example/provider", KEY, UsageQueryErrorCode::Unsupported),
        ("https://api.commandcode.ai/providers", KEY, UsageQueryErrorCode::Unsupported),
        (CLAUDE, "bad\r\nkey", UsageQueryErrorCode::CredentialUnavailable),
        (CODEX, "", UsageQueryErrorCode::CredentialUnavailable),
    ] {
        let result = query_command_code(base, key, NOW, ID).await;
        for result in [result.balance, result.coding_plan] {
            let UsageQueryOutcome::Failure { error, .. } = result.outcome else {
                panic!("expected rejection");
            };
            assert_eq!(error.code, code);
            assert_eq!(error.correlation_id, ID);
            assert_eq!(result.queried_at, None);
        }
    }
}

#[tokio::test]
async fn usage_coding_command_code_keeps_balance_and_rolling_windows_on_summary_failure() {
    let result = replay(
        CODEX,
        &[WHOAMI, CREDITS, SUBSCRIPTION, (SUMMARY_PATH, "503 Service Unavailable", KEY)],
    )
    .await;
    assert!(matches!(result.balance.outcome, UsageQueryOutcome::Success { .. }));
    let UsageQueryOutcome::Success { data: UsageQueryData::CodingPlan(windows) } =
        result.coding_plan.outcome
    else {
        panic!("expected rolling windows");
    };
    assert_eq!(
        windows.iter().map(|window| window.name.as_str()).collect::<Vec<_>>(),
        ["five_hour", "weekly_limit"]
    );
    assert_eq!(result.window_errors.len(), 1);
    assert_eq!(result.window_errors[0].code, UsageQueryErrorCode::UpstreamUnavailable);
    assert_eq!(result.window_errors[0].correlation_id, ID);
    assert!(!result.window_errors[0].message.contains(KEY));
}

#[tokio::test]
async fn usage_coding_command_code_discards_data_on_late_unauthorized_response() {
    let result =
        replay(CLAUDE, &[WHOAMI, CREDITS, SUBSCRIPTION, (SUMMARY_PATH, "401 Unauthorized", KEY)])
            .await;
    for result in [result.balance, result.coding_plan] {
        assert_eq!(result.credential_state, UsageCredentialState::Expired);
        let UsageQueryOutcome::Failure { error, .. } = result.outcome else {
            panic!("expected authentication failure");
        };
        assert_eq!(error.code, UsageQueryErrorCode::UpstreamUnauthorized);
        assert!(!error.message.contains(KEY));
    }
}

#[tokio::test]
async fn usage_coding_command_code_preserves_balance_when_subscription_lookup_fails() {
    let result = replay(
        CLAUDE,
        &[
            WHOAMI,
            (CREDITS.0, "200 OK", r#"{"credits":{"purchasedCredits":18.25}}"#),
            (SUBSCRIPTION.0, "429 Too Many Requests", KEY),
        ],
    )
    .await;
    assert!(matches!(result.balance.outcome, UsageQueryOutcome::Success { .. }));
    let UsageQueryOutcome::Failure { error, .. } = result.coding_plan.outcome else {
        panic!("expected the quota lookup failure");
    };
    assert_eq!(error.code, UsageQueryErrorCode::UpstreamRateLimited);
    assert!(!error.message.contains(KEY));
}

#[tokio::test]
async fn usage_coding_command_code_preserves_pay_as_you_go_without_a_subscription() {
    let result = replay(
        CODEX,
        &[
            (WHOAMI.0, "200 OK", r#"{"org":null}"#),
            (
                "/alpha/billing/credits",
                "200 OK",
                r#"{"credits":{"planId":"provider","purchasedCredits":18.25,"freeCredits":-1}}"#,
            ),
            ("/alpha/billing/subscriptions", "200 OK", r#"{"success":true,"data":null}"#),
            ("/alpha/usage/summary", "200 OK", r#"{"totalCost":"6"}"#),
        ],
    )
    .await;
    let UsageQueryOutcome::Success { data: UsageQueryData::Balance(wallets) } =
        result.balance.outcome
    else {
        panic!("expected pay-as-you-go balance");
    };
    assert_eq!(wallets[0].remaining.map(f64::from), Some(17.25));
    let UsageQueryOutcome::Success { data: UsageQueryData::CodingPlan(windows) } =
        result.coding_plan.outcome
    else {
        panic!("expected monthly usage without an active subscription");
    };
    assert_eq!(windows.len(), 1);
    assert_eq!(windows[0].plan_label.as_deref(), Some("provider"));
    assert_eq!(windows[0].resets_at, None);
    assert_eq!(windows[0].utilization.map(f64::from), Some(6.0 / 24.25 * 100.0));
}

#[tokio::test]
async fn usage_coding_command_code_reports_a_malformed_window_and_keeps_its_siblings() {
    let result = replay(
        CODEX,
        &[
            WHOAMI,
            (
                CREDITS.0,
                "200 OK",
                r#"{"credits":{"monthlyCredits":16,"windowLimits":{"limited":true,
                    "fiveHour":{"used":4,"cap":0},"weekly":{"used":12,"cap":40}}}}"#,
            ),
            SUBSCRIPTION,
            (SUMMARY_PATH, "200 OK", r#"{"totalCost":8}"#),
        ],
    )
    .await;
    let UsageQueryOutcome::Success { data: UsageQueryData::CodingPlan(windows) } =
        result.coding_plan.outcome
    else {
        panic!("expected the weekly and monthly windows");
    };
    assert_eq!(
        windows.iter().map(|window| window.name.as_str()).collect::<Vec<_>>(),
        ["weekly_limit", "monthly"]
    );
    assert_eq!(result.window_errors.len(), 1);
    assert_eq!(result.window_errors[0].code, UsageQueryErrorCode::InvalidResponse);
}
