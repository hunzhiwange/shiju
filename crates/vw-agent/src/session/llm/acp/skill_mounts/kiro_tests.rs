#[test]
fn acp_skill_mount_outbound_kiro_rejects_unverified_mounts() {
    assert!(super::super::validate("kiro", true).is_err());
    assert!(super::super::validate("kiro", false).is_ok());
}
