#[test]
fn acp_skill_mount_outbound_vibe_rejects_unverified_mounts() {
    assert!(super::super::validate("vibe", true).is_err());
    assert!(super::super::validate("vibe", false).is_ok());
}
