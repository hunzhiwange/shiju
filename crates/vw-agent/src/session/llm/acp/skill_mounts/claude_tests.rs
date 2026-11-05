#[test]
fn acp_skill_mount_outbound_claude_rejects_unverified_mounts() {
    assert!(super::super::validate("claude", true).is_err());
    assert!(super::super::validate("claude", false).is_ok());
}
