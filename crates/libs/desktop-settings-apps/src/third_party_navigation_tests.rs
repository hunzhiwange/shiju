use crate::system_settings_apps::catalog::CatalogRow;
use crate::third_party_navigation::*;
use desktop_provider_apps::AgentAppsContext;
use vw_api_app_identity::{AppCatalogEntry, AppId, AppReadResponse};

fn context() -> AgentAppsContext {
    AgentAppsContext {
        gateway: "http://gateway".into(),
        tenant: vw_api_transport::TenantScope::new("p", "c").unwrap(),
        project: None,
    }
}

fn rows() -> Vec<CatalogRow> {
    [AppId::OpenClaw, AppId::Codex]
        .into_iter()
        .map(|id| CatalogRow {
            entry: AppCatalogEntry {
                id,
                name: "应用".into(),
                cli_command: None,
                runtime_kinds: vec![],
            },
            observation: Ok(AppReadResponse { installations: vec![], desktop: None }),
        })
        .collect()
}

#[test]
fn third_party_task_01_scope_and_navigation() {
    let mut state = State::default();
    let rows = rows();
    state.sync_catalog(Some(&context()), "host", 1, &rows);
    state.open_app(AppId::OpenClaw, AppPage::Memory, &rows).unwrap();
    let old = state.scope().unwrap();
    state.open_resource(ResourcePage::Mcp).unwrap();
    assert_eq!(state.selected_app(), Some(AppId::OpenClaw));
    assert!(!state.accepts(&old));
    state.return_to_app(&rows).unwrap();
    assert_eq!(state.page(), Page::App { app: AppId::OpenClaw, page: AppPage::Memory });
    assert!(!state.accepts(&old));
    state.open_app(AppId::Codex, AppPage::Overview, &rows).unwrap();
    assert_eq!(state.page(), Page::App { app: AppId::Codex, page: AppPage::Overview });
}

#[test]
fn third_party_task_01_rejects_unsupported_and_absent_apps() {
    let mut state = State::default();
    let rows = rows();
    assert_eq!(state.open_resource(ResourcePage::Skills), Err(NavigationError::ContextUnavailable));
    state.sync_catalog(Some(&context()), "host", 1, &rows);
    assert_eq!(
        state.open_app(AppId::Codex, AppPage::Workspace, &rows),
        Err(NavigationError::UnsupportedPage)
    );
    assert_eq!(
        state.open_app(AppId::Hermes, AppPage::Overview, &rows),
        Err(NavigationError::AppUnavailable)
    );
    assert_eq!(state.page(), Page::Agents);
    assert_eq!(state.selected_app(), None);
}

#[test]
fn third_party_task_01_invalidates_context_catalog_and_removed_selection() {
    let mut state = State::default();
    let rows = rows();
    let mut context = context();
    state.sync_catalog(Some(&context), "host", 1, &rows);
    state.open_app(AppId::Codex, AppPage::Overview, &rows).unwrap();
    let old = state.scope().unwrap();
    state.sync_catalog(Some(&context), "host", 2, &rows);
    assert!(!state.accepts(&old));
    assert_eq!(state.selected_app(), Some(AppId::Codex));
    context.tenant = vw_api_transport::TenantScope::new("other", "c").unwrap();
    let old = state.scope().unwrap();
    state.sync_catalog(Some(&context), "host", 2, &rows);
    assert!(!state.accepts(&old));
    assert_eq!(state.page(), Page::Agents);
    state.open_app(AppId::Codex, AppPage::Overview, &rows).unwrap();
    state.sync_catalog(Some(&context), "other-host", 2, &rows);
    assert_eq!(state.selected_app(), None);
    state.open_app(AppId::Codex, AppPage::Overview, &rows).unwrap();
    state.open_resource(ResourcePage::Usage).unwrap();
    state.sync_catalog(Some(&context), "other-host", 3, &[]);
    assert_eq!(state.page(), Page::Resource(ResourcePage::Usage));
    assert_eq!(state.return_to_app(&rows), Err(NavigationError::NoSelectedApp));
}
