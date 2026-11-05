#[test]
fn acp_skill_mount_outbound_kimi_rejects_unverified_mounts() {
    assert!(super::super::validate("kimi", true).is_err());
    assert!(super::super::validate("kimi", false).is_ok());
}
