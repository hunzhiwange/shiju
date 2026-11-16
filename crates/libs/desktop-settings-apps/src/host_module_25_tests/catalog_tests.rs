use super::page_fixture::{AppsCatalogMessage, AppsCatalogState, catalog_route};
use crate::system_settings_apps::catalog::*;
use desktop_apps_ui::AgentAppsContext;
use gateway_http::AgentAppsReadError;
use vw_api_app_identity::{
    AppCatalogEntry, AppDesktopObservation, AppId, AppInstallation, AppInstallationEnvironment,
    AppInstallationId, AppInstallationProbeFailure, AppInstallationRuntimeStatus,
    AppInstallationSource, AppReadResponse, AppRuntimeKind,
};

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
    let (_, context) = super::lifecycle_app();
    let mut app = AppsCatalogState::default();
    let generation = app.begin(context.clone());
    let mut other = context.clone();
    other.gateway = "http://other-gateway".into();
    for (context, generation) in
        [(other, generation), (context.clone(), generation.wrapping_sub(1))]
    {
        let _ = catalog_route(
            &mut app,
            AppsCatalogMessage::LocalReady {
                context,
                generation,
                result: Ok(local_app_auth::Credentials { token: "stale-local-credential".into() }),
            },
        );
        assert!(app.token.is_empty());
    }
    let _ = catalog_route(
        &mut app,
        AppsCatalogMessage::LocalReady {
            context,
            generation,
            result: Ok(local_app_auth::Credentials { token: "current-local-credential".into() }),
        },
    );
    assert_eq!(app.host, local_app_auth::HOST_ID);
    assert_eq!(app.token, "current-local-credential");
    assert!(app.loading);
    assert!(!app.connection_open);
}

#[test]
fn catalog_cache_expires_and_invalidates_after_failure_or_context_change() {
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
    let (_, context) = super::lifecycle_app();
    let mut app = AppsCatalogState::default();
    app.host = "remote".into();
    app.token = "fixture".into();
    let generation = app.begin(context.clone());
    assert!(app.finish(&context, generation, Ok(vec![row()])));
    let _ = catalog_route(&mut app, AppsCatalogMessage::EnsureLoaded);
    assert!(!app.loading);
    assert_eq!(app.generation, generation);
    assert_eq!(app.rows.len(), 1);
    let _ = catalog_route(&mut app, AppsCatalogMessage::Refresh);
    assert!(app.loading);
    assert!(!app.cache_is_fresh());
    let generation = app.generation;
    assert!(app.finish(&context, generation, Ok(vec![row()])));
    let _ = catalog_route(&mut app, AppsCatalogMessage::TokenInput("replacement".into()));
    assert!(!app.cache_is_fresh());
    assert!(app.rows.is_empty());
    let generation = app.begin(context.clone());
    assert!(app.finish(&context, generation, Ok(vec![row()])));
    let _ = catalog_route(&mut app, AppsCatalogMessage::HostInput("other".into()));
    assert!(!app.cache_is_fresh());
    assert!(app.rows.is_empty());
}
