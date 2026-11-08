use crate::{TimestampMs, agent_apps_auth::*};
use serde_json::{Value, from_value, json, to_value};

#[test]
fn auth6_dto_commits_expected_scoped_result() {
    let flow_id = AuthFlowId("fixture-flow-reference".into());
    let expires_at = TimestampMs(1_795_000_000_000);
    let verification_url = "https://github.example/login/device?state=fixture-state";
    let user_code = "FIXTURE-CODE";
    let start = AuthStartResponse::Pending {
        flow_id: flow_id.clone(),
        expires_at,
        retry_after_seconds: 5,
        verification_url: verification_url.into(),
        user_code: Some(user_code.into()),
        can_paste: None,
    };
    let started_json = json!({
        "state": "pending",
        "flow_id": flow_id.0,
        "expires_at": expires_at.0,
        "retry_after_seconds": 5,
        "verification_url": verification_url,
        "user_code": user_code
    });
    assert_eq!(to_value(&start).unwrap(), started_json);
    assert_eq!(
        from_value::<AuthStartResponse>(started_json).unwrap(),
        start
    );

    let browser = AuthStartResponse::Pending {
        flow_id: flow_id.clone(),
        expires_at,
        retry_after_seconds: 2,
        verification_url: "https://claude.example/login?state=fixture-browser-state".into(),
        user_code: None,
        can_paste: Some(true),
    };
    assert_eq!(
        to_value(&browser).unwrap(),
        json!({
            "state": "pending",
            "flow_id": flow_id.0,
            "expires_at": expires_at.0,
            "retry_after_seconds": 2,
            "verification_url": "https://claude.example/login?state=fixture-browser-state",
            "can_paste": true
        })
    );

    let pending = AuthFlow::Pending {
        flow_id: flow_id.clone(),
        expires_at,
        retry_after_seconds: 5,
    };
    assert_eq!(
        to_value(&pending).unwrap(),
        json!({
            "state": "pending",
            "flow_id": flow_id.0,
            "expires_at": expires_at.0,
            "retry_after_seconds": 5
        })
    );
    let account = AuthAccount {
        account_id: "fixture-account".into(),
        service: AuthService::GithubCopilot,
        label: "示例账号".into(),
        credential_state: AuthCredentialState::Ready,
        is_default: true,
        authenticated_at: Some(TimestampMs(1_794_999_000_000)),
        login_expires_at: None,
        github_domain: Some("github.example".into()),
        region: None,
    };
    let approved = AuthFlow::Approved {
        flow_id: flow_id.clone(),
        expires_at,
        account,
    };
    let approved_json = json!({
        "state": "approved",
        "flow_id": flow_id.0,
        "expires_at": expires_at.0,
        "account": {
            "account_id": "fixture-account",
            "service": "github_copilot",
            "label": "示例账号",
            "credential_state": "ready",
            "is_default": true,
            "authenticated_at": 1_794_999_000_000_i64,
            "github_domain": "github.example"
        }
    });
    assert_eq!(to_value(&approved).unwrap(), approved_json);
    assert_eq!(from_value::<AuthFlow>(approved_json).unwrap(), approved);
    let status = AuthStatus {
        service: AuthService::GithubCopilot,
        default_account_id: None,
        capabilities: AuthCapabilities {
            login_methods: vec![AuthLoginMethod::DeviceCode],
            can_set_default: true,
            can_bind: true,
            can_revoke: false,
        },
    };
    assert_eq!(
        to_value(&status).unwrap(),
        json!({
            "service": "github_copilot",
            "default_account_id": null,
            "capabilities": {
                "login_methods": ["device_code"],
                "can_set_default": true,
                "can_bind": true,
                "can_revoke": false
            }
        })
    );
    let failed = AuthFlow::Failed {
        flow_id: flow_id.clone(),
        expires_at,
        error: AuthError {
            code: AuthErrorCode::AuthorizationDenied,
            message: "Authorization declined".into(),
            correlation_id: "fixture-correlation".into(),
        },
    };
    assert_eq!(
        to_value(&failed).unwrap(),
        json!({
            "state": "failed",
            "flow_id": flow_id.0,
            "expires_at": expires_at.0,
            "error": {
                "code": "authorization_denied",
                "message": "Authorization declined",
                "correlation_id": "fixture-correlation"
            }
        })
    );

    let debug = format!("{start:?} {browser:?} {pending:?} {approved:?} {failed:?}");
    for sensitive in [
        &flow_id.0,
        verification_url,
        "fixture-browser-state",
        user_code,
    ] {
        assert!(
            !debug.contains(sensitive),
            "sensitive authorization data in Debug"
        );
    }
    let paste = AuthPasteRequest {
        pasted: "fixture-callback-authorization-code".into(),
    };
    assert!(
        !format!("{paste:?}").contains(&paste.pasted),
        "pasted authorization data in Debug"
    );
}

#[test]
fn auth6_dto_rejects_invalid_or_stale_input() {
    use AuthErrorCode::{InvalidInput, Unsupported};
    use AuthService::*;

    for (service, request, result) in [
        (
            GithubCopilot,
            json!({"method": "device_code", "github_domain": "github.example"}),
            Ok(()),
        ),
        (
            CodexOauth,
            json!({"method": "device_code", "target_account_id": "fixture"}),
            Ok(()),
        ),
        (
            CodexOauth,
            json!({"method": "browser", "target_account_id": "fixture"}),
            Ok(()),
        ),
        (XaiOauth, json!({"method": "device_code"}), Ok(())),
        (
            KimiOauth,
            json!({"method": "device_code", "region": "mainland-cn"}),
            Ok(()),
        ),
        (
            KimiOauth,
            json!({"method": "device_code", "target_account_id": "fixture"}),
            Ok(()),
        ),
        (ClaudeOfficial, json!({"method": "browser"}), Ok(())),
        (ClaudeOfficial, json!({"method": "copy_link"}), Ok(())),
        (AgyOauth, json!({"method": "browser"}), Ok(())),
        (DshAccount, json!({"method": "browser"}), Ok(())),
        (XaiOauth, json!({"method": "browser"}), Err(Unsupported)),
        (
            ClaudeOfficial,
            json!({"method": "device_code"}),
            Err(Unsupported),
        ),
        (CodexOauth, json!({"method": "copy_link"}), Err(Unsupported)),
        (
            KimiOauth,
            json!({"method": "device_code"}),
            Err(InvalidInput),
        ),
        (
            CodexOauth,
            json!({"method": "browser", "region": "global"}),
            Err(InvalidInput),
        ),
        (
            DshAccount,
            json!({"method": "browser", "github_domain": "github.example"}),
            Err(InvalidInput),
        ),
        (
            GithubCopilot,
            json!({"method": "device_code", "target_account_id": "fixture"}),
            Err(InvalidInput),
        ),
    ] {
        let request = from_value::<AuthStartRequest>(request).unwrap();
        assert_eq!(
            request.validate_for(service),
            result,
            "{service:?} {request:?}"
        );
    }
    assert!(from_value::<AuthService>(json!("unknown")).is_err());
    assert!(from_value::<AuthStartRequest>(json!({"method": "unknown"})).is_err());
    assert!(from_value::<AuthStartRequest>(json!({})).is_err());
    assert!(
        from_value::<AuthStartRequest>(json!({"method": "device_code", "region": "elsewhere"}))
            .is_err()
    );
    assert_eq!(to_value(Unsupported).unwrap(), "unsupported");
    assert_eq!(to_value(InvalidInput).unwrap(), "invalid_input");

    let challenge = json!({
        "state": "pending",
        "flow_id": "fixture-flow-reference",
        "expires_at": 1_795_000_000_000_i64,
        "retry_after_seconds": 5,
        "verification_url": "https://github.example/login/device"
    });
    for key in [
        "token",
        "access_token",
        "refresh_token",
        "id_token",
        "device_code",
        "code_verifier",
    ] {
        let mut request = json!({"method": "device_code"});
        request[key] = Value::String("fixture-secret".into());
        assert!(from_value::<AuthStartRequest>(request).is_err(), "{key}");
        let mut response = challenge.clone();
        response[key] = Value::String("fixture-secret".into());
        assert!(from_value::<AuthStartResponse>(response).is_err(), "{key}");
    }
    assert!(
        from_value::<AuthFlow>(challenge).is_err(),
        "challenge accepted as a poll response"
    );
    assert!(
        from_value::<AuthFlow>(json!({
            "state": "approved",
            "flow_id": "fixture-flow-reference",
            "expires_at": 1_795_000_000_000_i64,
            "account": {
                "account_id": "fixture-account",
                "service": "github_copilot",
                "label": "示例账号",
                "credential_state": "ready",
                "is_default": true,
                "refresh_token": "fixture-secret"
            }
        }))
        .is_err(),
        "secret accepted in nested account"
    );
    assert!(
        from_value::<AuthFlow>(json!({
            "state": "expired",
            "flow_id": "fixture-flow-reference",
            "expires_at": 1_795_000_000_000_i64,
            "retry_after_seconds": 5
        }))
        .is_err(),
        "expired flow accepted pending data"
    );
}
