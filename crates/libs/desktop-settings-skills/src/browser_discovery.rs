use crate::BrowserContext;
use crate::Event as Message;
use crate::browser_widgets::header_panel_style;
use crate::catalog::section_card_style;
use crate::types::SkillsDirectoryScope;
use config_skills::SkillsDirectoryProvider;
use config_ui::DesktopLocale;
use desktop_i18n::{I18n, settings_extensions::text as tr};
use desktop_ui_controls::system_settings_common::settings_value_badge;
use iced::widget::{column, container, row, text};
use iced::{Alignment, Element, Length};

pub fn skill_project_options(app: BrowserContext<'_>) -> Vec<String> {
    let mut options = Vec::new();
    for path in app.project_path.iter().chain(app.recent_projects.iter()) {
        let path = path.trim();
        if !path.is_empty() && !options.iter().any(|option: &String| option.as_str() == path) {
            options.push(path.to_string());
        }
    }
    options
}

pub fn scope_source_matches(scope: SkillsDirectoryScope, source: &str) -> bool {
    match scope {
        SkillsDirectoryScope::Project => source == "workspace",
        SkillsDirectoryScope::Ancestor => source == "ancestor",
        SkillsDirectoryScope::Global => source == "global",
        SkillsDirectoryScope::Bundled => source == "bundled",
        SkillsDirectoryScope::All => true,
    }
}

pub fn scope_title(scope: SkillsDirectoryScope) -> &'static str {
    match scope {
        SkillsDirectoryScope::Project => "项目目录技能",
        SkillsDirectoryScope::Ancestor => "父级目录技能",
        SkillsDirectoryScope::Global => "全局目录技能",
        SkillsDirectoryScope::Bundled => "内置技能",
        SkillsDirectoryScope::All => "全部技能目录",
    }
}

pub fn provider_project_paths(provider: SkillsDirectoryProvider) -> &'static str {
    match provider {
        SkillsDirectoryProvider::Vibewindow => ".shiju/skills, skills, .agents/skills",
        SkillsDirectoryProvider::Codex => ".codex/skills, .agents/skills, skills",
        SkillsDirectoryProvider::Claude => ".claude/skills, skills, .agents/skills",
        SkillsDirectoryProvider::Cursor => ".cursor/skills, skills, .agents/skills",
    }
}

pub fn provider_global_paths(provider: SkillsDirectoryProvider) -> &'static str {
    match provider {
        SkillsDirectoryProvider::Vibewindow => {
            #[cfg(debug_assertions)]
            {
                "~/.shiju-harness-dev/skills, ~/.skills"
            }
            #[cfg(not(debug_assertions))]
            {
                "~/.shiju-harness/skills, ~/.skills"
            }
        }
        SkillsDirectoryProvider::Codex => "~/.codex/skills, ~/.agents/skills",
        SkillsDirectoryProvider::Claude => "~/.claude/skills",
        SkillsDirectoryProvider::Cursor => "~/.cursor/skills",
    }
}

pub fn scope_description(
    locale: DesktopLocale,
    scope: SkillsDirectoryScope,
    provider: SkillsDirectoryProvider,
) -> String {
    match scope {
        SkillsDirectoryScope::Project => desktop_i18n::__!(
            desktop_i18n::I18n::new(locale),
            "查看当前项目 %s 目录命中的技能。",
            (provider_project_paths(provider)).to_string()
        )
        .expect("valid skills display template"),
        SkillsDirectoryScope::Ancestor => desktop_i18n::__!(
            desktop_i18n::I18n::new(locale),
            "查看从项目父级向上发现的 %s 技能。",
            (provider_project_paths(provider)).to_string()
        )
        .expect("valid skills display template"),
        SkillsDirectoryScope::Global => desktop_i18n::__!(
            desktop_i18n::I18n::new(locale),
            "查看当前用户 %s 中的全局技能。",
            (provider_global_paths(provider)).to_string()
        )
        .expect("valid skills display template"),
        SkillsDirectoryScope::Bundled => {
            tr(locale, "查看产品随包提供、可安装到项目的内置技能。").to_string()
        }
        SkillsDirectoryScope::All => tr(
            locale,
            "全部结果包含当前项目、父级目录、全局目录以及内置技能，便于核对最终命中来源。",
        )
        .to_string(),
    }
}

pub fn discovery_order_text(app: BrowserContext<'_>) -> String {
    let locale = app.desktop_locale;
    if let Some(project_path) = &app.project_path {
        return desktop_i18n::__!(
            desktop_i18n::I18n::new(locale),
            "发现顺序：%s/%s -> 父级 %s -> %s",
            (project_path).to_string(),
            (provider_project_paths(app.skills_settings.directory_provider)).to_string(),
            (provider_project_paths(app.skills_settings.directory_provider)).to_string(),
            (provider_global_paths(app.skills_settings.directory_provider)).to_string()
        )
        .expect("valid skills display template");
    }

    desktop_i18n::__!(
        desktop_i18n::I18n::new(locale),
        "未打开项目时，仅显示 %s 与内置技能。",
        (provider_global_paths(app.skills_settings.directory_provider)).to_string()
    )
    .expect("valid skills display template")
}

pub fn discovery_order_view<'a>(app: BrowserContext<'a>) -> Element<'a, Message> {
    let locale = app.desktop_locale;
    let project_open = app.project_path.is_some();
    let provider = app.skills_settings.directory_provider;
    let order = if project_open {
        vec![
            (
                "1",
                desktop_i18n::__!(
                    desktop_i18n::I18n::new(locale),
                    "当前项目 %s",
                    (provider_project_paths(provider)).to_string()
                )
                .expect("valid skills display template"),
                tr(locale, "项目私有技能，优先级最高。").to_string(),
            ),
            (
                "2",
                desktop_i18n::__!(
                    desktop_i18n::I18n::new(locale),
                    "父级 %s",
                    (provider_project_paths(provider)).to_string()
                )
                .expect("valid skills display template"),
                tr(locale, "从项目父级向上发现，越近优先级越高。").to_string(),
            ),
            (
                "3",
                provider_global_paths(provider).to_string(),
                tr(locale, "当前用户全局技能目录。").to_string(),
            ),
            (
                "4",
                tr(locale, "内置技能").to_string(),
                tr(locale, "产品随包提供，可安装到项目后覆盖。").to_string(),
            ),
        ]
    } else {
        vec![
            (
                "1",
                provider_global_paths(provider).to_string(),
                tr(locale, "未打开项目时使用当前用户全局技能目录。").to_string(),
            ),
            (
                "2",
                tr(locale, "内置技能").to_string(),
                tr(locale, "产品随包提供，可安装到项目后覆盖。").to_string(),
            ),
        ]
    };

    let rows = order.into_iter().fold(column![].spacing(10), |column, (index, title, desc)| {
        column.push(
            container(
                row![
                    settings_value_badge(index),
                    column![
                        text(title).size(14).wrapping(iced::widget::text::Wrapping::Glyph),
                        text(desc).size(12).style(|theme: &iced::Theme| {
                            iced::widget::text::Style {
                                color: Some(theme.palette().text.scale_alpha(0.64)),
                            }
                        }),
                    ]
                    .spacing(3)
                    .width(Length::Fill),
                ]
                .spacing(12)
                .align_y(Alignment::Center),
            )
            .padding([12, 14])
            .width(Length::Fill)
            .style(section_card_style),
        )
    });

    let title = tr(locale, "发现顺序");
    column![
        container(
            column![
                row![
                    (app.help)(
                        I18n::new(locale),
                        Some(app.app_theme),
                        title,
                        text(title).size(22),
                        tr(locale, "同名技能只展示最先命中的目录，保持项目优先，避免列表重复。",)
                            .into(),
                    )
                    .width(Length::Fill)
                    .wrap(),
                    settings_value_badge(if project_open {
                        tr(locale, "项目已打开")
                    } else {
                        tr(locale, "未打开项目")
                    }),
                ]
                .spacing(12)
                .align_y(Alignment::Center),
                text(discovery_order_text(app))
                    .size(12)
                    .width(Length::Fill)
                    .wrapping(iced::widget::text::Wrapping::Glyph)
                    .style(|theme: &iced::Theme| iced::widget::text::Style {
                        color: Some(theme.palette().text.scale_alpha(0.68)),
                    }),
            ]
            .spacing(12),
        )
        .padding([18, 20])
        .width(Length::Fill)
        .style(header_panel_style),
        rows,
    ]
    .spacing(16)
    .width(Length::Fill)
    .into()
}
