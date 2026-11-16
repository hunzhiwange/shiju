// Tests migrated from desktop host state; production types retain their original owners.
mod behavior_0 {
use crate::system_settings_themes::state::*;
#[test]
fn themes_settings_default_opens_list_with_basics_editor_tab() {
    let state = ThemesSettingsState::default();
    assert_eq!(state.mode, ThemesSettingsMode::List);
    assert_eq!(state.editor_tab, ThemeEditorTab::Basics);
    assert!(state.entries.is_empty());
    assert_eq!(state.page_index, 0);
    assert!(state.draft.is_none());
    assert!(state.error.is_none());
    assert!(state.color_picker_target.is_none());
}
}
