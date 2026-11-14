//! 技能配置设置界面组件。

mod browser;
mod updates_view;
mod install_view;
mod jobs_view;
mod market_view;
mod repo_modal;
mod deeplink;
mod import_modal;
mod claude_view;
mod bindings_view;
mod storage_modal;
mod backup_modal;
mod catalog;
mod help;
mod plugin_install_modal;
pub(crate) mod plugins;

use crate::app::components::system_settings_common::{
    SETTINGS_CONTROL_TEXT_SIZE, settings_checkbox_style, settings_error_banner,
    settings_flexible_row_with_help, settings_group_card, settings_group_heading_with_help,
    settings_help_button_with_locale, settings_modal_card_style, settings_page_heading_with_help,
    settings_select, settings_tab_button, settings_tab_row,
};
use crate::app::components::system_settings_search::SettingsTabDescriptor;
use crate::app::i18n::{I18n, settings_extensions::text as tr};
use crate::app::state::SkillsSettingsTab;
use crate::app::{App, Message, message};
use iced::widget::{checkbox, column, container, row};
use iced::{Alignment, Element, Length};
use vw_config_types::skills::{SkillsDirectoryProvider, SkillsPromptInjectionMode};

fn field_row<'a>(
    app: &App,
    label: &'static str,
    description: &'static str,
    control: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    settings_flexible_row_with_help(
        I18n::new(app.desktop_locale),
        &app.app_theme,
        label,
        description,
        330.0,
        control,
    )
    .into()
}

fn skills_tab_button(
    label: &'static str,
    tab: SkillsSettingsTab,
    active_tab: SkillsSettingsTab,
) -> Element<'static, Message> {
    settings_tab_button(
        label,
        tab == active_tab,
        Some(Message::Settings(message::SettingsMessage::SkillsTabChanged(tab))),
    )
    .into()
}

const SKILLS_TAB_LABELS: &[(&str, SkillsSettingsTab)] = &[
    ("技能", SkillsSettingsTab::Skills),
    ("顺序", SkillsSettingsTab::DiscoveryOrder),
    ("系统配置", SkillsSettingsTab::SystemConfig),
];
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

pub(super) fn skills_tab_labels() -> &'static [(&'static str, SkillsSettingsTab)] {
    SKILLS_TAB_LABELS
}

fn system_config_view(app: &App) -> Element<'_, Message> {
    let s = &app.skills_settings;

    let provider_pick =
        settings_select(SkillsDirectoryProvider::ALL, Some(s.directory_provider), |provider| {
            Message::Settings(message::SettingsMessage::SkillsDirectoryProviderChanged(provider))
        })
        .placeholder("ShiJu")
        .width(Length::Fill);

    let provider_row = field_row(
        app,
        tr(app.desktop_locale, "技能目录提供者"),
        tr(app.desktop_locale, "切换技能目录兼容模式；配置缺失时使用 ShiJu。"),
        provider_pick,
    );

    let project_auto_load_row = field_row(
        app,
        tr(app.desktop_locale, "项目技能"),
        tr(
            app.desktop_locale,
            "自动向委托代理提供项目及父级 skills、.agents/skills；无需逐项选择。",
        ),
        checkbox(s.auto_load_project_skills)
            .label(tr(app.desktop_locale, "默认加载项目技能"))
            .on_toggle(|v| {
                Message::Settings(message::SettingsMessage::SkillsProjectAutoLoadToggled(v))
            })
            .text_size(SETTINGS_CONTROL_TEXT_SIZE)
            .style(settings_checkbox_style),
    );

    let mode_full = matches!(s.prompt_injection_mode, SkillsPromptInjectionMode::Full);
    let mode_row = field_row(
        app,
        tr(app.desktop_locale, "提示注入模式"),
        tr(app.desktop_locale, "compact 更省上下文，full 会注入完整技能内容。"),
        checkbox(mode_full)
            .label(tr(app.desktop_locale, "使用 full（关闭时为 compact）"))
            .on_toggle(|v| {
                Message::Settings(message::SettingsMessage::SkillsPromptInjectionModeChanged(
                    if v {
                        SkillsPromptInjectionMode::Full
                    } else {
                        SkillsPromptInjectionMode::Compact
                    },
                ))
            })
            .text_size(SETTINGS_CONTROL_TEXT_SIZE)
            .style(settings_checkbox_style),
    );

    let config_card = column![
        settings_group_heading_with_help(
            I18n::new(app.desktop_locale),
            &app.app_theme,
            tr(app.desktop_locale, "系统配置"),
            tr(
                app.desktop_locale,
                "这里配置项目技能默认加载与 prompt injection；目录顺序单独展示。"
            ),
        ),
        settings_group_card(column![provider_row, project_auto_load_row, mode_row]),
    ]
    .spacing(16)
    .width(Length::Fill);

    config_card.into()
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
                    crate::app::message::settings::skills::repo_modal::Event::Open))),
            iced::widget::button("Claude 配置").padding([8, 10])
                .style(crate::app::components::system_settings_common::rounded_action_btn_style)
                .on_press(crate::app::message::settings::skills::claude_view::message(
                    crate::app::message::settings::skills::claude_view::Event::Open)),
            iced::widget::button("技能市场").padding([8, 10])
                .style(crate::app::components::system_settings_common::rounded_action_btn_style)
                .on_press(crate::app::message::settings::skills::market_view::message(
                    crate::app::message::settings::skills::market_view::Event::Open)),
            iced::widget::button("更新与卸载").padding([8, 10])
                .style(crate::app::components::system_settings_common::rounded_action_btn_style)
                .on_press(crate::app::message::settings::skills::updates_view::message(
                    crate::app::message::settings::skills::updates_view::Event::Open)),
            iced::widget::button("详情与安装").padding([8, 10])
                .style(crate::app::components::system_settings_common::rounded_action_btn_style)
                .on_press(crate::app::message::settings::skills::install_view::message(
                    crate::app::message::settings::skills::install_view::Event::Open)),
            iced::widget::button("导入技能").padding([8, 10])
                .style(crate::app::components::system_settings_common::rounded_action_btn_style)
                .on_press(crate::app::message::settings::skills::import_modal::message(
                    crate::app::message::settings::skills::import_modal::Event::Open)),
            iced::widget::button("任务进度").padding([8, 10])
                .style(crate::app::components::system_settings_common::rounded_action_btn_style)
                .on_press(crate::app::message::settings::skills::jobs_view::message(
                    crate::app::message::settings::skills::jobs_view::Event::Open)),
            iced::widget::button("应用绑定").padding([8, 10])
                .style(crate::app::components::system_settings_common::rounded_action_btn_style)
                .on_press(crate::app::message::settings::skills::bindings_view::message(
                    crate::app::message::settings::skills::bindings_view::Event::Open)),
            iced::widget::button("存储与同步").padding([8, 10])
                .style(crate::app::components::system_settings_common::rounded_action_btn_style)
                .on_press(crate::app::message::settings::skills::storage_modal::message(
                    crate::app::message::settings::skills::storage_modal::Event::Open)),
            iced::widget::button("备份与恢复").padding([8, 10])
                .style(crate::app::components::system_settings_common::rounded_action_btn_style)
                .on_press(crate::app::message::settings::skills::backup_modal::message(
                    crate::app::message::settings::skills::backup_modal::Event::Open)),
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
mod browser_tests;
#[cfg(test)]
mod catalog_tests;
#[cfg(test)]
mod help_tests;
#[cfg(test)]
#[path = "system_settings_skills_tests.rs"]
mod system_settings_skills_tests;

/// Summarize only observed dependency facts, never infer runtime compatibility from counts.
pub fn plugin_dependency_summary(
    checks: &[vw_gateway_client::DesktopPluginDependencyDto],
) -> &'static str {
    if checks.iter().any(|c| {
        !matches!(
            c.status.as_str(),
            "present"
                | "missing_host"
                | "missing_package"
                | "unverified"
                | "scope_excluded"
                | "invalid"
        )
    }) {
        "未知检查结果，需重新审计"
    } else if checks.iter().any(|c| matches!(c.status.as_str(), "missing_host" | "missing_package"))
    {
        "缺少运行依赖"
    } else if checks.iter().any(|c| c.status == "invalid") {
        "依赖声明无效"
    } else if checks.iter().any(|c| c.status == "unverified") {
        "运行依赖待验证"
    } else if checks.is_empty() {
        "未声明依赖，运行待验证"
    } else if checks.iter().any(|c| c.status == "scope_excluded") {
        "含范围外托管能力"
    } else {
        "依赖文件已检测，运行待验证"
    }
}

fn modal_card_style(theme: &iced::Theme) -> iced::widget::container::Style {
    let mut style = settings_modal_card_style(theme);
    if let Some(iced::Background::Color(color)) = &mut style.background {
        color.a = 1.0;
    }
    style
}

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
