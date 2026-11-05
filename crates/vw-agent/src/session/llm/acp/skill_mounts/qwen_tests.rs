#[test]
fn acp_skill_mount_outbound_qwen_rejects_unverified_mounts() {
    assert!(super::super::validate("qwen", true).is_err());
    assert!(super::super::validate("qwen", false).is_ok());
}
