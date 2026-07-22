//! 技能配置设置界面组件。

mod browser;
mod catalog;
mod help;
mod plugins;

use crate::app::components::system_settings_common::{
    SETTINGS_LABEL_WIDTH, settings_checkbox_style, settings_help_button, settings_muted_text_style,
    settings_page_intro, settings_panel, settings_pick_list_menu_style, settings_pick_list_style,
    settings_section_card, settings_segment_button_style, settings_text_input_style,
};
use crate::app::state::SkillsSettingsTab;
use crate::app::{App, Message, message};
use iced::widget::{button, checkbox, column, container, pick_list, row, text, text_input};
use iced::{Alignment, Element, Length};
use vw_config_types::skills::{SkillsDirectoryProvider, SkillsPromptInjectionMode};

fn field_row<'a>(
    label: &'static str,
    description: &'static str,
    control: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    container(
        row![
            column![
                text(label).size(13),
                text(description).size(11).style(settings_muted_text_style),
            ]
            .spacing(4)
            .width(Length::Fixed(SETTINGS_LABEL_WIDTH)),
            container(control.into()).width(Length::Fill),
        ]
        .spacing(22)
        .align_y(Alignment::Center),
    )
    .padding([14, 0])
    .width(Length::Fill)
    .into()
}

fn settings_tab_button(
    label: &'static str,
    tab: SkillsSettingsTab,
    active_tab: SkillsSettingsTab,
) -> Element<'static, Message> {
    let is_active = tab == active_tab;

    button(text(label).size(13))
        .padding([8, 14])
        .on_press(Message::Settings(message::SettingsMessage::SkillsTabChanged(tab)))
        .style(move |theme: &iced::Theme, status| {
            settings_segment_button_style(theme, status, is_active)
        })
        .into()
}

pub(super) fn skills_tab_labels() -> &'static [(&'static str, SkillsSettingsTab)] {
    &[
        ("技能", SkillsSettingsTab::Skills),
        ("顺序", SkillsSettingsTab::DiscoveryOrder),
        ("插件", SkillsSettingsTab::Plugins),
        ("系统配置", SkillsSettingsTab::SystemConfig),
    ]
}

fn system_config_view(app: &App) -> Element<'_, Message> {
    let s = &app.skills_settings;

    let provider_pick =
        pick_list(SkillsDirectoryProvider::ALL, Some(s.directory_provider), |provider| {
            Message::Settings(message::SettingsMessage::SkillsDirectoryProviderChanged(provider))
        })
        .placeholder("VibeWindow")
        .padding([10, 12])
        .style(settings_pick_list_style)
        .menu_style(settings_pick_list_menu_style)
        .width(Length::Fill);

    let provider_row = field_row(
        "Directory provider",
        "切换技能目录兼容模式；配置缺失时使用 VibeWindow。",
        provider_pick,
    );

    let open_enabled_row = field_row(
        "Community sync",
        "控制是否启用 open-skills 仓库同步。",
        checkbox(s.open_skills_enabled)
            .label("启用 open-skills 仓库同步")
            .on_toggle(|v| Message::Settings(message::SettingsMessage::SkillsOpenEnabledToggled(v)))
            .style(settings_checkbox_style),
    );

    let open_dir_row = field_row(
        "Repository path",
        "为空时默认使用 $HOME/open-skills。",
        text_input("默认: $HOME/open-skills", &s.open_skills_dir_input)
            .on_input(|v| Message::Settings(message::SettingsMessage::SkillsOpenDirChanged(v)))
            .padding([10, 12])
            .size(13)
            .style(settings_text_input_style)
            .width(Length::Fill),
    );

    let mode_full = matches!(s.prompt_injection_mode, SkillsPromptInjectionMode::Full);
    let mode_row = field_row(
        "Injection mode",
        "compact 更省上下文，full 会注入完整技能内容。",
        checkbox(mode_full)
            .label("使用 full（关闭时为 compact）")
            .on_toggle(|v| {
                Message::Settings(message::SettingsMessage::SkillsPromptInjectionModeChanged(
                    if v {
                        SkillsPromptInjectionMode::Full
                    } else {
                        SkillsPromptInjectionMode::Compact
                    },
                ))
            })
            .style(settings_checkbox_style),
    );

    let config_card = column![
        settings_section_card(
            "系统配置",
            "这里保留社区同步与 prompt injection 配置，目录发现顺序单独放在顺序页。",
        ),
        settings_panel(column![provider_row, open_enabled_row, open_dir_row, mode_row].spacing(0)),
    ]
    .spacing(16)
    .width(Length::Fill);

    config_card.into()
}

pub fn view(app: &App) -> Element<'_, Message> {
    let s = &app.skills_settings;
    let help_btn =
        settings_help_button(Message::Settings(message::SettingsMessage::SkillsHelpOpen));

    let content = match s.active_tab {
        SkillsSettingsTab::Skills => browser::view(app),
        SkillsSettingsTab::DiscoveryOrder => browser::discovery_order_view(app),
        SkillsSettingsTab::Plugins => plugins::view(app),
        SkillsSettingsTab::SystemConfig => system_config_view(app),
    };

    column![
        row![
            container(settings_page_intro(
                "技能配置",
                "技能、发现顺序、插件与系统配置分开展示；技能页只保留浏览和搜索。",
            ))
            .width(Length::Fill),
            help_btn,
        ]
        .align_y(Alignment::Start),
        skills_tab_labels()
            .iter()
            .fold(row![].spacing(8).align_y(Alignment::Center), |row, (label, tab)| {
                row.push(settings_tab_button(label, *tab, s.active_tab))
            }),
        content,
    ]
    .spacing(16)
    .width(Length::Fill)
    .into()
}

pub fn view_overlays<'a>(app: &'a App, dialog: Element<'a, Message>) -> Element<'a, Message> {
    let dialog = browser::view_overlays(app, dialog);
    help::view_overlays(app, dialog)
}

#[cfg(test)]
mod browser_tests;
#[cfg(test)]
mod catalog_tests;
#[cfg(test)]
mod help_tests;
#[cfg(test)]
#[path = "system_settings_skills_tests.rs"]
mod system_settings_skills_tests;
