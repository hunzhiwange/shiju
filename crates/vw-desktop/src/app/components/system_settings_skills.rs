//! 技能配置设置界面组件。

mod backup_modal;
mod bindings_view;
mod browser;
mod catalog;
mod claude_view;
mod deeplink;
mod help;
mod import_modal;
mod install_view;
mod jobs_view;
mod market_view;
mod plugin_install_modal;
pub(crate) mod plugins;
mod repo_modal;
mod storage_modal;
mod updates_view;

use crate::app::components::system_settings_common::{
    settings_error_banner, settings_help_button_with_locale, settings_page_heading_with_help,
    settings_tab_row,
};
use crate::app::components::system_settings_search::SettingsTabDescriptor;
use crate::app::i18n::{I18n, settings_extensions::text as tr};
use crate::app::state::SkillsSettingsTab;
use crate::app::{App, Message, message};
use iced::widget::{column, container, row};
use iced::{Alignment, Element, Length};

use desktop_settings_skills::page::SKILLS_TAB_LABELS;
fn page_message(event: desktop_settings_skills::page::Message) -> Message {
    use desktop_settings_skills::page::Message as Ui;
    Message::Settings(match event {
        Ui::None => return Message::None,
        Ui::CopyText(text) => return Message::CopyCode(text),
        Ui::SkillsTabChanged(v) => message::SettingsMessage::SkillsTabChanged(v),
        Ui::SkillsDirectoryProviderChanged(v) => {
            message::SettingsMessage::SkillsDirectoryProviderChanged(v)
        }
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
        desktop_locale: app.desktop_locale,
        app_theme: &app.app_theme,
        directory_provider: s.directory_provider,
        auto_load_project_skills: s.auto_load_project_skills,
        prompt_injection_mode: s.prompt_injection_mode,
    }
}
fn skills_tab_button(
    label: &'static str,
    tab: SkillsSettingsTab,
    active_tab: SkillsSettingsTab,
) -> Element<'static, Message> {
    desktop_settings_skills::page::skills_tab_button(label, tab, active_tab).map(page_message)
}
pub(super) const SKILLS_TABS: &[SettingsTabDescriptor<SkillsSettingsTab>] = &[
    SettingsTabDescriptor::new(
        SKILLS_TAB_LABELS[0].1,
        SKILLS_TAB_LABELS[0].0,
        &["skills", "catalog", "技能目录"],
    ),
    SettingsTabDescriptor::new(
        SKILLS_TAB_LABELS[1].1,
        SKILLS_TAB_LABELS[1].0,
        &["discovery order", "发现顺序", "project", "ancestor", "global", "bundled"],
    ),
    SettingsTabDescriptor::new(
        SKILLS_TAB_LABELS[2].1,
        SKILLS_TAB_LABELS[2].0,
        &["directory provider", "提示注入", "prompt injection", "技能目录路径"],
    ),
];

pub(super) use desktop_settings_skills::page::skills_tab_labels;
fn system_config_view(app: &App) -> Element<'_, Message> {
    desktop_settings_skills::page::system_config_view(config_context(app)).map(page_message)
}

pub fn view(app: &App) -> Element<'_, Message> {
    let s = &app.skills_settings;
    let help_btn = settings_help_button_with_locale(
        app.desktop_locale,
        Message::Settings(message::SettingsMessage::SkillsHelpOpen),
    );

    let content = match s.active_tab {
        SkillsSettingsTab::Skills => browser::view(app),
        SkillsSettingsTab::DiscoveryOrder => browser::discovery_order_view(app),
        SkillsSettingsTab::SystemConfig => system_config_view(app),
    };

    let mut page = column![
        row![
            container(settings_page_heading_with_help(
                I18n::new(app.desktop_locale),
                &app.app_theme,
                tr(app.desktop_locale, "技能配置"),
                tr(
                    app.desktop_locale,
                    "技能、发现顺序与系统配置分开展示；技能页只保留浏览和搜索。"
                ),
            ))
            .width(Length::Fill),
            iced::widget::button("仓库管理")
                .padding([8, 10])
                .style(crate::app::components::system_settings_common::rounded_action_btn_style)
                .on_press(Message::Settings(message::SettingsMessage::SkillRepositories(
                    crate::app::message::settings::skills::repo_modal::Event::Open
                ))),
            iced::widget::button("Claude 配置")
                .padding([8, 10])
                .style(crate::app::components::system_settings_common::rounded_action_btn_style)
                .on_press(crate::app::message::settings::skills::claude_view::message(
                    crate::app::message::settings::skills::claude_view::Event::Open
                )),
            iced::widget::button("技能市场")
                .padding([8, 10])
                .style(crate::app::components::system_settings_common::rounded_action_btn_style)
                .on_press(crate::app::message::settings::skills::market_view::message(
                    crate::app::message::settings::skills::market_view::Event::Open
                )),
            iced::widget::button("更新与卸载")
                .padding([8, 10])
                .style(crate::app::components::system_settings_common::rounded_action_btn_style)
                .on_press(crate::app::message::settings::skills::updates_view::message(
                    crate::app::message::settings::skills::updates_view::Event::Open
                )),
            iced::widget::button("详情与安装")
                .padding([8, 10])
                .style(crate::app::components::system_settings_common::rounded_action_btn_style)
                .on_press(crate::app::message::settings::skills::install_view::message(
                    crate::app::message::settings::skills::install_view::Event::Open
                )),
            iced::widget::button("导入技能")
                .padding([8, 10])
                .style(crate::app::components::system_settings_common::rounded_action_btn_style)
                .on_press(crate::app::message::settings::skills::import_modal::message(
                    crate::app::message::settings::skills::import_modal::Event::Open
                )),
            iced::widget::button("任务进度")
                .padding([8, 10])
                .style(crate::app::components::system_settings_common::rounded_action_btn_style)
                .on_press(crate::app::message::settings::skills::jobs_view::message(
                    crate::app::message::settings::skills::jobs_view::Event::Open
                )),
            iced::widget::button("应用绑定")
                .padding([8, 10])
                .style(crate::app::components::system_settings_common::rounded_action_btn_style)
                .on_press(crate::app::message::settings::skills::bindings_view::message(
                    crate::app::message::settings::skills::bindings_view::Event::Open
                )),
            iced::widget::button("存储与同步")
                .padding([8, 10])
                .style(crate::app::components::system_settings_common::rounded_action_btn_style)
                .on_press(crate::app::message::settings::skills::storage_modal::message(
                    crate::app::message::settings::skills::storage_modal::Event::Open
                )),
            iced::widget::button("备份与恢复")
                .padding([8, 10])
                .style(crate::app::components::system_settings_common::rounded_action_btn_style)
                .on_press(crate::app::message::settings::skills::backup_modal::message(
                    crate::app::message::settings::skills::backup_modal::Event::Open
                )),
            help_btn,
        ]
        .align_y(Alignment::Start),
        settings_tab_row(skills_tab_labels().iter().map(|(label, tab)| {
            skills_tab_button(tr(app.desktop_locale, label), *tab, s.active_tab)
        }))
        .wrap(),
    ]
    .spacing(16)
    .width(Length::Fill);

    if let Some(error) = &s.save_error {
        page = page
            .push(settings_error_banner(crate::app::i18n::I18n::new(app.desktop_locale).__(error)));
    }
    page.push(content).into()
}

pub fn view_overlays<'a>(app: &'a App, dialog: Element<'a, Message>) -> Element<'a, Message> {
    let dialog = browser::view_overlays(app, dialog);
    let dialog = help::view_overlays(app, dialog);
    let dialog = repo_modal::view(app, dialog);
    let dialog = claude_view::view(app, dialog);
    let dialog = bindings_view::view(app, dialog);
    let dialog = storage_modal::view(app, dialog);
    let dialog = backup_modal::view(app, dialog);
    let dialog = market_view::view(app, dialog);
    let dialog = install_view::view(app, dialog);
    let dialog = import_modal::view(app, dialog);
    let dialog = jobs_view::view(app, dialog);
    updates_view::view(app, dialog)
}

#[cfg(test)]
mod help_tests;
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
