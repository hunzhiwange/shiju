//! 选择聊天工具输出的具体渲染组件。
//! 模块把工具名称和渲染函数绑定在一处，便于新增视图时保持边界清晰。

use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::svg;
use iced::widget::tooltip::{Position as TooltipPosition, Tooltip};
use iced::widget::{Space, button, column, container, row, scrollable, text};
use iced::{Alignment, Background, Border, Color, Element, Length, Theme};

use super::utils::icon_svg;
use crate::app::assets::Icon;
use crate::app::components::input_panel::styles::{
    popover_style, selectable_list_button_style, tooltip_dark_style,
};
use crate::app::state::{
    MAIN_AGENT_KEY, SessionToolGroup, SessionToolSelectorTab, SkillsDirectoryScope,
    tool_display_name, tool_group,
};
use crate::app::{App, Message, message};

pub(super) const SESSION_SELECTOR_SCROLLBAR_WIDTH: f32 = 4.0;
pub(super) const SESSION_SELECTOR_LIST_MAX_HEIGHT: f32 = 300.0;
const SESSION_SELECTOR_LIST_RIGHT_PADDING: f32 = 5.0;
pub(super) const SESSION_SELECTOR_SKILL_DESCRIPTION_CHARS: usize = 28;

pub(super) fn count_badge<'a>(label: String, active: bool) -> Element<'a, Message> {
    container(text(label).size(10))
        .padding([3, 8])
        .style(move |theme: &Theme| {
            let palette = theme.extended_palette();
            let is_dark = theme.palette().background.r
                + theme.palette().background.g
                + theme.palette().background.b
                < 1.5;
            let color =
                if active { palette.primary.base.color } else { palette.background.strong.color };
            let text_color = if active && !is_dark {
                Color::from_rgba8(34, 38, 48, 0.94)
            } else if active {
                palette.primary.base.text
            } else {
                theme.palette().text.scale_alpha(0.82)
            };
            iced::widget::container::Style {
                background: Some(Background::Color(color.scale_alpha(if active {
                    if is_dark { 0.18 } else { 0.16 }
                } else if is_dark {
                    0.32
                } else {
                    0.52
                }))),
                border: Border { width: 0.0, color: Color::TRANSPARENT, radius: 999.0.into() },
                text_color: Some(text_color),
                ..Default::default()
            }
        })
        .into()
}

pub(super) fn tab_button(tab: SessionToolSelectorTab, selected: bool) -> Element<'static, Message> {
    button(text(tab.label()).size(12))
        .padding([6, 10])
        .style(move |theme: &Theme, status| selectable_list_button_style(theme, status, selected))
        .on_press(Message::Chat(message::ChatMessage::SessionToolSelectorTabSelected(tab)))
        .into()
}

pub(super) fn is_dark_theme(theme: &Theme) -> bool {
    theme.palette().background.r + theme.palette().background.g + theme.palette().background.b < 1.5
}

pub(super) fn context_row_text_color(theme: &Theme, selected: bool, alpha: f32) -> Color {
    if selected && !is_dark_theme(theme) {
        Color::from_rgba8(28, 32, 40, alpha)
    } else {
        theme.palette().text.scale_alpha(alpha)
    }
}

pub(super) fn context_list_button_style(
    theme: &Theme,
    status: iced::widget::button::Status,
    selected: bool,
) -> iced::widget::button::Style {
    let mut style = selectable_list_button_style(theme, status, selected);
    if selected {
        style.text_color = context_row_text_color(theme, true, 0.96);
    }
    style
}

pub(super) fn search_text_input_style(
    theme: &Theme,
    status: iced::widget::text_input::Status,
) -> iced::widget::text_input::Style {
    let palette = theme.palette();
    let disabled = matches!(status, iced::widget::text_input::Status::Disabled);

    iced::widget::text_input::Style {
        background: Background::Color(Color::TRANSPARENT),
        border: Border { width: 0.0, color: Color::TRANSPARENT, radius: 0.0.into() },
        icon: palette.text.scale_alpha(0.65),
        placeholder: palette.text.scale_alpha(0.48),
        value: if disabled { palette.text.scale_alpha(0.50) } else { palette.text },
        selection: palette.primary.scale_alpha(0.22),
    }
}

pub(super) fn selector_search_bar(query: String) -> Element<'static, Message> {
    container(
        row![
            icon_svg(Icon::Search).width(Length::Fixed(14.0)).height(Length::Fixed(14.0)).style(
                |theme: &Theme, _status| svg::Style {
                    color: Some(theme.palette().text.scale_alpha(0.60)),
                }
            ),
            iced::widget::TextInput::new("搜索当前列表", query.as_str())
                .on_input(|value| {
                    Message::Chat(message::ChatMessage::SessionToolSelectorSearchChanged(value))
                })
                .padding([6, 0])
                .size(12)
                .style(search_text_input_style)
                .width(Length::Fill)
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    )
    .padding([0, 8])
    .width(Length::Fill)
    .style(|theme: &Theme| {
        let palette = theme.extended_palette();
        iced::widget::container::Style {
            text_color: Some(theme.palette().text),
            background: Some(Background::Color(palette.background.weak.color.scale_alpha(0.20))),
            border: Border {
                width: 1.0,
                color: palette.background.strong.color.scale_alpha(0.58),
                radius: 8.0.into(),
            },
            ..Default::default()
        }
    })
    .into()
}

pub(super) fn matches_query(query: &str, parts: &[&str]) -> bool {
    let query = query.trim().to_ascii_lowercase();
    if query.is_empty() {
        return true;
    }

    parts.iter().any(|part| part.to_ascii_lowercase().contains(&query))
}

pub(super) fn ellipsize_text(value: &str, max_chars: usize) -> String {
    let compact = value.split_whitespace().collect::<Vec<_>>().join(" ");
    if compact.chars().count() <= max_chars {
        return compact;
    }

    let keep = max_chars.saturating_sub(3);
    let mut clipped = compact.chars().take(keep).collect::<String>();
    clipped.push_str("...");
    clipped
}

pub(super) fn skill_tooltip_content(
    title: String,
    skill_id: String,
    source: String,
    description: String,
) -> Element<'static, Message> {
    let description =
        if description.trim().is_empty() { "暂无技能介绍".to_string() } else { description };

    container(
        column![
            text(title)
                .size(12)
                .style(|_theme: &Theme| iced::widget::text::Style { color: Some(Color::WHITE) }),
            text(format!("{skill_id} · {source}")).size(10).style(|_theme: &Theme| {
                iced::widget::text::Style { color: Some(Color::WHITE.scale_alpha(0.70)) }
            }),
            text(description).size(11).width(Length::Fixed(300.0)).style(|_theme: &Theme| {
                iced::widget::text::Style { color: Some(Color::WHITE.scale_alpha(0.88)) }
            }),
        ]
        .spacing(4),
    )
    .padding([7, 9])
    .style(tooltip_dark_style)
    .into()
}

pub(super) fn skill_scope_source_matches(scope: SkillsDirectoryScope, source: &str) -> bool {
    match scope {
        SkillsDirectoryScope::Project => source == "workspace",
        SkillsDirectoryScope::Ancestor => source == "ancestor",
        SkillsDirectoryScope::Global => source == "global",
        SkillsDirectoryScope::Bundled => source == "bundled",
        SkillsDirectoryScope::All => true,
    }
}

pub(super) fn skill_scope_label(scope: SkillsDirectoryScope) -> &'static str {
    match scope {
        SkillsDirectoryScope::Project => "项目",
        SkillsDirectoryScope::Ancestor => "父级",
        SkillsDirectoryScope::Global => "全局",
        SkillsDirectoryScope::Bundled => "内置",
        SkillsDirectoryScope::All => "全部",
    }
}

pub(super) fn skill_scope_button(
    scope: SkillsDirectoryScope,
    active_scope: SkillsDirectoryScope,
) -> Element<'static, Message> {
    let selected = scope == active_scope;
    button(text(skill_scope_label(scope)).size(11))
        .padding([5, 8])
        .style(move |theme: &Theme, status| selectable_list_button_style(theme, status, selected))
        .on_press(Message::Chat(
            message::ChatMessage::SessionToolSelectorSkillDirectoryScopeChanged(scope),
        ))
        .into()
}

/// 执行 session_tool_selector_popover 对应的模块功能。
///
/// 参数含义保持与函数签名一致；返回值用于调用方继续组合处理，错误由返回类型显式表达。
pub(crate) fn session_tool_selector_popover(app: &App) -> Element<'_, Message> {
    let runtime = app.current_session_runtime();
    let mut manual_tools = app
        .agents_settings
        .available_tools
        .iter()
        .filter_map(|tool| {
            let trimmed = tool.trim();
            (!trimmed.is_empty()).then(|| trimmed.to_string())
        })
        .collect::<Vec<_>>();
    manual_tools.sort();
    manual_tools.dedup();
    let query = runtime.tool_selector.query().to_string();

    let selected_agent_key = runtime.agent.as_deref().unwrap_or(MAIN_AGENT_KEY);
    let available_agents = app
        .agents_settings
        .entries
        .iter()
        .filter(|entry| {
            entry.key == MAIN_AGENT_KEY
                || entry.enabled
                || runtime.agent.as_deref() == Some(entry.key.as_str())
        })
        .filter(|entry| {
            matches_query(&query, &[entry.key.as_str(), entry.label.as_str(), entry.model.as_str()])
        })
        .collect::<Vec<_>>();
    let filtered_manual_tools = manual_tools
        .iter()
        .filter(|tool_id| {
            let display_name = tool_display_name(tool_id);
            matches_query(&query, &[tool_id.as_str(), display_name.as_str()])
        })
        .cloned()
        .collect::<Vec<_>>();
    let usable_skill_count =
        app.skills_settings.catalog.iter().filter(|skill| skill.installed && skill.enabled).count();
    let selected_tool_count = runtime.tool_selector.selected_manual_tools().len();
    let selected_skill_count = runtime.tool_selector.selected_manual_skills().len();
    let summary = if manual_tools.is_empty() {
        format!(
            "可切换 {} 个代理；工具清单等待网关返回；已选 {} 个技能。",
            available_agents.len(),
            selected_skill_count,
        )
    } else {
        format!(
            "可切换 {} 个代理；可选 {} 个工具 / {} 个技能；已选 {} 个工具 / {} 个技能。",
            available_agents.len(),
            manual_tools.len(),
            usable_skill_count,
            selected_tool_count,
            selected_skill_count,
        )
    };
    let scope_hint = match runtime.tool_selector.active_tab() {
        SessionToolSelectorTab::Agent => "代理只决定本会话委托目标，不改写系统设置。",
        SessionToolSelectorTab::Tools => "工具会随下一次发送写入提示词，可多选，不会立即执行。",
        SessionToolSelectorTab::Skills => "技能会随下一次发送写入提示词，可多选，不会立即加载。",
    };

    let tabs = row![
        tab_button(
            SessionToolSelectorTab::Agent,
            runtime.tool_selector.active_tab() == SessionToolSelectorTab::Agent,
        ),
        tab_button(
            SessionToolSelectorTab::Tools,
            runtime.tool_selector.active_tab() == SessionToolSelectorTab::Tools,
        ),
        tab_button(
            SessionToolSelectorTab::Skills,
            runtime.tool_selector.active_tab() == SessionToolSelectorTab::Skills,
        )
    ]
    .spacing(6)
    .align_y(Alignment::Center);

    let body: Element<'_, Message> = match runtime.tool_selector.active_tab() {
        SessionToolSelectorTab::Agent => {
            let mut list = column![].spacing(6);
            let mut has_agents = false;
            for entry in available_agents {
                has_agents = true;
                let selected = selected_agent_key == entry.key;
                let allowlist_text = if entry.allowed_tools.is_empty() {
                    "未声明默认工具范围".to_string()
                } else {
                    format!("默认工具范围 {} 项", entry.allowed_tools.len())
                };
                let subtitle = if entry.model.trim().is_empty() {
                    allowlist_text
                } else {
                    format!("{} / {}", entry.model, allowlist_text)
                };
                let check: Element<'_, Message> = if selected {
                    icon_svg(Icon::Check)
                        .width(Length::Fixed(14.0))
                        .height(Length::Fixed(14.0))
                        .style(|theme: &Theme, _status| svg::Style {
                            color: Some(theme.palette().text.scale_alpha(0.92)),
                        })
                        .into()
                } else {
                    Space::new().width(Length::Fixed(14.0)).into()
                };
                let agent_for_press =
                    if entry.key == MAIN_AGENT_KEY { None } else { Some(entry.key.clone()) };
                list = list.push(
                    button(
                        row![
                            column![
                                text(entry.label.clone()).size(13),
                                text(subtitle).size(11).style(|theme: &Theme| {
                                    iced::widget::text::Style {
                                        color: Some(theme.palette().text.scale_alpha(0.62)),
                                    }
                                })
                            ]
                            .spacing(3)
                            .width(Length::Fill),
                            check
                        ]
                        .spacing(10)
                        .align_y(Alignment::Center),
                    )
                    .padding([8, 10])
                    .width(Length::Fill)
                    .style(move |theme: &Theme, status| {
                        selectable_list_button_style(theme, status, selected)
                    })
                    .on_press(Message::Chat(
                        message::ChatMessage::SessionAgentSelected(agent_for_press),
                    )),
                );
            }
            if !has_agents {
                let empty_message = if query.trim().is_empty() {
                    "当前没有可用于会话切换的 delegate agent。"
                } else {
                    "没有匹配的代理。"
                };
                list = list.push(
                    container(text(empty_message).size(11).style(|theme: &Theme| {
                        iced::widget::text::Style {
                            color: Some(theme.palette().text.scale_alpha(0.62)),
                        }
                    }))
                    .padding([4, 2]),
                );
            }
            scrollable(container(list).padding(iced::Padding {
                top: 0.0,
                right: SESSION_SELECTOR_LIST_RIGHT_PADDING,
                bottom: 0.0,
                left: 0.0,
            }))
            .direction(Direction::Vertical(
                Scrollbar::new()
                    .width(SESSION_SELECTOR_SCROLLBAR_WIDTH)
                    .scroller_width(SESSION_SELECTOR_SCROLLBAR_WIDTH),
            ))
            .height(Length::Fixed(SESSION_SELECTOR_LIST_MAX_HEIGHT))
            .into()
        }
        SessionToolSelectorTab::Tools => {
            if filtered_manual_tools.is_empty() {
                let empty_message = if manual_tools.is_empty() {
                    "等待网关返回可用工具列表。"
                } else {
                    "没有匹配的工具。"
                };
                container(text(empty_message).size(11).style(|theme: &Theme| {
                    iced::widget::text::Style {
                        color: Some(theme.palette().text.scale_alpha(0.62)),
                    }
                }))
                .padding([4, 2])
                .into()
            } else {
                let mut groups = column![].spacing(10);
                for group in SessionToolGroup::ALL {
                    let total = filtered_manual_tools
                        .iter()
                        .filter(|tool_id| tool_group(tool_id) == group)
                        .count();
                    if total == 0 {
                        continue;
                    }
                    let selected = filtered_manual_tools
                        .iter()
                        .filter(|tool_id| {
                            tool_group(tool_id) == group
                                && runtime.tool_selector.is_manual_tool_selected(tool_id)
                        })
                        .count();
                    let collapsed = runtime.tool_selector.is_group_collapsed(group);
                    let group_tools = filtered_manual_tools
                        .iter()
                        .filter(|tool_id| tool_group(tool_id) == group)
                        .cloned()
                        .collect::<Vec<_>>();
                    let group_title = format!("{} {}", group.label(), group.description());
                    let header = row![
                        button(
                            row![
                                icon_svg(if collapsed {
                                    Icon::ChevronRight
                                } else {
                                    Icon::ChevronDown
                                })
                                .width(Length::Fixed(12.0))
                                .height(Length::Fixed(12.0))
                                .style(
                                    |theme: &Theme, _status| svg::Style {
                                        color: Some(theme.palette().text.scale_alpha(0.72)),
                                    }
                                ),
                                text(group_title)
                                    .size(12)
                                    .width(Length::Fill)
                                    .wrapping(iced::widget::text::Wrapping::None)
                                    .style(|theme: &Theme| {
                                        iced::widget::text::Style {
                                            color: Some(theme.palette().text.scale_alpha(0.78)),
                                        }
                                    })
                            ]
                            .spacing(8)
                            .align_y(Alignment::Center),
                        )
                        .width(Length::Fill)
                        .padding([0, 0])
                        .style(|_theme: &Theme, _status| iced::widget::button::Style {
                            background: None,
                            border: Border::default(),
                            text_color: Color::TRANSPARENT,
                            ..Default::default()
                        })
                        .on_press(Message::Chat(
                            message::ChatMessage::SessionToolGroupCollapsedToggled(group),
                        )),
                        Space::new().width(Length::Fill),
                        count_badge(format!("{selected} / {total}"), selected > 0)
                    ]
                    .spacing(8)
                    .align_y(Alignment::Center);

                    let mut section = column![header].spacing(6);
                    if !collapsed {
                        let mut rows = column![].spacing(4);
                        for tool_id in group_tools {
                            let selected = runtime.tool_selector.is_manual_tool_selected(&tool_id);
                            let tool_label =
                                format!("{} · {}", tool_display_name(&tool_id), tool_id);
                            let check: Element<'_, Message> = if selected {
                                icon_svg(Icon::Check)
                                    .width(Length::Fixed(14.0))
                                    .height(Length::Fixed(14.0))
                                    .style(|theme: &Theme, _status| svg::Style {
                                        color: Some(context_row_text_color(theme, true, 0.92)),
                                    })
                                    .into()
                            } else {
                                Space::new().width(Length::Fixed(14.0)).into()
                            };
                            rows = rows.push(
                                button(
                                    row![
                                        text(tool_label)
                                            .size(12)
                                            .width(Length::Fill)
                                            .wrapping(iced::widget::text::Wrapping::None)
                                            .style(move |theme: &Theme| {
                                                iced::widget::text::Style {
                                                    color: Some(context_row_text_color(
                                                        theme, selected, 0.86,
                                                    )),
                                                }
                                            }),
                                        check
                                    ]
                                    .spacing(10)
                                    .align_y(Alignment::Center),
                                )
                                .padding([7, 10])
                                .width(Length::Fill)
                                .style(move |theme: &Theme, status| {
                                    context_list_button_style(theme, status, selected)
                                })
                                .on_press(Message::Chat(
                                    message::ChatMessage::SessionManualToolSelected(tool_id),
                                )),
                            );
                        }
                        section = section.push(container(rows).padding(iced::Padding {
                            top: 0.0,
                            right: 0.0,
                            bottom: 0.0,
                            left: 0.0,
                        }));
                    }
                    groups = groups.push(section);
                }

                scrollable(container(groups).padding(iced::Padding {
                    top: 0.0,
                    right: SESSION_SELECTOR_LIST_RIGHT_PADDING,
                    bottom: 0.0,
                    left: 0.0,
                }))
                .direction(Direction::Vertical(
                    Scrollbar::new()
                        .width(SESSION_SELECTOR_SCROLLBAR_WIDTH)
                        .scroller_width(SESSION_SELECTOR_SCROLLBAR_WIDTH),
                ))
                .height(Length::Fixed(SESSION_SELECTOR_LIST_MAX_HEIGHT))
                .into()
            }
        }
        SessionToolSelectorTab::Skills => {
            if app.skills_settings.loading && app.skills_settings.catalog.is_empty() {
                container(text("技能目录加载中。").size(11).style(|theme: &Theme| {
                    iced::widget::text::Style {
                        color: Some(theme.palette().text.scale_alpha(0.62)),
                    }
                }))
                .padding([4, 2])
                .into()
            } else {
                let skill_scope = runtime.tool_selector.skill_directory_scope();
                let scope_tabs = row![
                    skill_scope_button(SkillsDirectoryScope::Project, skill_scope),
                    skill_scope_button(SkillsDirectoryScope::Ancestor, skill_scope),
                    skill_scope_button(SkillsDirectoryScope::Global, skill_scope),
                    skill_scope_button(SkillsDirectoryScope::Bundled, skill_scope),
                    skill_scope_button(SkillsDirectoryScope::All, skill_scope),
                ]
                .spacing(6)
                .align_y(Alignment::Center);
                let mut list = column![].spacing(4);
                let mut has_skills = false;
                let mut has_scope_skills = false;
                for skill in app
                    .skills_settings
                    .catalog
                    .iter()
                    .filter(|skill| skill.installed && skill.enabled)
                    .filter(|skill| skill_scope_source_matches(skill_scope, &skill.source))
                {
                    has_scope_skills = true;
                    if !matches_query(
                        &query,
                        &[
                            skill.id.as_str(),
                            skill.title.as_str(),
                            skill.description.as_str(),
                            skill.source.as_str(),
                            skill.source_path.as_deref().unwrap_or_default(),
                        ],
                    ) {
                        continue;
                    }
                    has_skills = true;
                    let skill_id = skill.id.clone();
                    let selected = runtime.tool_selector.is_manual_skill_selected(&skill_id);
                    let subtitle = if skill.description.trim().is_empty() {
                        skill.id.clone()
                    } else {
                        format!(
                            "{} · {}",
                            skill.id,
                            ellipsize_text(
                                &skill.description,
                                SESSION_SELECTOR_SKILL_DESCRIPTION_CHARS,
                            )
                        )
                    };
                    let tooltip_content = skill_tooltip_content(
                        skill.title.clone(),
                        skill_id.clone(),
                        skill.source.clone(),
                        skill.description.clone(),
                    );
                    let check: Element<'_, Message> = if selected {
                        icon_svg(Icon::Check)
                            .width(Length::Fixed(14.0))
                            .height(Length::Fixed(14.0))
                            .style(|theme: &Theme, _status| svg::Style {
                                color: Some(context_row_text_color(theme, true, 0.92)),
                            })
                            .into()
                    } else {
                        Space::new().width(Length::Fixed(14.0)).into()
                    };
                    let skill_button = button(
                        row![
                            column![
                                text(ellipsize_text(&skill.title, 34))
                                    .size(13)
                                    .width(Length::Fill)
                                    .wrapping(iced::widget::text::Wrapping::None)
                                    .style(move |theme: &Theme| {
                                        iced::widget::text::Style {
                                            color: Some(context_row_text_color(
                                                theme, selected, 0.90,
                                            )),
                                        }
                                    }),
                                text(subtitle)
                                    .size(11)
                                    .width(Length::Fill)
                                    .wrapping(iced::widget::text::Wrapping::None)
                                    .style(move |theme: &Theme| iced::widget::text::Style {
                                        color: Some(context_row_text_color(theme, selected, 0.66)),
                                    })
                            ]
                            .spacing(3)
                            .width(Length::Fill),
                            check
                        ]
                        .spacing(10)
                        .align_y(Alignment::Center),
                    )
                    .padding([8, 10])
                    .width(Length::Fill)
                    .style(move |theme: &Theme, status| {
                        context_list_button_style(theme, status, selected)
                    })
                    .on_press(Message::Chat(message::ChatMessage::SessionSkillSelected(skill_id)));
                    list = list.push(
                        Tooltip::new(skill_button, tooltip_content, TooltipPosition::Right)
                            .gap(8.0),
                    );
                }

                if !has_skills {
                    let empty_message = if app.skills_settings.catalog.is_empty() {
                        "技能目录尚未加载。"
                    } else if !has_scope_skills {
                        "当前目录筛选下没有已安装且启用的技能。"
                    } else {
                        "没有匹配的技能。"
                    };
                    list = list.push(
                        container(text(empty_message).size(11).style(|theme: &Theme| {
                            iced::widget::text::Style {
                                color: Some(theme.palette().text.scale_alpha(0.62)),
                            }
                        }))
                        .padding([4, 2]),
                    );
                }

                let content = column![
                    scope_tabs,
                    scrollable(container(list).padding(iced::Padding {
                        top: 0.0,
                        right: SESSION_SELECTOR_LIST_RIGHT_PADDING,
                        bottom: 0.0,
                        left: 0.0,
                    }))
                    .direction(Direction::Vertical(
                        Scrollbar::new()
                            .width(SESSION_SELECTOR_SCROLLBAR_WIDTH)
                            .scroller_width(SESSION_SELECTOR_SCROLLBAR_WIDTH),
                    ),)
                    .height(Length::Fixed(SESSION_SELECTOR_LIST_MAX_HEIGHT))
                ]
                .spacing(8);

                content.into()
            }
        }
    };

    container(
        column![
            text("会话控制").size(13),
            text(summary).size(11).style(|theme: &Theme| iced::widget::text::Style {
                color: Some(theme.palette().text.scale_alpha(0.84)),
            }),
            text(scope_hint).size(10).style(|theme: &Theme| iced::widget::text::Style {
                color: Some(theme.palette().text.scale_alpha(0.62)),
            }),
            tabs,
            selector_search_bar(query),
            body,
        ]
        .spacing(10),
    )
    .padding([10, 12])
    .width(Length::Fixed(360.0))
    .style(popover_style)
    .into()
}
