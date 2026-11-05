#[test]
fn acp_skill_mount_outbound_codex_rejects_unverified_mounts() {
    assert!(super::super::validate("codex", true).is_err());
    assert!(super::super::validate("codex", false).is_ok());
}
