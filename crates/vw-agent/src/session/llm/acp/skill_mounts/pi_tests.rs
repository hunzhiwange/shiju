#[test]
fn acp_skill_mount_outbound_pi_rejects_unverified_mounts() {
    assert!(super::super::validate("pi", true).is_err());
    assert!(super::super::validate("pi", false).is_ok());
}
