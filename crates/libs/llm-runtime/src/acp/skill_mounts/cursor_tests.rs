#[test]
fn acp_skill_mount_outbound_cursor_rejects_unverified_mounts() {
    assert!(super::super::validate("cursor", true).is_err());
    assert!(super::super::validate("cursor", false).is_ok());
}
