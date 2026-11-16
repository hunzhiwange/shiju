use crate::system_settings_apps::preferences::*;
use vw_api_app_identity::{
    AppCatalogEntry, AppHostPreferences, AppId, AppPreferencesResult, AppReadResponse,
    AppUserPreferences,
};
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
fn cc_apps_desktop_preferences_state_list_uses_saved_order_and_visibility() {
    use crate::system_settings_apps::catalog::CatalogRow;
    let rows = [AppId::Claude, AppId::Codex, AppId::Gemini]
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
        .collect::<Vec<_>>();
    let mut saved = value();
    saved.user.app_order = vec!["codex".into(), "claude".into(), "gemini".into()];
    saved.user.visible_apps = vec![AppId::Codex, AppId::Claude];
    let mut preferences = PreferencesState::default();
    preferences.saved = Some(saved.clone());
    preferences.draft = Some(value());
    assert_eq!(
        visible_rows(&crate::system_settings_apps::ViewState {
            rows: &rows,
            preferences: &preferences,
            lifecycle: &Default::default(),
            drag: &Default::default(),
            query: "",
            host: "",
            loading: false,
            error: &None,
            expanded: None,
            hovered: None
        })
        .iter()
        .map(|r| r.entry.id)
        .collect::<Vec<_>>(),
        vec![AppId::Codex, AppId::Claude]
    );
    saved.user.visible_apps = vec![AppId::Gemini];
    preferences.busy = true;
    assert!(preferences.finish(0, Ok(saved)));
    assert_eq!(
        visible_rows(&crate::system_settings_apps::ViewState {
            rows: &rows,
            preferences: &preferences,
            lifecycle: &Default::default(),
            drag: &Default::default(),
            query: "",
            host: "",
            loading: false,
            error: &None,
            expanded: None,
            hovered: None
        })
        .iter()
        .map(|r| r.entry.id)
        .collect::<Vec<_>>(),
        vec![AppId::Gemini]
    );
}
