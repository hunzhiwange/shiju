use serde_json::json;

use super::error_normalization::*;
use super::types::{
    EXIT_CODE_ERROR, EXIT_CODE_NO_SESSION, EXIT_CODE_PERMISSION_DENIED, EXIT_CODE_TIMEOUT,
    EXIT_CODE_USAGE, OutputErrorAcpPayload, OutputErrorCode, OutputErrorOrigin,
};

#[test]
fn metadata_overrides_default_code_and_origin() {
    let error = json!({
        "name": "TimeoutError",
        "message": "slow",
        "outputCode": "PERMISSION_DENIED",
        "detailCode": "POLICY",
        "origin": "queue",
        "retryable": false
    });

    let params = normalize_output_error(
        &error,
        NormalizeOutputErrorOptions {
            default_code: Some(OutputErrorCode::Runtime),
            origin: Some(OutputErrorOrigin::Acp),
            retryable: Some(true),
            ..NormalizeOutputErrorOptions::default()
        },
    );

    assert_eq!(params.code, OutputErrorCode::PermissionDenied);
    assert_eq!(params.detail_code.as_deref(), Some("POLICY"));
    assert_eq!(params.origin, Some(OutputErrorOrigin::Queue));
    assert_eq!(params.retryable, Some(false));
}

#[test]
fn auth_required_acp_payload_sets_detail_code() {
    let params = normalize_output_error(
        &json!({ "message": "authentication required" }),
        NormalizeOutputErrorOptions {
            acp: Some(OutputErrorAcpPayload {
                code: -32000,
                message: "auth required".to_string(),
                data: Some(json!({ "methodId": "token" })),
            }),
            ..NormalizeOutputErrorOptions::default()
        },
    );

    assert_eq!(params.detail_code.as_deref(), Some("AUTH_REQUIRED"));
    assert!(!is_retryable_prompt_error(&json!({
        "acp": { "code": -32000, "message": "auth required" }
    })));
}

#[test]
fn query_closed_detection_requires_internal_error_details() {
    assert!(is_acp_query_closed_before_response_error(&json!({
        "acp": {
            "code": -32603,
            "message": "internal error",
            "data": { "details": "Query closed before response received" }
        }
    })));
    assert!(!is_acp_query_closed_before_response_error(&json!({
        "acp": {
            "code": -32602,
            "message": "invalid params",
            "data": { "details": "Query closed before response received" }
        }
    })));
}

#[test]
fn format_error_message_preserves_known_message_sources() {
    assert_eq!(format_error_message(&json!("plain failure")), "plain failure");
    assert_eq!(
        format_error_message(&json!({ "message": "structured failure" })),
        "structured failure"
    );
    assert_eq!(format_error_message(&json!({ "code": "E_CUSTOM" })), r#"{"code":"E_CUSTOM"}"#);
    assert_eq!(format_error_message(&json!(false)), "false");
}

#[test]
fn normalize_output_error_maps_named_error_codes() {
    let cases = [
        ("PermissionPromptUnavailableError", OutputErrorCode::PermissionPromptUnavailable),
        ("PermissionDeniedError", OutputErrorCode::PermissionDenied),
        ("TimeoutError", OutputErrorCode::Timeout),
        ("NoSessionError", OutputErrorCode::NoSession),
        ("CommanderError", OutputErrorCode::Usage),
        ("InvalidArgumentError", OutputErrorCode::Usage),
    ];

    for (name, expected) in cases {
        let params = normalize_output_error(
            &json!({
                "name": name,
                "message": format!("{name} message")
            }),
            NormalizeOutputErrorOptions::default(),
        );

        assert_eq!(params.code, expected, "{name}");
    }
}

#[test]
fn normalize_output_error_maps_commander_code_to_usage() {
    let params = normalize_output_error(
        &json!({
            "name": "OtherError",
            "code": "commander.invalidArgument",
            "message": "bad arg"
        }),
        NormalizeOutputErrorOptions::default(),
    );

    assert_eq!(params.code, OutputErrorCode::Usage);
}

#[test]
fn normalize_output_error_uses_options_when_metadata_is_absent() {
    let params = normalize_output_error(
        &json!({ "message": "fallback" }),
        NormalizeOutputErrorOptions {
            default_code: Some(OutputErrorCode::Timeout),
            detail_code: Some("FROM_OPTIONS".to_string()),
            origin: Some(OutputErrorOrigin::Runtime),
            retryable: Some(true),
            acp: Some(OutputErrorAcpPayload {
                code: -32603,
                message: "internal error".to_string(),
                data: None,
            }),
        },
    );

    assert_eq!(params.code, OutputErrorCode::Timeout);
    assert_eq!(params.detail_code.as_deref(), Some("FROM_OPTIONS"));
    assert_eq!(params.origin, Some(OutputErrorOrigin::Runtime));
    assert_eq!(params.retryable, Some(true));
    assert_eq!(params.acp.as_ref().map(|payload| payload.code), Some(-32603));
    assert_eq!(params.timestamp, None);
}

#[test]
fn normalize_output_error_parses_all_embedded_output_codes() {
    let cases = [
        ("NO_SESSION", OutputErrorCode::NoSession),
        ("TIMEOUT", OutputErrorCode::Timeout),
        ("PERMISSION_DENIED", OutputErrorCode::PermissionDenied),
        ("PERMISSION_PROMPT_UNAVAILABLE", OutputErrorCode::PermissionPromptUnavailable),
        ("RUNTIME", OutputErrorCode::Runtime),
        ("USAGE", OutputErrorCode::Usage),
    ];

    for (output_code, expected) in cases {
        let params = normalize_output_error(
            &json!({
                "message": "with output code",
                "outputCode": output_code
            }),
            NormalizeOutputErrorOptions::default(),
        );

        assert_eq!(params.code, expected, "{output_code}");
    }
}

#[test]
fn normalize_output_error_parses_all_embedded_origins() {
    let cases = [
        ("cli", OutputErrorOrigin::Cli),
        ("runtime", OutputErrorOrigin::Runtime),
        ("queue", OutputErrorOrigin::Queue),
        ("acp", OutputErrorOrigin::Acp),
    ];

    for (origin, expected) in cases {
        let params = normalize_output_error(
            &json!({
                "message": "with origin",
                "origin": origin
            }),
            NormalizeOutputErrorOptions::default(),
        );

        assert_eq!(params.origin, Some(expected), "{origin}");
    }
}

#[test]
fn normalize_output_error_ignores_invalid_metadata_fields() {
    let params = normalize_output_error(
        &json!({
            "message": "invalid metadata",
            "outputCode": "CUSTOM",
            "detailCode": "   ",
            "origin": "worker",
            "retryable": "yes",
            "acp": {
                "code": -32603,
                "message": "   "
            }
        }),
        NormalizeOutputErrorOptions::default(),
    );

    assert_eq!(params.code, OutputErrorCode::Runtime);
    assert_eq!(params.detail_code, None);
    assert_eq!(params.origin, None);
    assert_eq!(params.retryable, None);
    assert_eq!(params.acp, None);
}

#[test]
fn normalize_output_error_resource_not_found_overrides_runtime_metadata() {
    let params = normalize_output_error(
        &json!({
            "message": "Session \"abc-123\" not found",
            "outputCode": "RUNTIME"
        }),
        NormalizeOutputErrorOptions::default(),
    );

    assert_eq!(params.code, OutputErrorCode::NoSession);
}

#[test]
fn normalize_output_error_detects_auth_required_sources() {
    let auth_messages = [
        "auth required",
        "authentication required",
        "authorization required",
        "credential required",
        "credentials required",
        "token required",
        "login required",
    ];

    for message in auth_messages {
        let params = normalize_output_error(
            &json!({
                "message": message,
                "acp": {
                    "code": -32000,
                    "message": message
                }
            }),
            NormalizeOutputErrorOptions::default(),
        );

        assert_eq!(params.detail_code.as_deref(), Some("AUTH_REQUIRED"), "{message}");
    }

    let auth_policy_params = normalize_output_error(
        &json!({
            "name": "AuthPolicyError",
            "message": "auth policy failed"
        }),
        NormalizeOutputErrorOptions::default(),
    );
    assert_eq!(auth_policy_params.detail_code.as_deref(), Some("AUTH_REQUIRED"));
}

#[test]
fn normalize_output_error_detects_auth_required_acp_data_shapes() {
    let cases = [
        json!({ "authRequired": true }),
        json!({ "methodId": "oauth" }),
        json!({ "methods": ["oauth"] }),
    ];

    for data in cases {
        let params = normalize_output_error(
            &json!({
                "message": "authorization pending",
                "acp": {
                    "code": -32000,
                    "message": "authorization pending",
                    "data": data
                }
            }),
            NormalizeOutputErrorOptions::default(),
        );

        assert_eq!(params.detail_code.as_deref(), Some("AUTH_REQUIRED"));
    }

    let params = normalize_output_error(
        &json!({
            "message": "ordinary policy failure",
            "acp": {
                "code": -32000,
                "message": "ordinary policy failure",
                "data": {
                    "authRequired": false,
                    "methodId": " ",
                    "methods": []
                }
            }
        }),
        NormalizeOutputErrorOptions::default(),
    );

    assert_eq!(params.detail_code, None);
}

#[test]
fn normalize_output_error_prefers_option_acp_payload_over_embedded_payload() {
    let params = normalize_output_error(
        &json!({
            "message": "embedded",
            "acp": {
                "code": -32602,
                "message": "embedded invalid params"
            }
        }),
        NormalizeOutputErrorOptions {
            acp: Some(OutputErrorAcpPayload {
                code: -32603,
                message: "option internal error".to_string(),
                data: None,
            }),
            ..NormalizeOutputErrorOptions::default()
        },
    );

    assert_eq!(params.acp.as_ref().map(|payload| payload.code), Some(-32603));
}

#[test]
fn normalize_output_error_falls_back_to_nested_acp_payload_and_no_session_mapping() {
    let params = normalize_output_error(
        &json!({
            "message": "outer wrapper",
            "error": {
                "code": -32001,
                "message": "session missing",
                "data": { "details": "unknown session" }
            }
        }),
        NormalizeOutputErrorOptions::default(),
    );

    assert_eq!(params.code, OutputErrorCode::NoSession);
    assert_eq!(params.acp.as_ref().map(|payload| payload.code), Some(-32001));
    assert!(params.acp.as_ref().and_then(|payload| payload.data.as_ref()).is_some());
}

#[test]
fn normalize_output_error_prefers_explicit_detail_code_over_auth_inference() {
    let params = normalize_output_error(
        &json!({
            "name": "AuthPolicyError",
            "message": "authentication required",
            "acp": {
                "code": -32000,
                "message": "auth required",
                "data": { "authRequired": true }
            }
        }),
        NormalizeOutputErrorOptions {
            detail_code: Some("FROM_OPTIONS".to_string()),
            ..NormalizeOutputErrorOptions::default()
        },
    );

    assert_eq!(params.detail_code.as_deref(), Some("FROM_OPTIONS"));
}

#[test]
fn normalize_output_error_does_not_infer_auth_required_for_other_acp_codes() {
    let params = normalize_output_error(
        &json!({
            "message": "authentication required",
            "acp": {
                "code": -32099,
                "message": "authentication required",
                "data": { "authRequired": true, "methodId": "token" }
            }
        }),
        NormalizeOutputErrorOptions::default(),
    );

    assert_eq!(params.detail_code, None);
}

#[test]
fn query_closed_detection_rejects_missing_or_non_matching_details() {
    let cases = [
        json!({ "message": "no acp" }),
        json!({ "acp": { "code": -32603, "message": "internal error" } }),
        json!({
            "acp": {
                "code": -32603,
                "message": "internal error",
                "data": { "details": 123 }
            }
        }),
        json!({
            "acp": {
                "code": -32603,
                "message": "internal error",
                "data": { "details": "different internal error" }
            }
        }),
    ];

    for error in cases {
        assert!(!is_acp_query_closed_before_response_error(&error));
    }
}

#[test]
fn query_closed_detection_reads_nested_acp_errors() {
    assert!(is_acp_query_closed_before_response_error(&json!({
        "error": {
            "code": -32603,
            "message": "internal error",
            "data": { "details": "query closed before response received" }
        }
    })));
}

#[test]
fn retryable_prompt_error_rejects_metadata_and_named_non_retryable_errors() {
    let output_codes =
        ["PERMISSION_DENIED", "PERMISSION_PROMPT_UNAVAILABLE", "TIMEOUT", "NO_SESSION", "USAGE"];

    for output_code in output_codes {
        assert!(!is_retryable_prompt_error(&json!({
            "outputCode": output_code,
            "acp": {
                "code": -32603,
                "message": "internal error"
            }
        })));
    }

    let names = [
        "PermissionDeniedError",
        "PermissionPromptUnavailableError",
        "TimeoutError",
        "NoSessionError",
        "CommanderError",
        "InvalidArgumentError",
    ];

    for name in names {
        assert!(!is_retryable_prompt_error(&json!({
            "name": name,
            "message": "non retryable",
            "acp": {
                "code": -32603,
                "message": "internal error"
            }
        })));
    }

    assert!(!is_retryable_prompt_error(&json!({
        "detailCode": "AUTH_REQUIRED",
        "acp": {
            "code": -32603,
            "message": "internal error"
        }
    })));
}

#[test]
fn retryable_prompt_error_rejects_nested_auth_required_payload() {
    assert!(!is_retryable_prompt_error(&json!({
        "error": {
            "code": -32000,
            "message": "auth required",
            "data": { "methods": ["oauth"] }
        }
    })));
}

#[test]
fn retryable_prompt_error_only_accepts_transient_acp_codes() {
    assert!(!is_retryable_prompt_error(&json!({ "message": "plain" })));

    for code in [-32001, -32002, -32601, -32602, -32000, -32099] {
        assert!(!is_retryable_prompt_error(&json!({
            "acp": {
                "code": code,
                "message": if code == -32000 { "token required" } else { "acp error" }
            }
        })));
    }

    for code in [-32603, -32700] {
        assert!(is_retryable_prompt_error(&json!({
            "error": {
                "code": code,
                "message": "transient acp error"
            }
        })));
    }
}

#[test]
fn normalize_output_error_handles_scalar_errors() {
    let params = normalize_output_error(
        &json!("plain failure"),
        NormalizeOutputErrorOptions {
            default_code: Some(OutputErrorCode::Usage),
            ..NormalizeOutputErrorOptions::default()
        },
    );

    assert_eq!(params.code, OutputErrorCode::Usage);
    assert_eq!(params.message, "plain failure");
    assert_eq!(params.detail_code, None);
    assert_eq!(params.acp, None);
}

#[test]
fn normalize_output_error_requires_auth_acp_signals() {
    for acp in [
        json!({ "code": -32000, "message": "ordinary policy failure" }),
        json!({
            "code": -32000,
            "message": "ordinary policy failure",
            "data": ["oauth"]
        }),
    ] {
        let params = normalize_output_error(
            &json!({ "message": "ordinary policy failure", "acp": acp }),
            NormalizeOutputErrorOptions::default(),
        );

        assert_eq!(params.detail_code, None);
    }
}

#[test]
fn retryable_prompt_error_allows_runtime_metadata_for_transient_acp() {
    assert!(is_retryable_prompt_error(&json!({
        "outputCode": "RUNTIME",
        "acp": {
            "code": -32603,
            "message": "internal error"
        }
    })));
}

#[test]
fn exit_code_mapping_matches_output_code() {
    assert_eq!(exit_code_for_output_error_code(OutputErrorCode::Usage), EXIT_CODE_USAGE);
    assert_eq!(exit_code_for_output_error_code(OutputErrorCode::Timeout), EXIT_CODE_TIMEOUT);
    assert_eq!(exit_code_for_output_error_code(OutputErrorCode::NoSession), EXIT_CODE_NO_SESSION);
    assert_eq!(
        exit_code_for_output_error_code(OutputErrorCode::PermissionDenied),
        EXIT_CODE_PERMISSION_DENIED
    );
    assert_eq!(
        exit_code_for_output_error_code(OutputErrorCode::PermissionPromptUnavailable),
        EXIT_CODE_PERMISSION_DENIED
    );
    assert_eq!(exit_code_for_output_error_code(OutputErrorCode::Runtime), EXIT_CODE_ERROR);
}
