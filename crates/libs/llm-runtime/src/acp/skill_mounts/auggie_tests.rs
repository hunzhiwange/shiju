#[test]
fn acp_skill_mount_outbound_auggie_rejects_unverified_mounts() {
    assert!(super::super::validate("auggie", true).is_err());
    assert!(super::super::validate("auggie", false).is_ok());
}
