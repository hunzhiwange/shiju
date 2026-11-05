#[test]
fn acp_skill_mount_outbound_copilot_rejects_unverified_mounts() {
    assert!(super::super::validate("copilot", true).is_err());
    assert!(super::super::validate("copilot", false).is_ok());
}
