#[test]
fn acp_skill_mount_outbound_kilocode_rejects_unverified_mounts() {
    assert!(super::super::validate("kilocode", true).is_err());
    assert!(super::super::validate("kilocode", false).is_ok());
}
