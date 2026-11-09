use crate::TimestampMs;
use crate::apps_usage::{
    NonFiniteUsageNumber, UsageCredentialState, UsageNumber, UsageQueryResult,
};
use serde::Deserialize;
use serde::de::value::{Error, F64Deserializer};
use serde_json::{Value, json};

#[test]
fn usage_types_returns_expected_result() {
    let window = json!({
        "name": "five_hour",
        "planLabel": "Pro",
        "utilization": 12.5,
        "used": 1.25,
        "total": 10.0,
        "remaining": 8.75,
        "unit": "USD",
        "resetsAt": "2026-11-09T18:15:00+08:00"
    });
    let windows = json!([window, {
        "name": "seven_day", "planLabel": null, "utilization": 0.0,
        "used": null, "total": null, "remaining": null, "unit": null, "resetsAt": null
    }]);
    let fixtures = [
        (
            "official",
            "codex",
            "official_subscription",
            json!({
                "subscriptionType": "pro",
                "rateLimitTier": "tier_2",
                "windows": windows,
                "extraUsage": {
                    "isEnabled": true, "monthlyLimit": 10.5, "usedCredits": 0.0,
                    "utilization": null, "currency": "USD"
                },
                "creditsBalance": 32.25,
                "resetCredits": ["2026-11-10T00:00:00Z", "2026-11-10T00:00:00Z", null]
            }),
        ),
        (
            "provider",
            "deepseek",
            "balance",
            json!([
                {"currency": "CNY", "remaining": -0.5, "total": null,
                 "used": null, "granted": 0.0, "toppedUp": 5.25},
                {"currency": "USD", "remaining": 1.0, "total": 2.0,
                 "used": 1.0, "granted": null, "toppedUp": null}
            ]),
        ),
        ("provider", "zenmux", "coding_plan", windows),
        (
            "script",
            "custom",
            "custom_script",
            json!([
                {"planName": "Basic", "extra": "requests", "isValid": true,
                 "invalidMessage": null, "total": 50.0, "used": 50.0, "remaining": 0.0,
                 "unit": "requests"},
                {"planName": "Pro", "extra": null, "isValid": false,
                 "invalidMessage": "套餐已结束", "total": null, "used": null, "remaining": null,
                 "unit": null}
            ]),
        ),
    ];

    for (source_kind, source_id, kind, value) in fixtures {
        let wire = json!({
            "source": {"kind": source_kind, "id": source_id},
            "queriedAt": 1794238218934_i64,
            "credentialState": "valid",
            "credentialMessage": null,
            "outcome": {"status": "success", "data": {"kind": kind, "value": value}}
        });
        let result = assert_round_trip(wire);
        assert_eq!(result.queried_at, Some(TimestampMs(1794238218934)));
        assert_eq!(result.credential_state, UsageCredentialState::Valid);
    }

    for (wire_state, state) in [
        ("expired", UsageCredentialState::Expired),
        ("refresh_pending", UsageCredentialState::RefreshPending),
        ("not_found", UsageCredentialState::NotFound),
        ("parse_error", UsageCredentialState::ParseError),
    ] {
        let result = assert_round_trip(json!({
            "source": {"kind": "official", "id": "codex"},
            "queriedAt": null,
            "credentialState": wire_state,
            "credentialMessage": "无法取用凭据",
            "outcome": {
                "status": "failure",
                "kind": "official_subscription",
                "error": {"code": "credential_unavailable", "message": "无法查询额度", "correlationId": "query-a"}
            }
        }));
        assert_eq!(result.credential_state, state);
        assert_eq!(result.queried_at, None);
    }
}

#[test]
fn usage_types_rejects_invalid_input() {
    for invalid in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert_eq!(UsageNumber::try_from(invalid), Err(NonFiniteUsageNumber));
        assert!(UsageNumber::deserialize(F64Deserializer::<Error>::new(invalid)).is_err());
    }
    for invalid in ["1e999", "-1e999", "NaN", "\"Infinity\""] {
        let wire = format!(
            r#"{{"source":{{"kind":"provider","id":"deepseek"}},"queriedAt":0,
            "credentialState":"valid","outcome":{{"status":"success","data":{{
            "kind":"balance","value":[{{"currency":"CNY","remaining":{invalid}}}]}}}}}}"#
        );
        assert!(serde_json::from_str::<UsageQueryResult>(&wire).is_err());
    }
}

#[test]
fn usage_types_preserves_finite_number_precision() {
    for value in [-0.0, 0.30000000000000004, 1.2345678901234567e100, f64::from_bits(1), f64::MAX] {
        let amount = UsageNumber::try_from(value).unwrap();
        let wire = serde_json::to_string(&amount).unwrap();
        let decoded: UsageNumber = serde_json::from_str(&wire).unwrap();
        assert_eq!(f64::from(decoded).to_bits(), value.to_bits(), "{wire}");
    }
}

fn assert_round_trip(wire: Value) -> UsageQueryResult {
    let decoded: UsageQueryResult = serde_json::from_str(&wire.to_string()).unwrap();
    let encoded = serde_json::to_string(&decoded).unwrap();
    assert_eq!(serde_json::from_str::<Value>(&encoded).unwrap(), wire);
    assert_eq!(serde_json::from_str::<UsageQueryResult>(&encoded).unwrap(), decoded);
    decoded
}
