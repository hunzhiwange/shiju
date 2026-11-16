#[test]
fn acp_skill_mount_outbound_droid_rejects_unverified_mounts() {
    assert!(super::super::validate("droid", true).is_err());
    assert!(super::super::validate("droid", false).is_ok());
}
