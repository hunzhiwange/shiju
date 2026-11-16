#[test]
fn acp_skill_mount_codex_accepts_native_contract() {
    assert!(super::super::validate("codex", true).is_ok());
    assert!(super::super::validate("codex", false).is_ok());
}
