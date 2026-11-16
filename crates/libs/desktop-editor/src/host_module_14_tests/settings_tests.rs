use crate::message::editor::EditorMessage;
use iced::Theme;
use iced_code_editor::i18n::Language;
struct ThemeFixture {
    editor_follow_system_theme: bool,
    app_theme: Theme,
    editor_theme: Theme,
}
impl Default for ThemeFixture {
    fn default() -> Self {
        Self {
            editor_follow_system_theme: false,
            app_theme: Theme::Light,
            editor_theme: Theme::Dark,
        }
    }
}
fn editor_message(message: EditorMessage) -> EditorMessage {
    message
}
/// 设置面板控件产生的局部交互。
enum SettingsControl {
    /// 关闭设置面板。
    Close,
    /// 更新编辑器字号。
    FontSize(f32),
    /// 切换自动行高。
    AutoLineHeight(bool),
    /// 更新手动行高。
    LineHeight(f32),
    /// 更新编辑器语言。
    Language(iced_code_editor::i18n::Language),
    /// 切换跟随系统主题。
    FollowSystemTheme(bool),
    /// 更新手动主题。
    Theme(iced::Theme),
}

/// 将设置面板的局部交互映射为应用消息。
fn control_message(control: SettingsControl) -> EditorMessage {
    let message = match control {
        SettingsControl::Close => EditorMessage::ToggleSettings,
        SettingsControl::FontSize(value) => EditorMessage::FontSizePreview(value),
        SettingsControl::AutoLineHeight(enabled) => EditorMessage::ToggleAutoLineHeight(enabled),
        SettingsControl::LineHeight(value) => EditorMessage::LineHeightPreview(value),
        SettingsControl::Language(language) => EditorMessage::LanguageChanged(language),
        SettingsControl::FollowSystemTheme(enabled) => {
            EditorMessage::ToggleFollowSystemTheme(enabled)
        }
        SettingsControl::Theme(theme) => EditorMessage::ThemeChanged(theme),
    };

    message
}

/// 返回设置面板当前应显示的编辑器主题。
fn selected_editor_theme(app: &ThemeFixture) -> iced::Theme {
    if app.editor_follow_system_theme { app.app_theme.clone() } else { app.editor_theme.clone() }
}

/// 格式化字号徽标。
fn font_size_badge_label(font_size: f32) -> String {
    format!("{font_size:.0}px")
}

/// 格式化行高徽标。
fn line_height_badge_label(line_height: f32) -> String {
    format!("{line_height:.1}px")
}

#[test]
fn close_control_maps_to_toggle_settings() {
    assert!(matches!(
        editor_message(control_message(SettingsControl::Close)),
        EditorMessage::ToggleSettings
    ));
}

#[test]
fn numeric_controls_preserve_slider_values() {
    match editor_message(control_message(SettingsControl::FontSize(17.25))) {
        EditorMessage::FontSizePreview(value) => assert_eq!(value, 17.25),
        other => panic!("expected font size change, got {other:?}"),
    }

    match editor_message(control_message(SettingsControl::LineHeight(28.5))) {
        EditorMessage::LineHeightPreview(value) => assert_eq!(value, 28.5),
        other => panic!("expected line height change, got {other:?}"),
    }
}

#[test]
fn boolean_controls_preserve_enabled_state() {
    assert!(matches!(
        editor_message(control_message(SettingsControl::AutoLineHeight(true))),
        EditorMessage::ToggleAutoLineHeight(true)
    ));
    assert!(matches!(
        editor_message(control_message(SettingsControl::AutoLineHeight(false))),
        EditorMessage::ToggleAutoLineHeight(false)
    ));
    assert!(matches!(
        editor_message(control_message(SettingsControl::FollowSystemTheme(true))),
        EditorMessage::ToggleFollowSystemTheme(true)
    ));
    assert!(matches!(
        editor_message(control_message(SettingsControl::FollowSystemTheme(false))),
        EditorMessage::ToggleFollowSystemTheme(false)
    ));
}

#[test]
fn language_control_preserves_selected_language() {
    assert!(matches!(
        editor_message(control_message(SettingsControl::Language(Language::ChineseSimplified))),
        EditorMessage::LanguageChanged(Language::ChineseSimplified)
    ));
}

#[test]
fn theme_control_preserves_selected_theme() {
    assert!(matches!(
        editor_message(control_message(SettingsControl::Theme(Theme::Dark))),
        EditorMessage::ThemeChanged(Theme::Dark)
    ));
}

#[test]
fn selected_editor_theme_uses_manual_theme_when_system_following_is_disabled() {
    let mut app = ThemeFixture::default();
    app.editor_follow_system_theme = false;
    app.app_theme = Theme::Light;
    app.editor_theme = Theme::Dark;

    assert!(matches!(selected_editor_theme(&app), Theme::Dark));
}

#[test]
fn selected_editor_theme_uses_app_theme_when_system_following_is_enabled() {
    let mut app = ThemeFixture::default();
    app.editor_follow_system_theme = true;
    app.app_theme = Theme::Dark;
    app.editor_theme = Theme::Light;

    assert!(matches!(selected_editor_theme(&app), Theme::Dark));
}

#[test]
fn badge_labels_use_declared_precision_at_slider_boundaries() {
    assert_eq!(font_size_badge_label(10.0), "10px");
    assert_eq!(font_size_badge_label(29.6), "30px");
    assert_eq!(line_height_badge_label(10.0), "10.0px");
    assert_eq!(line_height_badge_label(49.95), "50.0px");
}
