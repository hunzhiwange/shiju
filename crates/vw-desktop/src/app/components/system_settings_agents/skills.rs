//! 系统设置中委托代理默认技能和访问策略的界面拼装。
//!
//! 本模块只负责把已发现的技能目录映射成代理级默认注入选择控件。

use super::shared::{is_dark_theme, section_card};
use crate::app::components::system_settings_common::{
    rounded_action_btn_style, settings_muted_text_style, settings_panel_style,
    settings_segment_button_style, settings_value_badge,
};
use crate::app::message::settings::{AgentsMessage, SettingsMessage};
use crate::app::state::{DelegateAgentSettingsEntry, SkillsCatalogItem, SkillsDirectoryScope};
use crate::app::{App, Message};
use iced::widget::{button, column, row, text};
use iced::{Alignment, Background, Border, Color, Element, Length, Theme};
use vw_config_types::agent::DelegateCapabilityPolicy;

pub(super) fn capability_policy_label(policy: DelegateCapabilityPolicy) -> &'static str {
    match policy {
        DelegateCapabilityPolicy::Strict => "严格限制",
        DelegateCapabilityPolicy::UserOverride => "限制但允许用户选择",
        DelegateCapabilityPolicy::Unrestricted => "不限制",
    }
}

pub(super) fn skill_policy_message(
    agent_key: String,
    policy: DelegateCapabilityPolicy,
) -> Message {
    Message::Settings(SettingsMessage::Agents(AgentsMessage::SkillPolicyChanged(agent_key, policy)))
}

fn skill_policy_button(
    entry: &DelegateAgentSettingsEntry,
    policy: DelegateCapabilityPolicy,
) -> Element<'static, Message> {
    let selected = entry.skill_policy == policy;
    button(text(capability_policy_label(policy)).size(12))
        .padding([8, 12])
        .style(move |theme: &Theme, status| settings_segment_button_style(theme, status, selected))
        .on_press(skill_policy_message(entry.key.clone(), policy))
        .into()
}

pub(super) fn skill_card_button_style(
    theme: &Theme,
    status: iced::widget::button::Status,
    is_selected: bool,
) -> iced::widget::button::Style {
    let panel_style = settings_panel_style(theme);
    let segment_style = settings_segment_button_style(theme, status, is_selected);
    let palette = theme.extended_palette();
    let is_dark = is_dark_theme(theme);

    let background = if is_selected {
        segment_style.background
    } else {
        match status {
            iced::widget::button::Status::Hovered => Some(Background::Color(if is_dark {
                palette.background.weak.color.scale_alpha(0.30)
            } else {
                Color::WHITE.scale_alpha(0.88)
            })),
            iced::widget::button::Status::Pressed => Some(Background::Color(if is_dark {
                palette.background.strong.color.scale_alpha(0.82)
            } else {
                palette.background.weak.color.scale_alpha(0.92)
            })),
            _ => panel_style.background,
        }
    };

    let border_color = if is_selected {
        segment_style.border.color
    } else if is_dark {
        palette.background.strong.color.scale_alpha(0.78)
    } else {
        panel_style.border.color
    };

    iced::widget::button::Style {
        background,
        text_color: theme.palette().text,
        border: Border { width: 1.0, color: border_color, radius: 16.0.into() },
        ..Default::default()
    }
}

pub(super) fn skill_source_label(source: &str) -> &'static str {
    match source {
        "workspace" => "项目",
        "ancestor" => "父级",
        "global" => "全局",
        "bundled" => "内置",
        _ => "其他",
    }
}

pub(super) fn scope_source_matches(scope: SkillsDirectoryScope, source: &str) -> bool {
    match scope {
        SkillsDirectoryScope::Project => source == "workspace",
        SkillsDirectoryScope::Ancestor => source == "ancestor",
        SkillsDirectoryScope::Global => source == "global",
        SkillsDirectoryScope::Bundled => source == "bundled",
        SkillsDirectoryScope::All => true,
    }
}

pub(super) fn scope_description(scope: SkillsDirectoryScope) -> &'static str {
    match scope {
        SkillsDirectoryScope::Project => "当前项目 skills、.agents/skills 与所选 provider 目录。",
        SkillsDirectoryScope::Ancestor => {
            "从项目父级向上发现的 skills、.agents/skills 与所选 provider 目录。"
        }
        SkillsDirectoryScope::Global => {
            #[cfg(debug_assertions)]
            {
                "当前用户 ~/.vibewindowdev/skills。"
            }
            #[cfg(not(debug_assertions))]
            {
                "当前用户 ~/.vibewindow/skills。"
            }
        }
        SkillsDirectoryScope::Bundled => "产品随包提供的内置技能。",
        SkillsDirectoryScope::All => "按加载顺序汇总全部技能来源。",
    }
}

pub(super) fn discovery_order_text(app: &App) -> String {
    if let Some(project_path) = &app.project_path {
        format!(
            "{project_path}/skills -> {project_path}/.agents/skills -> 父级 skills、.agents/skills -> {}/skills -> 内置技能",
            vw_config_types::paths::tilde_config_path("")
        )
    } else {
        format!("{} -> 内置技能", vw_config_types::paths::tilde_config_path("skills"))
    }
}

fn scope_button(
    label: &'static str,
    scope: SkillsDirectoryScope,
    active_scope: SkillsDirectoryScope,
    enabled: bool,
) -> Element<'static, Message> {
    let is_active = scope == active_scope;
    let mut button = button(text(label).size(12)).padding([8, 12]);

    if enabled {
        button =
            button.on_press(Message::Settings(SettingsMessage::SkillsDirectoryScopeChanged(scope)));
    }

    button
        .style(move |theme: &Theme, status| settings_segment_button_style(theme, status, is_active))
        .into()
}

pub(super) fn enabled_skill_ids(skills: &[&SkillsCatalogItem]) -> Vec<String> {
    skills.iter().filter(|skill| skill.enabled).map(|skill| skill.id.clone()).collect()
}

fn skill_card<'a>(
    entry: &'a DelegateAgentSettingsEntry,
    skill: &'a SkillsCatalogItem,
) -> Element<'a, Message> {
    let checked = entry.allowed_skills.iter().any(|selected| selected == &skill.id);
    let agent_key = entry.key.clone();
    let skill_id = skill.id.clone();
    let source = skill_source_label(&skill.source);
    let status = if skill.enabled { source.to_string() } else { format!("{source} · 已停用") };

    button(
        column![
            row![
                text(&skill.title)
                    .size(14)
                    .font(iced::Font { weight: iced::font::Weight::Bold, ..Default::default() })
                    .width(Length::Fill),
                settings_value_badge(status),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
            text(&skill.description).size(12).style(move |theme: &Theme| {
                iced::widget::text::Style {
                    color: Some(theme.palette().text.scale_alpha(if checked {
                        0.72
                    } else {
                        0.66
                    })),
                }
            }),
            text(&skill.id).size(11).style(move |theme: &Theme| {
                iced::widget::text::Style {
                    color: Some(theme.palette().text.scale_alpha(if checked {
                        0.56
                    } else {
                        0.46
                    })),
                }
            }),
        ]
        .spacing(6)
        .width(Length::Fill),
    )
    .padding([10, 14])
    .width(Length::Fixed(260.0))
    .style(move |theme: &Theme, status| skill_card_button_style(theme, status, checked))
    .on_press(Message::Settings(SettingsMessage::Agents(AgentsMessage::AllowedSkillToggled(
        agent_key, skill_id, !checked,
    ))))
    .into()
}

pub(super) fn view<'a>(
    app: &'a App,
    entry: &'a DelegateAgentSettingsEntry,
) -> Element<'a, Message> {
    let active_scope = app.skills_settings.directory_scope;
    let skills = app
        .skills_settings
        .catalog
        .iter()
        .filter(|skill| scope_source_matches(active_scope, &skill.source))
        .collect::<Vec<_>>();
    let enabled_skills = enabled_skill_ids(&skills);
    let selected_count = entry
        .allowed_skills
        .iter()
        .filter(|skill| enabled_skills.iter().any(|available| available == *skill))
        .count();
    let project_open = app.project_path.is_some();
    let scope_filter = column![
        row![
            scope_button("项目目录", SkillsDirectoryScope::Project, active_scope, project_open),
            scope_button("父级目录", SkillsDirectoryScope::Ancestor, active_scope, project_open),
            scope_button("全局目录", SkillsDirectoryScope::Global, active_scope, true),
            scope_button("内置技能", SkillsDirectoryScope::Bundled, active_scope, true),
            scope_button("全部", SkillsDirectoryScope::All, active_scope, true),
        ]
        .spacing(8)
        .wrap(),
        text(scope_description(active_scope)).size(12).style(settings_muted_text_style),
        text(discovery_order_text(app))
            .size(11)
            .width(Length::Fill)
            .wrapping(iced::widget::text::Wrapping::Word)
            .style(settings_muted_text_style),
    ]
    .spacing(8);

    let skills_list: Element<'_, Message> = if skills.is_empty() {
        column![
            text(if app.skills_settings.loading {
                "正在刷新技能目录 ..."
            } else {
                "当前筛选范围暂无可用技能"
            })
            .size(12)
            .style(settings_muted_text_style),
            button(text("刷新技能").size(12))
                .padding([8, 14])
                .on_press(Message::Settings(SettingsMessage::SkillsRefresh))
                .style(rounded_action_btn_style),
        ]
        .spacing(10)
        .into()
    } else {
        let cards = skills.iter().map(|skill| skill_card(entry, *skill)).collect::<Vec<_>>();

        column![
            row![
                button(text("全选可用").size(12))
                    .padding([8, 14])
                    .on_press(Message::Settings(SettingsMessage::Agents(
                        AgentsMessage::AllowedSkillsSelectAll(entry.key.clone(), active_scope),
                    )))
                    .style(rounded_action_btn_style),
                button(text("反选可用").size(12))
                    .padding([8, 14])
                    .on_press(Message::Settings(SettingsMessage::Agents(
                        AgentsMessage::AllowedSkillsInvertSelection(
                            entry.key.clone(),
                            active_scope
                        ),
                    )))
                    .style(rounded_action_btn_style),
                settings_value_badge(format!("已选 {}/{}", selected_count, enabled_skills.len(),)),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
            row(cards).spacing(12).wrap(),
        ]
        .spacing(12)
        .into()
    };

    column![
        section_card(
            "默认技能",
            "选择会在委托代理首步默认注入的技能；它们不是技能白名单。",
        ),
        column![
            text("技能访问策略").size(14),
            text("控制委托代理可见技能的范围；不限制为默认策略。")
                .size(12)
                .style(settings_muted_text_style),
            row![
                skill_policy_button(entry, DelegateCapabilityPolicy::Strict),
                skill_policy_button(entry, DelegateCapabilityPolicy::UserOverride),
                skill_policy_button(entry, DelegateCapabilityPolicy::Unrestricted),
            ]
            .spacing(8)
            .wrap(),
        ]
        .spacing(8),
        scope_filter,
        skills_list,
    ]
    .spacing(14)
    .into()
}
