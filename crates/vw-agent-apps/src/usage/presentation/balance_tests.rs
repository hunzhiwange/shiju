use super::balance::{
    ScriptPackageStatus, balance_line, query_status, recharge_detail_url, script_package_lines,
};
use super::tiers::QuotaTone;
use crate::usage::cache::UsageCacheResult;
use std::sync::Arc;
use vw_api_types::TimestampMs;
use vw_api_types::apps_usage::{
    UsageCredentialState, UsageNumber, UsageQueryData, UsageQueryError, UsageQueryErrorCode,
    UsageQueryKind, UsageQueryOutcome, UsageQueryResult, UsageScriptPackage, UsageSource,
};

fn amount(value: f64) -> Option<UsageNumber> {
    Some(UsageNumber::try_from(value).unwrap())
}

#[test]
fn usage_presentation_balance_returns_expected_result() {
    let debt = balance_line(amount(-24.96), None).unwrap();
    assert_eq!(f64::from(debt.remaining), -24.96);
    assert_eq!(debt.tone, QuotaTone::Danger);
    assert_eq!(debt.bar_percent, None);

    let low = balance_line(amount(5.0), amount(100.0)).unwrap();
    assert_eq!(low.bar_percent, Some(5.0));
    assert_eq!(low.tone, QuotaTone::Normal);
    let zero = balance_line(amount(0.0), None).unwrap();
    assert_eq!(zero.tone, QuotaTone::Danger);
    assert_eq!(zero.bar_percent, None);
    assert_eq!(balance_line(None, amount(100.0)), None);
    for total in [None, amount(0.0), amount(-1.0)] {
        assert_eq!(balance_line(amount(5.0), total).unwrap().bar_percent, None);
    }
    assert_eq!(balance_line(amount(5.0), amount(1.0)).unwrap().bar_percent, Some(100.0));
    assert_eq!(balance_line(amount(-5.0), amount(1.0)).unwrap().bar_percent, Some(0.0));
}

#[test]
fn usage_presentation_balance_rejects_invalid_input() {
    let valid = "https://pay.example.test/wallet/top-up?tab=balance#details";
    assert_eq!(recharge_detail_url(valid).unwrap().as_str(), valid);
    for value in [
        "",
        "/top-up",
        "//pay.example.test/top-up",
        "https:///top-up",
        "https:pay.example.test",
        "https://",
        "http://pay.example.test/top-up",
        "javascript:alert(1)",
        "file:///top-up",
        "https://user:pass@pay.example.test/top-up",
        "https://@pay.example.test/top-up",
        "https://pay.example.test/\ntop-up",
        r"https://pay.example.test\other.test/top-up",
    ] {
        assert!(recharge_detail_url(value).is_none(), "accepted {value:?}");
    }
}

#[test]
fn usage_presentation_balance_keeps_available_and_expired_packages() {
    let active = UsageScriptPackage {
        plan_name: Some("Active plan".into()),
        extra: None,
        is_valid: Some(true),
        invalid_message: None,
        total: amount(100.0),
        used: amount(40.0),
        remaining: amount(60.0),
        unit: Some("requests".into()),
    };
    let packages = [
        active.clone(),
        UsageScriptPackage {
            plan_name: Some("Expired plan".into()),
            is_valid: Some(false),
            invalid_message: Some("  Plan ended  ".into()),
            ..active.clone()
        },
        UsageScriptPackage { is_valid: None, remaining: None, ..active },
    ];

    let lines = script_package_lines(&packages);
    let [active, expired, used_only] = lines.as_slice() else {
        panic!("expected every package in input order");
    };
    assert_eq!(active.package.plan_name.as_deref(), Some("Active plan"));
    let ScriptPackageStatus::Available(Some(balance)) = active.status else {
        panic!("expected an available balance");
    };
    assert_eq!(balance.bar_percent, Some(60.0));
    assert_eq!(expired.package.plan_name.as_deref(), Some("Expired plan"));
    assert_eq!(expired.status, ScriptPackageStatus::Expired { reason: Some("Plan ended") });
    assert_eq!(used_only.status, ScriptPackageStatus::Available(None));
    assert_eq!(used_only.package.used, amount(40.0));
}

#[test]
fn usage_presentation_balance_distinguishes_stale_and_expired_credentials() {
    let success = UsageQueryResult {
        source: UsageSource::Provider("fixture".into()),
        queried_at: Some(TimestampMs(10)),
        credential_state: UsageCredentialState::Valid,
        credential_message: None,
        outcome: UsageQueryOutcome::Success { data: UsageQueryData::Balance(vec![]) },
    };
    let fresh = UsageCacheResult::Current(success.clone());
    let current = query_status(&fresh);
    assert!(!current.stale);
    assert!(current.failure.is_none());

    let failure = UsageQueryResult {
        queried_at: Some(TimestampMs(20)),
        outcome: UsageQueryOutcome::Failure {
            kind: UsageQueryKind::Balance,
            error: UsageQueryError {
                code: UsageQueryErrorCode::Timeout,
                message: "Request timed out".into(),
                correlation_id: "fixture-timeout".into(),
            },
        },
        ..success.clone()
    };
    let stale = UsageCacheResult::Stale { last_success: Arc::new(success), failure };
    let stale_line = query_status(&stale);
    assert!(stale_line.stale);
    assert_eq!(stale_line.queried_at, Some(TimestampMs(10)));
    let error = stale_line.failure.unwrap();
    assert_eq!(error.code, UsageQueryErrorCode::Timeout);
    assert_eq!(error.message, "Request timed out");
    assert_eq!(error.correlation_id, "fixture-timeout");

    let UsageCacheResult::Stale { failure, .. } = stale else { unreachable!() };
    let rejected = UsageCacheResult::Current(UsageQueryResult {
        credential_state: UsageCredentialState::Expired,
        credential_message: Some("Sign in again".into()),
        outcome: UsageQueryOutcome::Failure {
            kind: UsageQueryKind::Balance,
            error: UsageQueryError {
                code: UsageQueryErrorCode::UpstreamUnauthorized,
                message: "Authorization rejected".into(),
                correlation_id: "fixture-auth".into(),
            },
        },
        ..failure
    });
    let line = query_status(&rejected);
    assert!(!line.stale);
    assert_eq!(line.queried_at, Some(TimestampMs(20)));
    assert_eq!(line.credential_state, UsageCredentialState::Expired);
    assert_eq!(line.credential_message, Some("Sign in again"));
    assert_eq!(line.failure.unwrap().code, UsageQueryErrorCode::UpstreamUnauthorized);
}
