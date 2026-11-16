use crate::system_settings_themes::{self, editor::*, messages::*, state::*};
use desktop_theme::{ThemeAssets, ThemeChrome, ThemeDefinition, ThemePaletteHex};
use iced::{Element, Theme};
type Message = ThemesMessage;
fn keep_element(element: Element<'_, Message>) {
    std::hint::black_box(element);
}
fn inputs(state: &ThemesSettingsState) -> system_settings_themes::View<'_, Message> {
    system_settings_themes::View {
        desktop_locale: config_ui::DesktopLocale::SimplifiedChinese,
        app_theme: &Theme::Light,
        active_custom_theme_id: &None,
        themes_settings: state,
        map: |message| message,
        none: ThemesMessage::CancelEdit,
        help: |_, body, _, _| iced::widget::text(body).into(),
    }
}
fn test_app_with_draft(draft: ThemeEditorDraft) -> ThemesSettingsState {
    ThemesSettingsState {
        mode: ThemesSettingsMode::Edit,
        editor_tab: ThemeEditorTab::Basics,
        draft: Some(draft),
        ..Default::default()
    }
}
fn sample_custom_theme() -> ThemeDefinition {
    ThemeDefinition {
        id: "mint-night".to_string(),
        name: "Mint Night".to_string(),
        tagline: Some("Cool mint accents".to_string()),
        palette: ThemePaletteHex {
            background: "#0b1220".to_string(),
            text: "#e2e8f0".to_string(),
            primary: "#34d399".to_string(),
            success: "#22c55e".to_string(),
            warning: "#f59e0b".to_string(),
            danger: "#ef4444".to_string(),
        },
        assets: ThemeAssets::default(),
        chrome: ThemeChrome::default(),
    }
}

#[test]
fn editor_tab_buttons_build_active_and_inactive_states() {
    let state = ThemesSettingsState::default();
    for (label, tab) in editor_tab_labels() {
        let inactive = if *tab == ThemeEditorTab::Preview {
            ThemeEditorTab::Basics
        } else {
            ThemeEditorTab::Preview
        };
        keep_element(editor_tab_button(&inputs(&state), label, *tab, *tab));
        keep_element(editor_tab_button(&inputs(&state), label, *tab, inactive));
    }
}
#[test]
fn view_builds_every_editor_tab() {
    let mut app = test_app_with_draft(draft_from_definition(&sample_custom_theme()));
    for (_, tab) in editor_tab_labels() {
        app.editor_tab = *tab;
        keep_element(view(&inputs(&app)));
    }
}
#[test]
fn view_builds_with_color_picker_anchored_to_swatch() {
    let mut app = test_app_with_draft(draft_from_definition(&sample_custom_theme()));
    app.editor_tab = ThemeEditorTab::Palette;
    app.color_picker_target = Some(ThemeColorPickerTarget::Palette(ThemePaletteField::Primary));
    keep_element(view(&inputs(&app)));

    app.editor_tab = ThemeEditorTab::Chrome;
    app.color_picker_target = Some(ThemeColorPickerTarget::Chrome(ThemeChromeField::InputBg));
    keep_element(view(&inputs(&app)));
}
