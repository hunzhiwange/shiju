use super::acp_projection::*;
use vw_api_app_identity::{AppId, AppRuntimeKind};

#[test]
fn cc_apps_acp_projection_preserves_registry_entries_and_legacy_view() {
    let projection = acp_registry();
    assert_eq!(projection, acp_agent_registry::built_in_agent_specs());
    assert_eq!(legacy_registry(), acp_agent_registry::built_in_agent_registry());
    assert!(legacy_registry().contains_key("claude-legacy"));
}

#[test]
fn cc_apps_acp_projection_preserves_registry_multiple_forms() {
    let apps = catalog_with_acp();
    assert_eq!(apps.len(), 14);
    let claude = apps.iter().find(|app| app.metadata.id == AppId::Claude).unwrap();
    assert_eq!(
        claude.metadata.runtime_kinds,
        vec![AppRuntimeKind::NativeCli, AppRuntimeKind::AcpAdapter]
    );
    let desktop = apps.iter().find(|app| app.metadata.id == AppId::ClaudeDesktop).unwrap();
    assert_eq!(desktop.metadata.runtime_kinds, vec![AppRuntimeKind::DesktopApp]);
    let registry = acp_registry();
    assert_eq!(registry, acp_agent_registry::built_in_agent_specs());
    assert!(registry.contains_key("qwen"), "unmapped ACP agents must remain visible");
}

#[test]
fn cc_apps_acp_projection_preserves_registry_review_gate() {
    for name in acp_registry().keys() {
        assert_eq!(reviewed_command(name), acp_agent_registry::reviewed_agent_command(name));
    }
    assert!(reviewed_command("claude-legacy").is_err());
    assert!(reviewed_command("arbitrary-command").is_err());
    assert!(verify_reviewed_executable("pi", "pi").is_err());
    assert!(verify_reviewed_executable("arbitrary-command", "anything").is_err());
}
