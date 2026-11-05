#[test]
fn acp_skill_mount_outbound_openclaw_rejects_unverified_mounts() {
    assert!(super::super::validate("openclaw", true).is_err());
    assert!(super::super::validate("openclaw", false).is_ok());
}
