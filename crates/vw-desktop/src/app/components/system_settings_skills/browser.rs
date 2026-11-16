use crate::app::assets::Icon;
#[cfg(test)]
use crate::app::state::SkillsDirectoryScope;
use crate::app::{App, Message, message::SettingsMessage};
use config_ui::DesktopLocale;
use desktop_settings_skills::{BrowserContext, BrowserState, Event};
use iced::Element;
fn context(app: &App) -> BrowserContext<'_> {
    let s = &app.skills_settings;
    BrowserContext {
        help,
        desktop_locale: app.general_settings.desktop_locale,
        app_theme: &app.appearance.app_theme,
        project_path: &app.navigation_tabs.project_path,
        recent_projects: &app.workspace_state.recent_projects,
        skills_settings: BrowserState {
            directory_provider: s.directory_provider,
            query: &s.query,
            directory_scope: s.directory_scope,
            loading: s.loading,
            catalog: &s.catalog,
            selected_skill_id: &s.selected_skill_id,
            selected_skill_detail: &s.selected_skill_detail,
            detail_loading: s.detail_loading,
            detail_error: &s.detail_error,
            status_message: &s.status_message,
            status_message_translation: &s.status_message_translation,
            status_is_error: s.status_is_error,
        },
    }
}
pub(super) fn message(event: Event) -> Message {
    Message::Settings(match event {
        Event::PluginsDeleteRequested(v) => SettingsMessage::PluginsDeleteRequested(v),
        Event::PluginsSelected(v) => SettingsMessage::PluginsSelected(v),
        Event::PluginsUpdateOpen(v) => SettingsMessage::PluginsUpdateOpen(v),
        Event::PluginsRefresh => SettingsMessage::PluginsRefresh,
        Event::PluginsInstallOpen => SettingsMessage::PluginsInstallOpen,
        Event::PluginsSetEnabledRequested { plugin_id, enabled } => SettingsMessage::PluginsSetEnabledRequested { plugin_id, enabled },
        Event::AppendText(text) => return desktop_ui_controls::selectable_text::markdown_selection::SelectionMessage::context(desktop_ui_controls::text_editor_context_menu::TextContextEffect::Append(text)),
        Event::OpenUrl(url) => return desktop_ui_controls::selectable_text::markdown_selection::SelectionMessage::context(desktop_ui_controls::text_editor_context_menu::TextContextEffect::OpenUrl(url)),
        Event::Selection(session, index, text_index, text) => return desktop_ui_controls::selectable_text::markdown_selection::SelectionMessage::selection(session, index, text_index, text),
        Event::None => return Message::None,
        Event::CopyText(text) => return Message::CopyCode(text),
        Event::SkillsDirectoryScopeChanged(v) => SettingsMessage::SkillsDirectoryScopeChanged(v),
        Event::SkillsRefresh => SettingsMessage::SkillsRefresh,
        Event::SkillsInstallBuiltInRequested(v) => {
            SettingsMessage::SkillsInstallBuiltInRequested(v)
        }
        Event::SkillsSetEnabledRequested { skill_id, enabled } => {
            SettingsMessage::SkillsSetEnabledRequested { skill_id, enabled }
        }
        Event::SkillsDeleteRequested(v) => SettingsMessage::SkillsDeleteRequested(v),
        Event::SkillsDetailClosed => SettingsMessage::SkillsDetailClosed,
        Event::SkillsQueryChanged(v) => SettingsMessage::SkillsQueryChanged(v),
        Event::SkillsProjectSelected(v) => SettingsMessage::SkillsProjectSelected(v),
        Event::SkillsDetailRequested(v) => SettingsMessage::SkillsDetailRequested(v),
    })
}
pub(super) fn view(app: &App) -> iced::Element<'_, Message> {
    desktop_settings_skills::browser::view(context(app)).map(message)
}
pub(super) fn discovery_order_view(app: &App) -> iced::Element<'_, Message> {
    desktop_settings_skills::browser::discovery_order_view(context(app)).map(message)
}
pub(super) fn view_overlays<'a>(
    app: &'a App,
    base: iced::Element<'a, Message>,
) -> iced::Element<'a, Message> {
    desktop_settings_skills::browser::view_overlays(context(app), base, message)
}
#[cfg(test)]
pub(super) fn detail_modal_body(app: &App, project_open: bool) -> iced::Element<'_, Message> {
    desktop_settings_skills::browser::detail_modal_body(context(app), project_open).map(message)
}
#[cfg(test)]
pub(super) fn discovery_order_text(app: &App) -> String {
    desktop_settings_skills::browser::discovery_order_text(context(app))
}
#[cfg(test)]
pub(super) fn skill_project_options(app: &App) -> Vec<String> {
    desktop_settings_skills::browser::skill_project_options(context(app))
}
#[cfg(test)]
pub(super) use desktop_settings_skills::browser::detail_source_note;
pub(super) use desktop_settings_skills::browser::{
    DetailActionStyle, catalog_panel_style, header_panel_style,
};
pub(super) fn status_banner<'a>(
    message: impl iced::widget::text::IntoFragment<'a>,
    is_error: bool,
) -> iced::Element<'a, Message> {
    desktop_settings_skills::browser::status_banner(message, is_error).map(self::message)
}
#[cfg(test)]
pub(super) fn scope_button(
    label: &'static str,
    scope: SkillsDirectoryScope,
    active_scope: SkillsDirectoryScope,
    enabled: bool,
) -> iced::Element<'static, Message> {
    desktop_settings_skills::browser::scope_button(label, scope, active_scope, enabled).map(message)
}
#[cfg(test)]
pub(super) fn refresh_button(
    locale: DesktopLocale,
    loading: bool,
) -> iced::Element<'static, Message> {
    desktop_settings_skills::browser::refresh_button(locale, loading).map(message)
}
pub(super) fn loading_banner<'a>(label: &'a str) -> iced::Element<'a, Message> {
    desktop_settings_skills::browser::loading_banner(label).map(message)
}
pub(super) fn empty_state<'a>(title: &'a str, description: &'a str) -> iced::Element<'a, Message> {
    desktop_settings_skills::browser::empty_state(title, description).map(message)
}
pub(super) fn detail_action_button(
    locale: DesktopLocale,
    label: &'static str,
    icon: Icon,
    style: DetailActionStyle,
    on_press: Option<Message>,
) -> Element<'static, Message> {
    desktop_settings_skills::browser::detail_action_button(locale, label, icon, style, on_press)
}

fn help<'a>(
    i18n: desktop_i18n::I18n,
    theme: Option<&iced::Theme>,
    title: &'a str,
    label: iced::widget::Text<'a>,
    description: std::borrow::Cow<'a, str>,
) -> iced::widget::Row<'a, desktop_settings_skills::Event> {
    let content = crate::app::components::chat_panel::message_view::selectable_titled_text(
        title,
        description,
        theme.map(|t| t.palette().text),
        i18n,
    )
    .map(|message| match message {
        Message::CopyCode(text) => desktop_settings_skills::Event::CopyText(text),
        Message::None => desktop_settings_skills::Event::None,
        _ => unreachable!("plain selectable help only emits copy or no-op events"),
    });
    desktop_ui_controls::system_settings_common::help::settings_hover_help_label(label, content)
}
