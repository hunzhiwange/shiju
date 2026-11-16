use crate::{
    hooks::{Effect, update},
    messages::HooksMessage,
    state::HooksSettingsState,
};
#[test]
fn plugin_hooks_toggle_persists_config() {
    let mut state = HooksSettingsState::default();
    state.plugins_enabled = true;
    state.save_error = Some("old".into());
    assert!(matches!(
        update(&mut state, HooksMessage::PluginHooksToggled(false)),
        Effect::Save { plugins_enabled: false, .. }
    ));
    assert!(!state.plugins_enabled);
    assert!(state.save_error.is_none());
}
