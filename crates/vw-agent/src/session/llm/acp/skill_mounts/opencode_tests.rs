#[test]
fn acp_skill_mount_outbound_opencode_rejects_unverified_mounts() {
    assert!(super::super::validate("opencode", true).is_err());
    assert!(super::super::validate("opencode", false).is_ok());
}
