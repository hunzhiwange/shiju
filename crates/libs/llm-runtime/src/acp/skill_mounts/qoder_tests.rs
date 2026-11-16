#[test]
fn acp_skill_mount_outbound_qoder_rejects_unverified_mounts() {
    assert!(super::super::validate("qoder", true).is_err());
    assert!(super::super::validate("qoder", false).is_ok());
}
