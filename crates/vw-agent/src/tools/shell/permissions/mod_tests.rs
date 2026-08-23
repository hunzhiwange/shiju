//! 验证 shell 权限结果构造器与权限上下文的安全默认值。

use std::path::PathBuf;

use crate::security::AutonomyLevel;
use crate::tools::shell::permissions::{
    Permission, PermissionContext, PermissionMode, PermissionResult,
};
use crate::tools::shell::security::{SecurityCategory, SecurityFinding, Severity};

#[test]
fn permission_result_default_has_no_decision_or_findings() {
    let result = PermissionResult::default();

    assert_eq!(result.permission, None);
    assert!(result.security_findings.is_empty());
}

#[test]
fn permission_result_helpers_build_expected_variants() {
    assert_eq!(PermissionResult::allow().permission, Some(Permission::Allow));
    assert_eq!(
        PermissionResult::deny("blocked").permission,
        Some(Permission::Deny { reason: "blocked".into() })
    );
    assert_eq!(
        PermissionResult::ask("review", Some("warning".into())).permission,
        Some(Permission::Ask { reason: "review".into(), warning: Some("warning".into()) })
    );
}

#[test]
fn permission_result_ask_preserves_missing_warning() {
    assert_eq!(
        PermissionResult::ask("review", None).permission,
        Some(Permission::Ask { reason: "review".into(), warning: None })
    );
}

#[test]
fn permission_result_with_findings_replaces_existing_findings_and_preserves_permission() {
    let original_findings = vec![SecurityFinding {
        severity: Severity::Warn,
        category: SecurityCategory::UnsafePattern,
        message: "original".into(),
        suggestion: None,
    }];
    let replacement_findings = vec![SecurityFinding {
        severity: Severity::Warn,
        category: SecurityCategory::UnsafePattern,
        message: "replacement".into(),
        suggestion: Some("review".into()),
    }];

    let result = PermissionResult::deny("blocked")
        .with_findings(original_findings)
        .with_findings(replacement_findings.clone());

    assert_eq!(result.permission, Some(Permission::Deny { reason: "blocked".into() }));
    assert_eq!(result.security_findings, replacement_findings);
}

#[test]
fn permission_context_new_uses_safe_defaults() {
    let workspace_dir = PathBuf::from("/workspace");
    let context = PermissionContext::new(AutonomyLevel::Supervised, workspace_dir.clone());

    assert_eq!(context.autonomy, AutonomyLevel::Supervised);
    assert!(!context.in_sandbox);
    assert_eq!(context.mode, PermissionMode::Normal);
    assert!(!context.approved);
    assert_eq!(context.workspace_dir, workspace_dir);
    assert!(context.allowed_roots.is_empty());
}
