use crate::help::{settings_hover_help_label, settings_group_heading_with_help, settings_flexible_row_with_help};
// 系统设置中委托代理默认技能和访问策略的界面拼装。
//
// 本模块只负责把已发现的技能目录映射成代理级默认注入选择控件。

use super::shared::is_dark_theme;
use desktop_ui_controls::system_settings_common::{
    rounded_action_btn_style, settings_error_banner, settings_field_label,
     settings_group_card,
     settings_muted_text_style, settings_panel_style,
    settings_segment_button_style, settings_select, settings_value_badge,
};
use desktop_i18n::{I18n, settings_extensions::text as tr};
use crate::message::{AgentsMessage, Event as Message};
use crate::state::DelegateAgentSettingsEntry;
use desktop_settings_skills::types::{SkillsCatalogItem, SkillsDirectoryScope};

use iced::widget::tooltip::{Position as TooltipPosition, Tooltip};
use iced::widget::{button, column, container, row, text};
use iced::{Alignment, Background, Border, Color, Element, Length, Theme};
use std::fmt;
use config_agent_profile::AgentDefault;
use config_agent::DelegateCapabilityPolicy;
use config_ui::DesktopLocale;

pub fn capability_policy_label(policy: DelegateCapabilityPolicy) -> &'static str {
    match policy {
        DelegateCapabilityPolicy::Strict => "严格限制",
        DelegateCapabilityPolicy::Unrestricted => "不限制",
    }
}

pub fn capability_policy_tooltip(policy: DelegateCapabilityPolicy) -> &'static str {
    match policy {
        DelegateCapabilityPolicy::Strict => {
            "仅允许原有白名单中已启用的技能，不允许借默认选择扩大范围。"
        }
        DelegateCapabilityPolicy::Unrestricted => {
            "默认提示使用下方已选技能，同时允许在会话中临时选择其他已启用技能。"
        }
    }
}

pub fn skill_policy_message(agent_key: String, policy: DelegateCapabilityPolicy) -> Message {
    Message::Agents(AgentsMessage::SkillPolicyChanged(agent_key, policy))
}

fn skill_policy_button(
    locale: DesktopLocale,
    entry: &DelegateAgentSettingsEntry,
    policy: DelegateCapabilityPolicy,
) -> Element<'static, Message> {
    let selected = entry.skill_policy == policy;
    let policy_button = button(text(tr(locale, capability_policy_label(policy))).size(12))
        .padding([8, 12])
        .style(move |theme: &Theme, status| settings_segment_button_style(theme, status, selected))
        .on_press(skill_policy_message(entry.key.clone(), policy));
    let tip = container(text(tr(locale, capability_policy_tooltip(policy))).size(12))
        .padding([6, 10])
        .style(settings_panel_style);

    Tooltip::new(policy_button, tip, TooltipPosition::Bottom).gap(6.0).into()
}

pub fn skill_card_button_style(
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

pub fn skill_source_label(source: &str) -> &'static str {
    match source {
        "workspace" => "项目",
        "ancestor" => "父级",
        "global" => "全局",
        "bundled" => "待安装",
        _ => "其他",
    }
}

pub fn agent_skills_scope(scope: SkillsDirectoryScope) -> SkillsDirectoryScope {
    match scope {
        SkillsDirectoryScope::Bundled => SkillsDirectoryScope::All,
        scope => scope,
    }
}

pub fn agent_skill_matches_scope(
    skill: &SkillsCatalogItem,
    scope: SkillsDirectoryScope,
) -> bool {
    skill.installed && scope_source_matches(scope, &skill.source)
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

pub fn scope_description(scope: SkillsDirectoryScope) -> &'static str {
    match scope {
        SkillsDirectoryScope::Project => "当前项目 skills、.agents/skills 与所选 provider 目录。",
        SkillsDirectoryScope::Ancestor => {
            "从项目父级向上发现的 skills、.agents/skills 与所选 provider 目录。"
        }
        SkillsDirectoryScope::Global => {
            #[cfg(debug_assertions)]
            {
                "当前用户 ~/.shiju-harness-dev/skills。"
            }
            #[cfg(not(debug_assertions))]
            {
                "当前用户 ~/.shiju-harness/skills。"
            }
        }
        SkillsDirectoryScope::Bundled => "当前页面仅显示已安装到技能目录的技能。",
        SkillsDirectoryScope::All => "按加载顺序汇总全部技能来源。",
    }
}

pub fn discovery_order_text(input: &Input<'_>) -> String {
    if let Some(project_path) = &input.skill_directory {
        desktop_i18n::__!(
            desktop_i18n::I18n::new(input.locale),
            "发现顺序：%s/skills -> %s/.agents/skills -> 父级 skills、.agents/skills -> %s/skills",
            project_path.as_str(),
            project_path.as_str(),
            config_runtime_paths::tilde_config_path("").as_str()
        )
        .expect("valid discovery order template")
    } else {
        config_runtime_paths::tilde_config_path("skills")
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
        button = button.on_press(Message::Agents(
            AgentsMessage::SkillDirectoryScopeChanged(scope),
        ));
    }

    button
        .style(move |theme: &Theme, status| settings_segment_button_style(theme, status, is_active))
        .into()
}

pub fn enabled_skill_ids(skills: &[&SkillsCatalogItem]) -> Vec<String> {
    skills.iter().filter(|skill| skill.enabled).map(|skill| skill.id.clone()).collect()
}

pub fn selected_skill_ids(entry: &DelegateAgentSettingsEntry) -> &[String] {
    match &entry.defaults.skills {
        AgentDefault::Inherit => &entry.allowed_skills,
        AgentDefault::Fixed(ids) => ids,
    }
}

fn skill_card<'a>(
    locale: DesktopLocale,
    entry: &'a DelegateAgentSettingsEntry,
    skill: &'a SkillsCatalogItem,
) -> Element<'a, Message> {
    let checked = selected_skill_ids(entry).iter().any(|selected| selected == &skill.id);
    let agent_key = entry.key.clone();
    let skill_id = skill.id.clone();
    let source = tr(locale, skill_source_label(&skill.source));
    let status = if skill.enabled {
        source.to_string()
    } else {
        desktop_i18n::__!(desktop_i18n::I18n::new(locale), "%s · 已停用", source)
            .expect("valid skill status template")
    };

    let selectable = matches!(entry.defaults.skills, AgentDefault::Fixed(_))
        && (checked
            || (skill.enabled
                && (entry.skill_policy == DelegateCapabilityPolicy::Unrestricted
                    || entry.allowed_skills.contains(&skill.id))));
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
    .on_press_maybe(selectable.then_some(Message::Agents(
        AgentsMessage::AllowedSkillToggled(agent_key, skill_id, !checked),
    )))
    .into()
}

#[derive(Clone, PartialEq)]
enum SkillDirectory {
    Global(&'static str),
    Project(String),
}

impl fmt::Display for SkillDirectory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Global(label) => f.write_str(label),
            Self::Project(path) => f.write_str(path),
        }
    }
}

pub fn view<'a>(
    input: Input<'a>,
    entry: &'a DelegateAgentSettingsEntry,
) -> Element<'a, Message> {
    let i18n = I18n::new(input.locale);
    let active_scope = agent_skills_scope(input.skill_directory_scope);
    let skills = input
        .skill_catalog
        .iter()
        .filter(|skill| agent_skill_matches_scope(skill, active_scope))
        .collect::<Vec<_>>();
    let enabled_skills = enabled_skill_ids(&skills);
    let selected_count = selected_skill_ids(entry)
        .iter()
        .filter(|skill| enabled_skills.iter().any(|available| available == *skill))
        .count();
    let fixed = matches!(entry.defaults.skills, AgentDefault::Fixed(_));
    let project_open = input.skill_directory.is_some();
    let global = SkillDirectory::Global(tr(input.locale, "全局目录"));
    let mut projects = input.project_options.to_vec();
    if let Some(path) = &input.skill_directory {
        if !projects.contains(path) {
            projects.push(path.clone());
        }
    }
    let selected = input
        .skill_directory
        .clone()
        .map(SkillDirectory::Project)
        .unwrap_or_else(|| global.clone());
    let directories =
        std::iter::once(global).chain(projects.into_iter().map(SkillDirectory::Project));
    let directory_picker = settings_select(directories, Some(selected), |value| {
        let path = match value {
            SkillDirectory::Global(_) => None,
            SkillDirectory::Project(path) => Some(path),
        };
        Message::Agents(AgentsMessage::SkillDirectorySelected(path))
    })
    .width(Length::Fill);
    let directory_label = tr(input.locale, "技能目录");
    let scope_filter = column![
        settings_hover_help_label(input.help,
            i18n,
            input.theme,
            directory_label,
            settings_field_label(directory_label),
            tr(input.locale, scope_description(active_scope)),
        )
        .wrap(),
        directory_picker,
        input.skill_catalog_error.as_ref().map(settings_error_banner),
        row![
            scope_button(
                tr(input.locale, "项目目录"),
                SkillsDirectoryScope::Project,
                active_scope,
                project_open
            ),
            scope_button(
                tr(input.locale, "父级目录"),
                SkillsDirectoryScope::Ancestor,
                active_scope,
                project_open
            ),
            scope_button(
                tr(input.locale, "全局目录"),
                SkillsDirectoryScope::Global,
                active_scope,
                true
            ),
            scope_button(
                tr(input.locale, "全部"),
                SkillsDirectoryScope::All,
                active_scope,
                true
            ),
        ]
        .spacing(8)
        .wrap(),
        text(discovery_order_text(&input))
            .size(11)
            .width(Length::Fill)
            .wrapping(text::Wrapping::WordOrGlyph)
            .style(settings_muted_text_style),
    ]
    .spacing(8);

    let skills_list: Element<'_, Message> = if skills.is_empty() {
        column![
            text(if input.skill_catalog_loading {
                tr(input.locale, "正在刷新技能目录 ...")
            } else {
                tr(input.locale, "当前筛选范围暂无可用技能")
            })
            .size(12)
            .style(settings_muted_text_style),
            button(text(tr(input.locale, "刷新技能")).size(12))
                .padding([8, 14])
                .on_press(Message::Agents(
                    AgentsMessage::SkillCatalogRefresh
                ))
                .style(rounded_action_btn_style),
        ]
        .spacing(10)
        .into()
    } else {
        let cards = skills
            .iter()
            .map(|skill| skill_card(input.locale, entry, *skill))
            .collect::<Vec<_>>();

        column![
            row![
                button(text(tr(input.locale, "全选可用")).size(12))
                    .padding([8, 14])
                    .on_press_maybe(fixed.then_some(Message::Agents(
                        AgentsMessage::AllowedSkillsSelectAll(entry.key.clone(), active_scope),
                    )))
                    .style(rounded_action_btn_style),
                button(text(tr(input.locale, "反选可用")).size(12))
                    .padding([8, 14])
                    .on_press_maybe(fixed.then_some(Message::Agents(
                        AgentsMessage::AllowedSkillsInvertSelection(
                            entry.key.clone(),
                            active_scope
                        ),
                    )))
                    .style(rounded_action_btn_style),
                settings_value_badge(
                    desktop_i18n::__!(
                        desktop_i18n::I18n::new(input.locale),
                        "已选 %u/%u",
                        selected_count,
                        enabled_skills.len()
                    )
                    .expect("valid skill count template")
                ),
            ]
            .spacing(10)
            .align_y(Alignment::Center)
            .wrap(),
            row(cards).spacing(12).wrap(),
        ]
        .spacing(12)
        .into()
    };
    let original = entry.defaults.clone();
    let mode = super::defaults::selection_mode(input.locale, fixed, move |fixed| {
        let mut defaults = original.clone();
        defaults.skills = if fixed { AgentDefault::Fixed(vec![]) } else { AgentDefault::Inherit };
        Message::Agents(AgentsMessage::ProfileDefaultsChanged(defaults))
    });
    let policy_controls = row![
        skill_policy_button(input.locale, entry, DelegateCapabilityPolicy::Strict),
        skill_policy_button(input.locale, entry, DelegateCapabilityPolicy::Unrestricted),
    ]
    .spacing(8)
    .wrap();

    column![
        settings_group_heading_with_help(input.help,
            i18n,
            input.theme,
            tr(input.locale, "默认技能"),
            tr(
                input.locale,
                "继承预览原有允许技能；固定空列表不预选技能。默认选择不会扩大严格访问范围。"
            ),
        ),
        settings_group_card(
            column![
                mode,
                settings_flexible_row_with_help(input.help,
                    i18n,
                    input.theme,
                    tr(input.locale, "技能访问策略"),
                    tr(input.locale, "严格限制保留原有允许范围；不限制允许会话临时选择。"),
                    320.0,
                    policy_controls,
                )
                .padding([6, 0]),
                button(text(tr(input.locale, "前往技能库管理")).size(12))
                    .padding([8, 14])
                    .style(rounded_action_btn_style)
                    .on_press(Message::OpenSkills),
            ]
            .spacing(8)
        )
        .padding([14, 16]),
        settings_group_card(scope_filter).padding([14, 16]),
        skills_list,
    ]
    .spacing(14)
    .into()
}

pub struct Input<'a> { pub locale: DesktopLocale, pub theme: &'a Theme, pub help: crate::help::Renderer, pub project_options: Vec<String>, pub skill_directory: &'a Option<String>, pub skill_directory_scope: SkillsDirectoryScope, pub skill_catalog: &'a [SkillsCatalogItem], pub skill_catalog_loading: bool, pub skill_catalog_error: &'a Option<String> }
