use super::validate;

#[test]
fn acp_skill_coexistence_deepagents_requires_a_reviewed_platform_for_mounts() {
    assert!(validate("deepagents", false).is_ok());
    assert_eq!(
        validate("deepagents", true).is_ok(),
        cfg!(all(target_os = "macos", target_arch = "aarch64"))
    );
    assert!(validate("unknown", true).is_err());
}
