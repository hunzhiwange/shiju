use super::preferences::*;
use vw_gateway_client::vw_api_types::agent_apps::*;
fn value() -> AppPreferencesResult {
    AppPreferencesResult {
        user: AppUserPreferences {
            app_order: vec!["codex".into()],
            visible_apps: vec![AppId::Codex],
        },
        host: AppHostPreferences::default(),
        restart_required: false,
        capabilities: None,
    }
}
#[test]
fn cc_apps_desktop_preferences_state_only_api_results_commit() {
    let original = value();
    let mut state = PreferencesState {
        saved: Some(original.clone()),
        draft: Some(original.clone()),
        busy: true,
        generation: 2,
        ..Default::default()
    };
    state.draft.as_mut().unwrap().host.preferred_terminal = Some("unsaved".into());
    assert!(state.finish(2, Err("明确拒绝".into())));
    assert_eq!(state.saved.as_ref(), Some(&original));
    assert_eq!(state.draft.as_ref(), Some(&original));
    assert!(!state.busy);
    assert_eq!(state.error.as_deref(), Some("明确拒绝"));
    state.busy = true;
    let mut normalized = original.clone();
    normalized.host.preferred_terminal = Some("terminal".into());
    normalized.restart_required = true;
    assert!(state.finish(2, Ok(normalized.clone())));
    assert_eq!(state.saved.as_ref(), Some(&normalized));
    assert_eq!(state.draft.as_ref(), Some(&normalized));
    assert!(!state.finish(1, Ok(original)));
    assert_eq!(state.saved.as_ref(), Some(&normalized));
}
#[test]
fn cc_apps_desktop_preferences_state_last_visible_and_busy_reject_edits() {
    let mut state = PreferencesState { draft: Some(value()), ..Default::default() };
    state.visible(AppId::Codex, false);
    assert_eq!(state.draft.as_ref().unwrap().user.visible_apps, vec![AppId::Codex]);
    assert!(state.error.as_deref().unwrap().contains("至少"));
    state.visible(AppId::Claude, true);
    state.visible(AppId::Codex, false);
    assert_eq!(state.draft.as_ref().unwrap().user.visible_apps, vec![AppId::Claude]);
    state.busy = true;
    state.visible(AppId::Codex, true);
    assert_eq!(state.draft.as_ref().unwrap().user.visible_apps, vec![AppId::Claude]);
}
#[test]
fn cc_apps_desktop_preferences_state_host_capabilities_and_invalidation() {
    let mut state = PreferencesState { draft: Some(value()), ..Default::default() };
    assert!(!state.host_editable());
    state.wsl_edit(AppId::Codex, "bash".into(), false);
    assert!(state.draft.as_ref().unwrap().host.wsl_shells.is_empty());
    state.draft.as_mut().unwrap().capabilities = Some(AppPreferencesCapabilities {
        terminals: vec!["terminal".into()],
        wsl_preferences: false,
    });
    assert!(state.host_editable());
    state.wsl_edit(AppId::Codex, "bash".into(), false);
    assert!(state.draft.as_ref().unwrap().host.wsl_shells.is_empty());
    state.draft.as_mut().unwrap().capabilities.as_mut().unwrap().wsl_preferences = true;
    state.wsl_edit(AppId::Codex, "bash".into(), false);
    state.wsl_edit(AppId::Codex, "-lc".into(), true);
    assert_eq!(
        state.draft.as_ref().unwrap().host.wsl_shells,
        vec![AppWslPreference {
            app_id: AppId::Codex,
            shell: Some("bash".into()),
            flag: Some("-lc".into())
        }]
    );
    state.wsl_edit(AppId::ClaudeDesktop, "bash".into(), false);
    assert_eq!(state.draft.as_ref().unwrap().host.wsl_shells.len(), 1);
    state.busy = true;
    let generation = state.generation;
    state.invalidate();
    assert!(!state.finish(generation, Ok(value())));
    assert!(state.saved.is_none() && state.draft.is_none() && !state.open);
}

#[test]
fn cc_apps_desktop_preferences_state_message_scopes_and_atomic_domains() {
    use crate::app::message::{SettingsMessage, settings::AppsCatalogMessage};
    let mut app = crate::app::App::new().0;
    let client = crate::app::config::gateway_client().unwrap();
    let context = crate::app::state::AgentAppsContext {
        gateway: client.endpoint().base_url(),
        tenant: client.endpoint().tenant_scope().clone(),
        project: app.project_path.clone(),
    };
    app.apps_catalog.context = Some(context.clone());
    app.apps_catalog.host = "中文主机".into();
    app.apps_catalog.token = "isolated-fixture-application-token".into();
    app.apps_catalog.preferences.saved = Some(value());
    app.apps_catalog.preferences.draft = Some(value());
    let route = |app: &mut crate::app::App, value| {
        crate::app::message::settings::update(
            app,
            SettingsMessage::AppsCatalog(AppsCatalogMessage::Preferences(value)),
        )
    };
    let _ = route(&mut app, PreferencesMessage::SaveUser);
    assert!(app.apps_catalog.preferences.busy);
    assert_eq!(app.apps_catalog.preferences.saved, Some(value()));
    let generation = app.apps_catalog.preferences.generation;
    let mut normalized = value();
    normalized.host.preferred_terminal = Some("server-normalized".into());
    let reply = |host: &str, catalog_generation, result| PreferencesMessage::Reply {
        context: context.clone(),
        host: host.into(),
        catalog_generation,
        generation,
        result,
    };
    let cg = app.apps_catalog.generation;
    let _ = route(&mut app, reply("wrong-host", cg, Ok(normalized.clone())));
    assert_eq!(app.apps_catalog.preferences.saved, Some(value()));
    let cg = app.apps_catalog.generation;
    let _ = route(&mut app, reply("中文主机", cg.wrapping_add(1), Ok(normalized.clone())));
    assert_eq!(app.apps_catalog.preferences.saved, Some(value()));
    let _ = route(&mut app, reply("中文主机", cg, Ok(normalized.clone())));
    assert_eq!(app.apps_catalog.preferences.saved, Some(normalized));
    let _ = route(&mut app, PreferencesMessage::SaveUser);
    let _ = crate::app::message::settings::update(
        &mut app,
        SettingsMessage::AppsCatalog(AppsCatalogMessage::HostInput("other".into())),
    );
    assert!(app.apps_catalog.preferences.saved.is_none());
    let _ = route(&mut app, reply("中文主机", cg, Ok(value())));
    assert!(app.apps_catalog.preferences.saved.is_none());
}

#[test]
fn cc_apps_desktop_preferences_state_list_uses_saved_order_and_visibility() {
    use super::catalog::CatalogRow;
    use crate::app::state::AppsCatalogState;
    let mut state = AppsCatalogState::default();
    state.rows = [AppId::Claude, AppId::Codex, AppId::Gemini]
        .into_iter()
        .map(|id| CatalogRow {
            entry: AppCatalogEntry {
                id,
                name: format!("{id:?}"),
                cli_command: None,
                runtime_kinds: vec![],
            },
            observation: Ok(AppReadResponse { installations: vec![], desktop: None }),
        })
        .collect();
    let mut saved = value();
    saved.user.app_order = vec!["codex".into(), "claude".into(), "gemini".into()];
    saved.user.visible_apps = vec![AppId::Codex, AppId::Claude];
    state.preferences.saved = Some(saved.clone());
    state.preferences.draft = Some(value());
    assert_eq!(
        visible_rows(&state).iter().map(|r| r.entry.id).collect::<Vec<_>>(),
        vec![AppId::Codex, AppId::Claude]
    );
    saved.user.visible_apps = vec![AppId::Gemini];
    state.preferences.busy = true;
    assert!(state.preferences.finish(0, Ok(saved)));
    assert_eq!(
        visible_rows(&state).iter().map(|r| r.entry.id).collect::<Vec<_>>(),
        vec![AppId::Gemini]
    );
}
