mod ui;
use crate::types::*;
use crate::update::state::State;
use desktop_i18n::settings_extensions::text as tr;
use iced::Element;

struct Fixture {
    project_path: Option<String>,
    recent_projects: Vec<String>,
    skills_settings: State,
    desktop_locale: config_ui::DesktopLocale,
    app_theme: iced::Theme,
}
#[derive(Clone, Debug)]
enum Message {
    None,
    Browser(crate::Event),
    Page(crate::page::Message),
    Plugin(crate::plugin_install::Event),
    CopyCode(String),
    View(ViewMessage),
}
#[derive(Clone, Debug)]
enum ViewMessage {
    OpenUrlExternal(String),
}
impl desktop_ui_controls::selectable_text::markdown_selection::SelectionMessage for Message {
    fn none() -> Self {
        Self::None
    }
    fn copy(text: String) -> Self {
        Self::CopyCode(text)
    }
    fn context(effect: desktop_ui_controls::text_editor_context_menu::TextContextEffect) -> Self {
        use desktop_ui_controls::text_editor_context_menu::TextContextEffect;
        match effect {
            TextContextEffect::Copy(text) => Self::CopyCode(text),
            TextContextEffect::OpenUrl(url) => Self::View(ViewMessage::OpenUrlExternal(url)),
            _ => Self::None,
        }
    }
    fn selection(_: String, _: usize, _: usize, _: Option<String>) -> Self {
        Self::None
    }
}
fn help<'a, 'theme, E: Clone + 'static>(
    i18n: desktop_i18n::I18n,
    theme: Option<&'theme iced::Theme>,
    title: &'a str,
    label: iced::widget::Text<'a>,
    description: std::borrow::Cow<'a, str>,
) -> iced::widget::Row<'a, E>
where
    E: desktop_ui_controls::selectable_text::markdown_selection::SelectionMessage,
{
    let content = desktop_ui_controls::selectable_text::markdown_selection::selectable_titled_text::<
        E,
    >(title, description, theme.map(|t| t.palette().text), i18n);
    desktop_ui_controls::system_settings_common::help::settings_hover_help_label(label, content)
}

fn context(app: &Fixture) -> crate::BrowserContext<'_> {
    let s = &app.skills_settings;
    crate::BrowserContext {
        help,
        desktop_locale: app.desktop_locale,
        app_theme: &app.app_theme,
        project_path: &app.project_path,
        recent_projects: &app.recent_projects,
        skills_settings: crate::BrowserState {
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
fn config_context(app: &Fixture) -> crate::page::ConfigContext<'_> {
    let s = &app.skills_settings;
    crate::page::ConfigContext {
        help,
        desktop_locale: app.desktop_locale,
        app_theme: &app.app_theme,
        directory_provider: s.directory_provider,
        auto_load_project_skills: s.auto_load_project_skills,
        prompt_injection_mode: s.prompt_injection_mode,
    }
}
fn browser_message(event: crate::Event) -> Message {
    match event {
        crate::Event::None => Message::None,
        crate::Event::CopyText(v) => Message::CopyCode(v),
        e => Message::Browser(e),
    }
}
fn page_message(event: crate::page::Message) -> Message {
    match event {
        crate::page::Message::None => Message::None,
        crate::page::Message::CopyText(v) => Message::CopyCode(v),
        e => Message::Page(e),
    }
}
fn view(app: &Fixture) -> Element<'_, Message> {
    let content = match app.skills_settings.active_tab {
        SkillsSettingsTab::Skills => crate::browser::view(context(app)).map(browser_message),
        SkillsSettingsTab::DiscoveryOrder => {
            crate::browser::discovery_order_view(context(app)).map(browser_message)
        }
        SkillsSettingsTab::SystemConfig => {
            crate::page::system_config_view(config_context(app)).map(page_message)
        }
    };
    crate::page::view(
        config_context(app),
        app.skills_settings.active_tab,
        &app.skills_settings.save_error,
        content,
        page_message,
        help::<Message>,
    )
}
fn view_overlays<'a>(app: &'a Fixture, base: Element<'a, Message>) -> Element<'a, Message> {
    let base = crate::browser::view_overlays(context(app), base, browser_message);
    if !app.skills_settings.show_help_modal {
        return base;
    }
    desktop_ui_controls::system_settings_common::help::with_settings_help_modal(
        base,
        tr(app.desktop_locale, "Skills 配置帮助").into(),
        crate::help::help_text(app.desktop_locale).into(),
        desktop_assets::assets::Icon::Copy,
        tr(app.desktop_locale, "复制"),
        Message::None,
        Message::None,
    )
}
fn with_plugin_install<'a>(app: &'a Fixture, base: Element<'a, Message>) -> Element<'a, Message> {
    match &app.skills_settings.plugin_install {
        Some(form) => crate::plugin_install_modal::view(
            app.desktop_locale,
            &app.app_theme,
            base,
            form,
            Message::Plugin,
        ),
        None => base,
    }
}
mod plugins {
    use super::*;
    pub(super) fn view(app: &Fixture) -> Element<'_, Message> {
        let s = &app.skills_settings;
        crate::plugins::view(crate::plugins::ViewState {
            desktop_locale: app.desktop_locale,
            app_theme: &app.app_theme,
            query: &s.query,
            plugin_catalog: &s.plugin_catalog,
            selected_plugin_id: &s.selected_plugin_id,
            plugin_loading: s.plugin_loading,
            plugin_status_message: &s.plugin_status_message,
            plugin_status_message_translation: &s.plugin_status_message_translation,
            plugin_status_is_error: s.plugin_status_is_error,
        })
        .map(browser_message)
    }
}
#[test]
fn view_builds_every_skills_settings_tab() {
    let mut app = Fixture::default();

    for (_, tab) in crate::page::skills_tab_labels() {
        app.skills_settings.active_tab = *tab;
        std::hint::black_box(view(&app));
        assert_eq!(app.skills_settings.active_tab, *tab);
    }
}

#[test]
fn skills_error_banner_follows_skills_tabs() {
    let mut app = Fixture::default();

    for (_, tab) in crate::page::skills_tab_labels() {
        app.skills_settings.active_tab = *tab;
        app.skills_settings.save_error = None;
        assert_eq!(view(&app).as_widget().children().len(), 3);

        app.skills_settings.save_error = Some("skills save failed".to_string());
        assert_eq!(view(&app).as_widget().children().len(), 4);
    }
}

#[test]
fn view_overlays_builds_with_help_closed_and_open() {
    let app = Fixture::default();
    std::hint::black_box(view_overlays(&app, iced::widget::text("基础对话框").into()));

    let mut app = Fixture::default();
    app.skills_settings.show_help_modal = true;
    std::hint::black_box(view_overlays(&app, iced::widget::text("基础对话框").into()));
}

#[test]
fn copy_install_command_emits_the_documented_template() {
    use crate::update::catalog::{Effect, Event, INSTALL_COMMAND_TEMPLATE, update};
    let mut state = State::default();
    assert!(
        matches!(update(&mut state, &None, Event::SkillsCopyInstallCommand), Effect::Copy(command) if command == INSTALL_COMMAND_TEMPLATE)
    );
}

#[cfg(test)]
mod host_module_39_render;

impl Default for Fixture {
    fn default() -> Self {
        Self {
            project_path: Default::default(),
            recent_projects: Default::default(),
            skills_settings: Default::default(),
            desktop_locale: Default::default(),
            app_theme: iced::Theme::Light,
        }
    }
}
