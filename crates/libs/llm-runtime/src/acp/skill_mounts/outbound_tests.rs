use super::*;
#[test]
fn acp_skill_mount_outbound_expanded_history_is_rejected() {
    let messages = serde_json::json!([{"role":"user","content":"<system-reminder>\nSkill \"selected\" is now active.\nBODY_SENTINEL\n</system-reminder>"}]);
    assert!(reject_expanded_invocations("codex", &messages).is_err());
    assert!(
        reject_expanded_invocations(
            "codex",
            &serde_json::json!([{"role":"user","content":"hello"}])
        )
        .is_ok()
    );
}
#[test]
fn acp_skill_mount_outbound_unknown_supplier_fails_closed() {
    assert!(validate("custom-supplier", true).is_err());
    assert!(validate("custom-supplier", false).is_ok());
}
