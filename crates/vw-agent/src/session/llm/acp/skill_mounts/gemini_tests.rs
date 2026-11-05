#[test]
fn acp_skill_mount_outbound_gemini_rejects_unverified_mounts() {
    assert!(super::super::validate("gemini", true).is_err());
    assert!(super::super::validate("gemini", false).is_ok());
}
