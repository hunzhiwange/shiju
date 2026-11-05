#[test]
fn acp_skill_mount_outbound_hermes_rejects_unverified_mounts() {
    assert!(super::super::validate("hermes", true).is_err());
    assert!(super::super::validate("hermes", false).is_ok());
}
