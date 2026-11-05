#[test]
fn acp_skill_mount_outbound_goose_rejects_unverified_mounts() {
    assert!(super::super::validate("goose", true).is_err());
    assert!(super::super::validate("goose", false).is_ok());
}
