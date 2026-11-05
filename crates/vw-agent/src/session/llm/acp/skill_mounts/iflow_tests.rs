#[test]
fn acp_skill_mount_outbound_iflow_rejects_unverified_mounts() {
    assert!(super::super::validate("iflow", true).is_err());
    assert!(super::super::validate("iflow", false).is_ok());
}
