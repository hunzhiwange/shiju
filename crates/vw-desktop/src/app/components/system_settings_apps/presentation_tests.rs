use super::{catalog::CatalogRow, presentation::*};
use vw_gateway_client::{AgentAppsReadError, vw_api_types::agent_apps::*};
fn app() -> CatalogRow {
    CatalogRow {
        entry: AppCatalogEntry {
            id: AppId::Codex,
            name: "Codex 工具".into(),
            cli_command: Some("codex".into()),
            runtime_kinds: vec![AppRuntimeKind::NativeCli],
        },
        observation: Ok(AppReadResponse { installations: vec![], desktop: None }),
    }
}
#[test]
fn apps_presentation_groups_only_verified_absence_as_missing() {
    let mut row = app();
    assert_eq!(presence(&row), Presence::Missing);
    row.observation = Err(AgentAppsReadError::Transport("offline".into()));
    assert_eq!(presence(&row), Presence::Unverified);
    row.entry.runtime_kinds = vec![AppRuntimeKind::DesktopApp];
    row.observation = Ok(AppReadResponse {
        installations: vec![],
        desktop: Some(AppDesktopObservation {
            supported: true,
            app_bundles: Some(vec!["/Applications/工具.app".into()]),
            config_directories: vec![],
        }),
    });
    assert_eq!(presence(&row), Presence::Installed);
    row.observation.as_mut().unwrap().desktop.as_mut().unwrap().app_bundles = None;
    assert_eq!(presence(&row), Presence::Unverified);
}
#[test]
fn apps_presentation_search_matches_names_and_commands_without_case_or_whitespace() {
    let row = app();
    assert!(matches_query(&row, " CODEX "));
    assert!(matches_query(&row, "工具"));
    assert!(matches_query(&row, ""));
    assert!(!matches_query(&row, "claude"));
}
