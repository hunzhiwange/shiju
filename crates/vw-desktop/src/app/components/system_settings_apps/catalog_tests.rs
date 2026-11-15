use vw_api_app_identity::{
    AppCatalogEntry,
    AppDesktopObservation,
    AppId,
    AppInstallation,
    AppInstallationEnvironment,
    AppInstallationId,
    AppInstallationProbeFailure,
    AppInstallationRuntimeStatus,
    AppInstallationSource,
    AppReadResponse,
    AppRuntimeKind,
};
use super::catalog::*;
use {gateway_http::AgentAppsReadError, vw_api_types::agent_apps::*};

fn row() -> CatalogRow {
    CatalogRow {
        entry: AppCatalogEntry {
            id: AppId::Codex,
            name: "中文应用".into(),
            cli_command: Some("codex".into()),
            runtime_kinds: vec![AppRuntimeKind::NativeCli, AppRuntimeKind::AcpAdapter],
        },
        observation: Ok(AppReadResponse { installations: vec![], desktop: None }),
    }
}





#[test]
fn cc_apps_desktop_catalog_state_retry_and_stale_response() {
    use crate::app::state::{AgentAppsContext, AppsCatalogState};
    let context = AgentAppsContext {
        gateway: "http://gateway".into(),
        tenant: vw_api_transport::TenantScope::new("p", "c").unwrap(),
        project: None,
    };
    let mut state = AppsCatalogState::default();
    let first = state.begin(context.clone());
    assert!(state.finish(&context, first, Err("远端离线".into())));
    let second = state.begin(context.clone());
    assert!(state.loading && state.error.is_none());
    assert!(!state.finish(&context, first, Ok(vec![row()])));
    assert!(state.finish(&context, second, Ok(vec![row()])));
    assert_eq!(state.rows.len(), 1);
    assert!(
        !format!(
            "{:?}",
            AppsCatalogState { token: "secret-never-log".into(), ..Default::default() }
        )
        .contains("secret")
    );
}





#[test]
fn cc_apps_desktop_catalog_state_context_switch_releases_pending_load() {
    use crate::app::state::{AgentAppsContext, AppsCatalogState};
    let old = AgentAppsContext {
        gateway: "old".into(),
        tenant: vw_api_transport::TenantScope::new("p", "c").unwrap(),
        project: None,
    };
    let current = AgentAppsContext { gateway: "new".into(), ..old.clone() };
    let mut state = AppsCatalogState { token: "do-not-forward".into(), ..Default::default() };
    let generation = state.begin(old.clone());
    assert!(state.reset_if_context_changed(&current));
    assert!(!state.loading);
    assert!(state.token.is_empty() && state.rows.is_empty() && !state.connection_open);
    assert!(!state.finish(&old, generation, Ok(vec![row()])));
    let retry = state.begin(current.clone());
    assert!(state.finish(&current, retry, Ok(vec![row()])));
    assert_eq!(state.rows.len(), 1);
}

#[test]
fn local_apps_credentials_only_apply_to_current_connection_and_generation() {
    use crate::app::{
        App,
        message::{SettingsMessage, settings::AppsCatalogMessage},
        state::AgentAppsContext,
    };
    let mut app = App::new().0;
    let client = crate::app::config::gateway_client().unwrap();
    let context = AgentAppsContext {
        gateway: client.endpoint().base_url(),
        tenant: client.endpoint().tenant_scope().clone(),
        project: app.project_path.clone(),
    };
    let generation = app.apps_catalog.begin(context.clone());
    let mut other = context.clone();
    other.gateway = "http://other-gateway".into();
    for (context, generation) in
        [(other, generation), (context.clone(), generation.wrapping_sub(1))]
    {
        let _ = app.update(crate::app::Message::Settings(SettingsMessage::AppsCatalog(
            AppsCatalogMessage::LocalReady {
                context,
                generation,
                result: Ok(local_app_auth::Credentials {
                    token: "stale-local-credential".into(),
                }),
            },
        )));
        assert!(app.apps_catalog.token.is_empty());
    }
    let _ = app.update(crate::app::Message::Settings(SettingsMessage::AppsCatalog(
        AppsCatalogMessage::LocalReady {
            context,
            generation,
            result: Ok(local_app_auth::Credentials {
                token: "current-local-credential".into(),
            }),
        },
    )));
    assert_eq!(app.apps_catalog.host, local_app_auth::HOST_ID);
    assert_eq!(app.apps_catalog.token, "current-local-credential");
    assert!(app.apps_catalog.loading);
    assert!(!app.apps_catalog.connection_open);
}

#[test]
fn catalog_cache_expires_and_invalidates_after_failure_or_context_change() {
    use crate::app::state::{AgentAppsContext, AppsCatalogState};
    let context = AgentAppsContext {
        gateway: "http://gateway".into(),
        tenant: vw_api_transport::TenantScope::new("p", "c").unwrap(),
        project: None,
    };
    let mut state =
        AppsCatalogState { host: "host".into(), token: "fixture".into(), ..Default::default() };
    let generation = state.begin(context.clone());
    assert!(!state.cache_is_fresh());
    assert!(state.finish(&context, generation, Ok(vec![row()])));
    assert!(state.cache_is_fresh());
    state.loaded_at = Some(web_time::Instant::now() - std::time::Duration::from_secs(31));
    assert!(!state.cache_is_fresh());
    let generation = state.begin(context.clone());
    let mut failed = row();
    failed.observation = Err(AgentAppsReadError::Http { status: 503, body: serde_json::json!({}) });
    assert!(state.finish(&context, generation, Ok(vec![failed])));
    assert!(!state.cache_is_fresh());
    let generation = state.begin(context.clone());
    assert!(state.finish(&context, generation, Ok(vec![])));
    assert!(state.cache_is_fresh());
    let other = AgentAppsContext { gateway: "http://other".into(), ..context };
    assert!(state.reset_if_context_changed(&other));
    assert!(!state.cache_is_fresh());
}

#[test]
fn catalog_reopening_reuses_cache_but_manual_refresh_and_credentials_invalidate_it() {
    use crate::app::{
        App,
        message::{SettingsMessage, settings::AppsCatalogMessage},
        state::AgentAppsContext,
    };
    let mut app = App::new().0;
    let client = crate::app::config::gateway_client().unwrap();
    let context = AgentAppsContext {
        gateway: client.endpoint().base_url(),
        tenant: client.endpoint().tenant_scope().clone(),
        project: app.project_path.clone(),
    };
    app.apps_catalog.host = "remote".into();
    app.apps_catalog.token = "fixture".into();
    let generation = app.apps_catalog.begin(context.clone());
    assert!(app.apps_catalog.finish(&context, generation, Ok(vec![row()])));
    let _ = app.update(crate::app::Message::Settings(SettingsMessage::SystemTabSelected(
        crate::app::components::system_settings::SystemTab::Apps,
    )));
    assert!(!app.apps_catalog.loading);
    assert_eq!(app.apps_catalog.generation, generation);
    assert_eq!(app.apps_catalog.rows.len(), 1);
    let _ = app.update(crate::app::Message::Settings(SettingsMessage::AppsCatalog(
        AppsCatalogMessage::Refresh,
    )));
    assert!(app.apps_catalog.loading);
    assert!(!app.apps_catalog.cache_is_fresh());
    let generation = app.apps_catalog.generation;
    assert!(app.apps_catalog.finish(&context, generation, Ok(vec![row()])));
    let _ = app.update(crate::app::Message::Settings(SettingsMessage::AppsCatalog(
        AppsCatalogMessage::TokenInput("replacement".into()),
    )));
    assert!(!app.apps_catalog.cache_is_fresh());
    assert!(app.apps_catalog.rows.is_empty());
    let generation = app.apps_catalog.begin(context.clone());
    assert!(app.apps_catalog.finish(&context, generation, Ok(vec![row()])));
    let _ = app.update(crate::app::Message::Settings(SettingsMessage::AppsCatalog(
        AppsCatalogMessage::HostInput("other".into()),
    )));
    assert!(!app.apps_catalog.cache_is_fresh());
    assert!(app.apps_catalog.rows.is_empty());
}
