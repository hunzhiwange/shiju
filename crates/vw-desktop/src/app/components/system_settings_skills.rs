//! 技能模块的本地技能页面。

mod browser;
mod catalog;
mod help;
mod plugin_install_modal;
pub(crate) mod plugins;
pub(crate) mod sync;

use crate::app::state::SkillsSettingsTab;
use crate::app::{App, Message, message};
#[cfg(test)]
use desktop_i18n::settings_extensions::text as tr;
use iced::Element;

fn page_message(event: desktop_settings_skills::page::Message) -> Message {
    use desktop_settings_skills::page::Message as Ui;
    Message::Settings(match event {
        Ui::HelpOpen => message::SettingsMessage::SkillsHelpOpen,
        #[cfg(not(target_arch = "wasm32"))]
        Ui::OpenSkillList => return Message::SkillHub(message::SkillHubMessage::OpenShijuSkills),
        Ui::None => return Message::None,
        Ui::CopyText(text) => return Message::CopyCode(text),
        Ui::SkillsTabChanged(v) => message::SettingsMessage::SkillsTabChanged(v),
        Ui::SkillsProjectAutoLoadToggled(v) => {
            message::SettingsMessage::SkillsProjectAutoLoadToggled(v)
        }
        Ui::SkillsPromptInjectionModeChanged(v) => {
            message::SettingsMessage::SkillsPromptInjectionModeChanged(v)
        }
    })
}
fn config_context(app: &App) -> desktop_settings_skills::page::ConfigContext<'_> {
    let s = &app.skills_settings;
    desktop_settings_skills::page::ConfigContext {
        help,
        desktop_locale: app.general_settings.desktop_locale,
        app_theme: &app.appearance.app_theme,
        auto_load_project_skills: s.auto_load_project_skills,
        prompt_injection_mode: s.prompt_injection_mode,
    }
}
fn system_config_view(app: &App) -> Element<'_, Message> {
    desktop_settings_skills::page::system_config_view(config_context(app)).map(page_message)
}

pub fn view(app: &App) -> Element<'_, Message> {
    let content = match app.skills_settings.active_tab {
        SkillsSettingsTab::Skills => browser::view(app),
        SkillsSettingsTab::DiscoveryOrder => browser::discovery_order_view(app),
        SkillsSettingsTab::SystemConfig => system_config_view(app),
    };
    desktop_settings_skills::page::view(
        config_context(app),
        app.skills_settings.active_tab,
        &app.skills_settings.save_error,
        content,
        page_message,
    )
}

pub fn view_overlays<'a>(app: &'a App, dialog: Element<'a, Message>) -> Element<'a, Message> {
    let dialog = browser::view_overlays(app, dialog);
    let dialog = help::view_overlays(app, dialog);
    sync::overlay(app, dialog)
}

pub(crate) fn is_visible(app: &App) -> bool {
    use crate::app::skill_hub::{ExpertHubSection, PublicSkillTab};

    let market = &app.expert_hub.public_market;
    !app.navigation_panels.show_system_settings
        && app.navigation_tabs.screen == crate::app::Screen::ExpertHub
        && app.expert_hub.section == ExpertHubSection::Skills
        && market.tab == PublicSkillTab::LocalSkills
        && !market.installed_open
        && market.detail.is_none()
}

pub(crate) fn invalidate(app: &mut App) {
    use message::SettingsMessage;

    app.skills_settings.sync.menu = None;
    app.skills_settings.sync.form = None;
    let _ = message::settings::skills::update(app, SettingsMessage::SkillsDetailClosed);
    let _ = message::settings::skills::update(app, SettingsMessage::SkillsHelpClose);
}

#[cfg(test)]
#[path = "system_settings_skills_tests.rs"]
mod system_settings_skills_tests;

pub use desktop_settings_skills::{modal_card_style, plugin_dependency_summary};

pub(crate) fn with_plugin_install<'a>(
    app: &'a App,
    base: Element<'a, Message>,
) -> Element<'a, Message> {
    match app.skills_settings.plugin_install.as_ref() {
        Some(form) => plugin_install_modal::view(app, base, form),
        None => base,
    }
}

#[cfg(test)]
#[path = "system_settings_skills_ui_refresh_19_tests.rs"]
mod ui_refresh_19_tests;

fn help<'a>(
    i18n: desktop_i18n::I18n,
    theme: Option<&iced::Theme>,
    title: &'a str,
    label: iced::widget::Text<'a>,
    description: std::borrow::Cow<'a, str>,
) -> iced::widget::Row<'a, desktop_settings_skills::page::Message> {
    let content = crate::app::components::chat_panel::message_view::selectable_titled_text(
        title,
        description,
        theme.map(|t| t.palette().text),
        i18n,
    )
    .map(|message| match message {
        Message::CopyCode(text) => desktop_settings_skills::page::Message::CopyText(text),
        Message::None => desktop_settings_skills::page::Message::None,
        _ => unreachable!("plain selectable help only emits copy or no-op events"),
    });
    desktop_ui_controls::system_settings_common::help::settings_hover_help_label(label, content)
}
