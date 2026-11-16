#[test]
fn acp_skill_mount_outbound_trae_rejects_unverified_mounts() {
    assert!(super::super::validate("trae", true).is_err());
    assert!(super::super::validate("trae", false).is_ok());
}
