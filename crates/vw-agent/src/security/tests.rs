//! 验证安全模块公共导出可用且敏感值不会完整回显。

use super::*;

#[test]
fn redact_never_returns_full_secret() {
    assert_eq!(redact(""), "***");
    assert_eq!(redact("abcd"), "***");
    assert_eq!(redact("abc"), "***");
    assert_eq!(redact("abcdef"), "abcd***");
}

#[test]
fn public_reexports_are_usable_from_security_root() {
    let _ = CanaryGuard::new(true);
    let _ = DomainMatcher::new(&["example.com".to_string()], &[]).unwrap();
    let _ = OtpValidator::from_config;
    let _ = PairingGuard::new(false, &[]);
    let _ = SecurityPolicy::default();
    let _ = ShellRedirectPolicy::Block;
    let _ = AutonomyLevel::Supervised;
    let _ = SecretStore::new(std::path::Path::new("/tmp"), false);
    let _ = NoopSandbox;
    let _ = PromptGuard::default();
    let _ = GuardAction::Warn;
    let _ = GuardResult::Safe;
}
