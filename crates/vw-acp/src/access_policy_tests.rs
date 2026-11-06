use super::*;
use crate::{AcpAgentConfig, AcpClient};
use agent_client_protocol::{SessionMode, SessionModeState};

fn client(name: &str, command: &str) -> AcpClient {
    AcpClient::new(
        name,
        AcpAgentConfig { command: command.into(), args: Vec::new(), env: Default::default() },
    )
}

#[test]
fn acp_full_access_policy_missing_selection_preserves_default_approval() {
    let policy = client("codex", "codex-acp").access_policy(None, None);
    assert!(!policy.full_access_requested);
    assert_eq!(policy.permission_mode, PermissionMode::ApproveAll);
    assert_eq!(policy.reason, AccessPolicyReason::NotRequested);
    assert_eq!(policy.support, FullAccessSupport::Unavailable);
}

#[test]
fn acp_full_access_policy_no_options_is_client_only() {
    let policy = client("codex", "codex-acp").with_full_access(true).access_policy(None, Some(&[]));
    assert!(policy.full_access_requested);
    assert_eq!(policy.support, FullAccessSupport::ClientOnly);
    assert_eq!(policy.reason, AccessPolicyReason::NoAdvertisedControls);
}

#[test]
fn acp_full_access_policy_unknown_backend_and_custom_command_cannot_claim_native() {
    let modes = SessionModeState::new("auto", vec![SessionMode::new("auto", "Auto approve all")]);
    let config = vec![SessionConfigOption::boolean("full_access", "Full access", true)];
    for (name, command) in [("unknown", "unknown"), ("codex", "/custom/wrapper")] {
        let policy =
            client(name, command).with_full_access(true).access_policy(Some(&modes), Some(&config));
        assert_eq!(policy.support, FullAccessSupport::ClientOnly);
        assert_eq!(policy.reason, AccessPolicyReason::UnverifiedControls);
    }
}

#[test]
fn acp_full_access_policy_deny_precedes_explicit_selection_in_either_builder_order() {
    for enabled in [false, true] {
        let first = client("codex", "codex-acp")
            .with_full_access(enabled)
            .with_permission_mode(PermissionMode::DenyAll);
        let second = client("codex", "codex-acp")
            .with_permission_mode(PermissionMode::DenyAll)
            .with_full_access(enabled);
        for policy in [first.access_policy(None, None), second.access_policy(None, None)] {
            assert_eq!(policy.permission_mode, PermissionMode::DenyAll);
            assert_eq!(policy.support, FullAccessSupport::Unavailable);
            assert_eq!(policy.reason, AccessPolicyReason::DeniedByHost);
        }
    }
}

#[test]
fn acp_full_access_policy_read_approval_remains_partial() {
    let policy = client("codex", "codex-acp")
        .with_full_access(true)
        .with_permission_mode(PermissionMode::ApproveReads)
        .access_policy(None, None);
    assert_eq!(policy.support, FullAccessSupport::Partial);
    assert_eq!(policy.reason, AccessPolicyReason::ReadApprovalOnly);
    assert_eq!(policy.permission_mode, PermissionMode::ApproveReads);
}

#[test]
fn acp_full_access_policy_reject_only_modes_cannot_enable_native() {
    let modes = SessionModeState::new("deny", vec![SessionMode::new("deny", "Reject all")]);
    let policy =
        client("codex", "codex-acp").with_full_access(true).access_policy(Some(&modes), None);
    assert_eq!(policy.support, FullAccessSupport::ClientOnly);
    assert_eq!(policy.reason, AccessPolicyReason::UnverifiedControls);
}
