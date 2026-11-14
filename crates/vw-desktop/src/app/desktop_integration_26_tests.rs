//! Navigation visibility is a projection of confirmed user preferences.
//! Database reopening is covered by the gateway preferences API tests.

use super::components::system_settings_apps::{catalog::CatalogRow, preferences};
use super::state::AppsCatalogState;
use vw_gateway_client::vw_api_types::agent_apps::*;

fn catalog_state() -> AppsCatalogState {
    AppsCatalogState {
        rows: [AppId::Claude, AppId::Codex, AppId::Gemini]
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
            .collect(),
        ..Default::default()
    }
}

fn visible_ids(state: &AppsCatalogState) -> Vec<AppId> {
    preferences::visible_rows(state).iter().map(|row| row.entry.id).collect()
}

#[test]
fn desktop_integration_26_preserves_expected_behavior() {
    let confirmed = AppPreferencesResult {
        user: AppUserPreferences {
            app_order: vec!["gemini".into(), "codex".into(), "claude".into()],
            visible_apps: vec![AppId::Gemini, AppId::Codex],
        },
        host: AppHostPreferences::default(),
        restart_required: false,
        capabilities: None,
    };
    // A fresh desktop receives the same DTO from the gateway after a restart.
    let response = serde_json::to_vec(&confirmed).unwrap();
    let mut state = catalog_state();
    let catalog_before = state.rows.iter().map(|row| row.entry.clone()).collect::<Vec<_>>();
    state.preferences.busy = true;
    state.preferences.generation = 1;
    assert!(state.preferences.finish(1, Ok(serde_json::from_slice(&response).unwrap())));
    assert_eq!(visible_ids(&state), vec![AppId::Gemini, AppId::Codex]);

    // Draft edits and a rejected save must leave the confirmed projection intact.
    state.preferences.visible(AppId::Gemini, false);
    assert_eq!(visible_ids(&state), vec![AppId::Gemini, AppId::Codex]);
    state.preferences.busy = true;
    state.preferences.generation = 2;
    assert!(state.preferences.finish(2, Err("设置未保存".into())));
    assert_eq!(state.preferences.saved.as_ref(), Some(&confirmed));
    assert_eq!(state.preferences.draft.as_ref(), Some(&confirmed));

    state.preferences.visible(AppId::Gemini, false);
    state.preferences.visible(AppId::Codex, false);
    assert_eq!(state.preferences.draft.as_ref().unwrap().user.visible_apps, vec![AppId::Codex]);
    assert!(state.preferences.error.is_some());
    assert_eq!(state.preferences.draft.as_ref().unwrap().host, confirmed.host);
    assert_eq!(state.rows.iter().map(|row| row.entry.clone()).collect::<Vec<_>>(), catalog_before);

    drop(state);
    let mut restored = catalog_state();
    restored.preferences.busy = true;
    restored.preferences.generation = 1;
    assert!(restored.preferences.finish(1, Ok(serde_json::from_slice(&response).unwrap())));
    assert_eq!(visible_ids(&restored), vec![AppId::Gemini, AppId::Codex]);
    assert_eq!(restored.rows.len(), 3, "hidden applications remain in the business catalog");
}
