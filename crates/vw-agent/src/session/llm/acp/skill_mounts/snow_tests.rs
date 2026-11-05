#[test]
fn acp_skill_mount_outbound_snow_rejects_unverified_mounts() {
    assert!(super::super::validate("snow", true).is_err());
    assert!(super::super::validate("snow", false).is_ok());
}
