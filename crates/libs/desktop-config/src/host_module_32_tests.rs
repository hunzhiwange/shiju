//! Configuration ownership and project read isolation.
#[cfg(not(target_arch = "wasm32"))]
mod gateway;

use crate::config_desktop::{DesktopPreferencesPersistence, preferences_scope};
use crate::config_project_chat::{ProjectChatPreferences, ProjectPreferencesScope};
use gateway_endpoint::GatewayEndpoint;
use serde_json::json;

fn endpoint() -> GatewayEndpoint {
    crate::config_gateway::gateway_client_endpoint_from_config(&Default::default())
}

#[test]
fn late_desktop_load_preserves_edited_fields_only_in_its_scope() {
    let mut state = DesktopPreferencesPersistence::default();
    let scope = preferences_scope(&endpoint());
    state.edited_desktop_fields.entry(scope.clone()).or_default().insert("theme".into());
    let mut config = json!({"theme": "old", "model": "auto"});
    state.retain_unedited_desktop_preferences(&scope, &mut config);
    assert_eq!(config, json!({"model": "auto"}));
    assert!(state.desktop_preference_edited(&scope, "theme"));
    assert!(!state.desktop_preference_edited(&scope, "model"));
}

#[test]
fn project_read_rejects_old_revision_and_other_project() {
    let endpoint = endpoint();
    let mut state = ProjectChatPreferences::default();
    let scope = ProjectPreferencesScope::new(&endpoint, "/project".into());
    let selected = ("model".into(), false, None);
    state.invalidate_read();
    assert!(
        state
            .loaded(&endpoint, Some("/project"), scope.clone(), 0, Ok(Some(selected.clone())))
            .is_none()
    );
    assert!(
        state
            .loaded(
                &endpoint,
                Some("/other"),
                scope.clone(),
                state.revision,
                Ok(Some(selected.clone()))
            )
            .is_none()
    );
    assert_eq!(
        state.loaded(
            &endpoint,
            Some("/project"),
            scope,
            state.revision,
            Ok(Some(selected.clone()))
        ),
        Some(selected)
    );
    assert!(state.selection_override);
}

#[test]
fn gateway_save_feedback_requires_active_scope_and_no_existing_error() {
    use crate::config_desktop::SystemSettingsSaveFeedback as Feedback;
    use crate::config_system_settings::SystemSettingsSection as Section;
    let state = DesktopPreferencesPersistence::default();
    let scope = preferences_scope(&endpoint());
    let other = preferences_scope(&GatewayEndpoint::new("other.invalid", 1));
    assert!(matches!(
        state.system_settings_save_feedback(&scope, &scope, Section::GatewayClient, true, false),
        Feedback::GatewaySyncFailed
    ));
    assert!(matches!(
        state.system_settings_save_feedback(&scope, &other, Section::GatewayClient, true, false),
        Feedback::None
    ));
    assert!(matches!(
        state.system_settings_save_feedback(&scope, &scope, Section::GatewayClient, true, true),
        Feedback::None
    ));
    assert!(matches!(
        state.system_settings_save_feedback(&scope, &scope, Section::DialogueFlow, false, false),
        Feedback::DialogueFlow
    ));
}

#[cfg(not(target_arch = "wasm32"))]
mod desktop;

#[cfg(not(target_arch = "wasm32"))]
mod system_settings;
