use vw_api_app_identity::{
    AppHostPreferences,
    AppId,
    AppPreferencesCapabilities,
    AppPreferencesResult,
    AppUserPreferences,
    AppWslPreference,
};
use super::preferences::*;
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
