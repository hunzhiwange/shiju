use iced::Theme;
#[test]
fn theme_helpers_select_app_editor_and_pet_themes() {
    let mut state = crate::theme_state::State {
        app_theme: Theme::Dark,
        editor_theme: Theme::Light,
        editor_follow_system_theme: true,
        active_custom_theme_id: None,
        active_theme_skin: None,
    };
    assert_eq!(state.app_theme.to_string(), Theme::Dark.to_string());
    state.editor_follow_system_theme = true;
    assert_eq!(state.effective_editor_theme().to_string(), Theme::Dark.to_string());
    assert_eq!(state.effective_editor_theme().to_string(), Theme::Dark.to_string());

    state.editor_follow_system_theme = false;
    assert_eq!(state.effective_editor_theme().to_string(), Theme::Light.to_string());
    assert_eq!(state.effective_editor_theme().to_string(), Theme::Light.to_string());
}
