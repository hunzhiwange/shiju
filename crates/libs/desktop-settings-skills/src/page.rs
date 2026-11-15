use crate::types::SkillsSettingsTab;
use config_skills::{SkillsDirectoryProvider, SkillsPromptInjectionMode};
use desktop_i18n::{I18n, settings_extensions::text as tr};
use desktop_ui_controls::system_settings_common::*;
use iced::{
    Element, Length,
    widget::{checkbox, column},
};
#[derive(Clone, Copy)]
pub struct ConfigContext<'a> {
    pub help: crate::HelpRenderer<Message>,
    pub desktop_locale: config_ui::DesktopLocale,
    pub app_theme: &'a iced::Theme,
    pub directory_provider: SkillsDirectoryProvider,
    pub auto_load_project_skills: bool,
    pub prompt_injection_mode: SkillsPromptInjectionMode,
}
#[derive(Clone, Debug)]
pub enum Message {
    None,
    CopyText(String),
    SkillsTabChanged(SkillsSettingsTab),
    SkillsDirectoryProviderChanged(SkillsDirectoryProvider),
    SkillsProjectAutoLoadToggled(bool),
    SkillsPromptInjectionModeChanged(SkillsPromptInjectionMode),
}
pub fn field_row<'a>(
    app: ConfigContext<'_>,
    label: &'static str,
    description: &'static str,
    control: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    let label = (app.help)(
        I18n::new(app.desktop_locale),
        Some(app.app_theme),
        label,
        settings_field_label(label),
        description.into(),
    )
    .width(Length::Fill)
    .wrap();
    settings_flexible_row_with_label(label, 330.0, control).into()
}

pub fn skills_tab_button(
    label: &'static str,
    tab: SkillsSettingsTab,
    active_tab: SkillsSettingsTab,
) -> Element<'static, Message> {
    settings_tab_button(label, tab == active_tab, Some(Message::SkillsTabChanged(tab))).into()
}

pub const SKILLS_TAB_LABELS: &[(&str, SkillsSettingsTab)] = &[
    ("技能", SkillsSettingsTab::Skills),
    ("顺序", SkillsSettingsTab::DiscoveryOrder),
    ("系统配置", SkillsSettingsTab::SystemConfig),
];
pub fn skills_tab_labels() -> &'static [(&'static str, SkillsSettingsTab)] {
    SKILLS_TAB_LABELS
}

pub fn system_config_view(app: ConfigContext<'_>) -> Element<'static, Message> {
    let s = app;

    let provider_pick =
        settings_select(SkillsDirectoryProvider::ALL, Some(s.directory_provider), |provider| {
            Message::SkillsDirectoryProviderChanged(provider)
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
            .on_toggle(|v| Message::SkillsProjectAutoLoadToggled(v))
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
                Message::SkillsPromptInjectionModeChanged(if v {
                    SkillsPromptInjectionMode::Full
                } else {
                    SkillsPromptInjectionMode::Compact
                })
            })
            .text_size(SETTINGS_CONTROL_TEXT_SIZE)
            .style(settings_checkbox_style),
    );

    let config_card = column![
        (app.help)(
            I18n::new(app.desktop_locale),
            Some(app.app_theme),
            tr(app.desktop_locale, "系统配置"),
            iced::widget::text(tr(app.desktop_locale, "系统配置")).size(17),
            tr(
                app.desktop_locale,
                "这里配置项目技能默认加载与 prompt injection；目录顺序单独展示。"
            )
            .into(),
        )
        .wrap(),
        settings_group_card(column![provider_row, project_auto_load_row, mode_row]),
    ]
    .spacing(16)
    .width(Length::Fill);

    config_card.into()
}
