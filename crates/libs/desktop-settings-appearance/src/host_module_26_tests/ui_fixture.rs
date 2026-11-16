//! Borrowed domain views and real reducers for appearance rendering tests.
use crate::system_settings_themes::{
    self as themes, messages::ThemesMessage, state::ThemesSettingsState,
};
pub use desktop_editor::components::editor_settings_fields::Message as EditorAction;
use iced::{Element, Theme};
pub type EditorMessage = EditorAction<Box<Message>>;
#[derive(Debug, Clone)]
pub enum Message {
    None,
    CopyCode(String),
    AppendText(String),
    OpenUrl(String),
    General(crate::general::Action<desktop_terminal::shell::Shell>),
    Editor(EditorMessage),
    Settings(SettingsMessage),
}
#[derive(Debug, Clone)]
pub enum SettingsMessage {
    Themes(ThemesMessage),
}
pub struct Fixture {
    pub general_settings_tab: desktop_ui_controls::settings_navigation::GeneralSettingsTab,
    pub chat_timezone: String,
    pub terminal: Terminal,
    pub preview_auto_save_mode: config_ui::PreviewAutoSaveMode,
    pub desktop_locale: config_ui::DesktopLocale,
    pub app_theme: Theme,
    pub editor_theme: Theme,
    pub editor_follow_system_theme: bool,
    pub current_font_size: f32,
    pub current_line_height: f32,
    pub auto_adjust_line_height: bool,
    pub themes_settings: ThemesSettingsState,
    pub active_custom_theme_id: Option<String>,
    pub active_theme_skin: Option<desktop_theme::ActiveThemeSkin>,
}
impl Default for Fixture {
    fn default() -> Self {
        Self {
            general_settings_tab: desktop_ui_controls::settings_navigation::GeneralSettingsTab::Interface,
            chat_timezone: "Asia/Shanghai".into(),
            terminal: Terminal::default(),
            preview_auto_save_mode: config_ui::PreviewAutoSaveMode::Off,
            desktop_locale: config_ui::DesktopLocale::SimplifiedChinese,
            app_theme: Theme::Light,
            editor_theme: Theme::Light,
            editor_follow_system_theme: true,
            current_font_size: 15.0,
            current_line_height: 24.0,
            auto_adjust_line_height: true,
            themes_settings: Default::default(),
            active_custom_theme_id: None,
            active_theme_skin: None,
        }
    }
}
impl desktop_chat_view::message_view::markdown_selection::SelectionMessage for Message {
    fn none() -> Self {
        Self::None
    }
    fn copy(text: String) -> Self {
        Self::CopyCode(text)
    }
    fn context(effect: desktop_ui_controls::text_editor_context_menu::TextContextEffect) -> Self {
        match effect {
            desktop_ui_controls::text_editor_context_menu::TextContextEffect::Copy(text) => {
                Self::CopyCode(text)
            }
            desktop_ui_controls::text_editor_context_menu::TextContextEffect::Append(text) => {
                Self::AppendText(text)
            }
            desktop_ui_controls::text_editor_context_menu::TextContextEffect::OpenUrl(url) => {
                Self::OpenUrl(url)
            }
        }
    }
    fn selection(_: String, _: usize, _: usize, _: Option<String>) -> Self {
        Self::None
    }
}
pub fn help<'a>(
    title: &'a str,
    body: std::borrow::Cow<'a, str>,
    color: Option<iced::Color>,
    i18n: desktop_i18n::I18n,
) -> Element<'a, Message> {
    desktop_chat_view::message_view::markdown_selection::selectable_titled_text(
        title, body, color, i18n,
    )
}
pub fn inputs(app: &Fixture) -> themes::View<'_, Message> {
    themes::View {
        desktop_locale: app.desktop_locale,
        app_theme: app.app_theme,
        themes_settings: &app.themes_settings,
        active_custom_theme_id: &app.active_custom_theme_id,
        map: |v| Message::Settings(SettingsMessage::Themes(v)),
        none: Message::None,
        help,
    }
}
pub fn editor_fields(app: &Fixture, width: f32) -> Element<'_, Message> {
    use desktop_editor::components::editor_settings_fields::{self as fields, SettingsView};
    let i18n = desktop_i18n::I18n::new(app.desktop_locale);
    let color = app.app_theme.palette().text;
    fields::view(
        SettingsView {
            locale: app.desktop_locale,
            editor_theme: if app.editor_follow_system_theme {
                app.app_theme.clone()
            } else {
                app.editor_theme.clone()
            },
            follow_system_theme: app.editor_follow_system_theme,
            font_size: app.current_font_size,
            line_height: app.current_line_height,
            auto_line_height: app.auto_adjust_line_height,
            language: iced_code_editor::i18n::Language::default(),
            #[cfg(not(target_arch = "wasm32"))]
            lsp_disabled: false,
        },
        width,
        move |title, body| help(title, body, Some(color), i18n).map(Box::new),
    )
    .map(|v| match v {
        EditorAction::Help(message) => *message,
        v => Message::Editor(v),
    })
}
pub fn view(app: &Fixture) -> Element<'_, Message> {
    crate::editor::view(app.desktop_locale, app.app_theme, editor_fields(app, 250.0), help)
}
pub mod editor {
    pub use crate::system_settings_themes::editor::*;
    pub fn view(app: &super::Fixture) -> iced::Element<'_, super::Message> {
        crate::system_settings_themes::editor::view(&super::inputs(app))
    }
}
pub mod gallery {
    pub use crate::system_settings_themes::gallery::*;
    pub fn view(app: &super::Fixture) -> iced::Element<'_, super::Message> {
        crate::system_settings_themes::gallery::view(&super::inputs(app))
    }
}
pub fn handle_theme_message(app: &mut Fixture, messages: Vec<Message>) {
    let [Message::Settings(SettingsMessage::Themes(message))] = messages.as_slice() else {
        panic!("expected a theme action, got {messages:?}");
    };
    let mut update = themes::update::Update {
        themes_settings: &mut app.themes_settings,
        active_custom_theme_id: &mut app.active_custom_theme_id,
        active_theme_skin: &mut app.active_theme_skin,
        app_theme: &mut app.app_theme,
        effects: Vec::new(),
    };
    let _ = themes::update::update(&mut update, message.clone());
}

pub struct Terminal {
    pub shell: desktop_terminal::shell::Shell,
    pub font_family: String,
    pub font_size: f32,
}
impl Default for Terminal {
    fn default() -> Self {
        Self {
            shell: desktop_terminal::shell::Shell::all()[0],
            font_family: "JetBrains Mono".into(),
            font_size: 14.0,
        }
    }
}
pub fn general_view(app: &Fixture) -> Element<'_, Message> {
    let inputs = crate::general::View {
        desktop_locale: app.desktop_locale,
        app_theme: app.app_theme,
        shell: app.terminal.shell,
        shell_options: desktop_terminal::shell::Shell::all().to_vec(),
        font_family: &app.terminal.font_family,
        displayed_font_size: app.terminal.font_size,
        preview_auto_save_mode: app.preview_auto_save_mode,
        chat_timezone: &app.chat_timezone,
        launch_at_login: false,
        launch_at_login_busy: false,
        close_to_tray: false,
        task_completion_notification: false,
        notification_supported: true,
        general_settings_tab: app.general_settings_tab,
    };
    let page = match app.general_settings_tab {
        desktop_ui_controls::settings_navigation::GeneralSettingsTab::Editor => crate::editor::content(
            app.desktop_locale,
            app.app_theme,
            editor_fields(app, 250.0),
            help,
        ),
        desktop_ui_controls::settings_navigation::GeneralSettingsTab::Themes => {
            crate::system_settings_themes::gallery::view(&self::inputs(app))
        }
        _ => iced::widget::Space::new().into(),
    };
    crate::general::view(&inputs, Message::General, help, page)
}
pub fn preview_overlay<'a>(app: &'a Fixture, base: Element<'a, Message>) -> Element<'a, Message> {
    let i18n = desktop_i18n::I18n::new(app.desktop_locale);
    desktop_editor::components::preview_panel::view::settings::build_settings_overlay(
        i18n,
        help(
            i18n.__("编辑器设置"),
            i18n.__("统一调整字号、行高、语言与主题。").into(),
            Some(app.app_theme.palette().text),
            i18n,
        ),
        editor_fields(app, 240.0),
        Message::None,
        base,
    )
}

pub mod system_settings_themes {
    pub fn view(app: &super::Fixture) -> iced::Element<'_, super::Message> {
        crate::system_settings_themes::gallery::view(&super::inputs(app))
    }
}
