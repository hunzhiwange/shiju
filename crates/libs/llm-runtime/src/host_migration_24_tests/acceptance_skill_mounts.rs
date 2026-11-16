use crate::acp::acceptance_support::skill_mounts::accepted;
use serde_json::json;

#[test]
fn acp_skill_mount_harness_zero_requests_never_pass() {
    assert!(!accepted(&json!({"model_requests":0,"tool_results":0})));
}

#[test]
fn acp_skill_mount_harness_old_model_report_never_passes() {
    assert!(!accepted(&json!({"requests":[{"tool_results":5}],"errors":0})));
}

#[test]
fn acp_skill_mount_harness_requires_discovery_clean_prompt_and_real_results() {
    let evidence = json!({"errors":0,"model_requests":4,"tool_requests":3,"tool_results":3,
        "discovery_events":1,"first_prompt_clean":true,"attachment_read":true,"script_executed":true});
    assert!(accepted(&evidence));
    for key in ["model_requests", "tool_requests", "tool_results", "discovery_events"] {
        let mut invalid = evidence.clone();
        invalid[key] = json!(0);
        assert!(!accepted(&invalid));
    }
    for key in ["first_prompt_clean", "attachment_read", "script_executed"] {
        let mut invalid = evidence.clone();
        invalid[key] = json!(false);
        assert!(!accepted(&invalid));
    }
}
