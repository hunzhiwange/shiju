use super::acp_projection::*;
use vw_api_types::agent_apps::{AppId, AppRuntimeKind};

#[test]
fn cc_apps_acp_projection_preserves_registry_entries_and_legacy_view() {
    let projection = acp_registry();
    assert_eq!(projection.len(), 22);
    assert_eq!(projection, vw_acp::built_in_agent_specs());
    assert_eq!(legacy_registry(), vw_acp::built_in_agent_registry());
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
    assert_eq!(acp_registry().len(), 22, "unmapped ACP agents must remain visible");
}

#[test]
fn cc_apps_acp_projection_preserves_registry_review_gate() {
    for name in acp_registry().keys() {
        assert_eq!(reviewed_command(name), vw_acp::reviewed_agent_command(name));
    }
    assert!(reviewed_command("claude-legacy").is_err());
    assert!(reviewed_command("arbitrary-command").is_err());
    assert!(verify_reviewed_executable("pi", "pi").is_err());
    assert!(verify_reviewed_executable("arbitrary-command", "anything").is_err());
}
