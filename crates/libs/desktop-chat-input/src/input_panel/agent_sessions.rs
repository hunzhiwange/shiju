//! 展示代理子会话与计划运行会话的状态弹层和控制入口。

use std::collections::{HashMap, HashSet};

use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::tooltip::{Position as TooltipPosition, Tooltip};
use iced::widget::{Space, button, column, container, row, scrollable, text};
use iced::{Alignment, Background, Border, Color, Element, Length, Theme};

use desktop_ui_controls::overlays::AboveOverlay;
use desktop_ui_controls::status_animation::spinner_frame;
use super::agent_children::{
    AgentChildSession, AgentChildSessionKind, AgentChildSessionStatus, PlanRunSessionControl,
    agent_child_session_projection, plan_run_session_controls_from_content,
};
use desktop_i18n::I18n;
use session_ui_types::ChatMessage;
use std::sync::Arc;
use session_info::Info;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action { StopPlanRun(String), ToggleAgentSessionsPopover, CloseAgentSessionsPopover }

const POPOVER_WIDTH: f32 = 340.0;
const POPOVER_MAX_HEIGHT: f32 = 248.0;
const SCROLLBAR_WIDTH: f32 = 4.0;
const TITLE_MAX_CHARS: usize = 42;
const TASK_MAX_CHARS: usize = 58;
const PLANNING_TITLE_PREFIX: &str = "规划 · ";

fn theme_is_dark(theme: &Theme) -> bool {
    let background = theme.palette().background;
    background.r + background.g + background.b < 1.5
}

fn compact_and_truncate(value: &str, max_chars: usize) -> String {
    let compact = value.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut chars = compact.chars();
    let truncated = chars.by_ref().take(max_chars).collect::<String>();
    if chars.next().is_some() { format!("{truncated}…") } else { truncated }
}

fn planning_row_label(title: &str, session_id: &str) -> String {
    let task_id = title
        .trim()
        .strip_prefix(PLANNING_TITLE_PREFIX)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .or_else(|| session_id.trim().strip_suffix("-plan"))
        .filter(|value| !value.is_empty())
        .unwrap_or(session_id.trim());
    format!("规划 · {task_id} · 计划任务")
}

/// 将子会话状态映射为用户可见标签；计划会话额外区分规划阶段。
pub fn agent_session_status_label(
    kind: AgentChildSessionKind,
    status: AgentChildSessionStatus,
) -> &'static str {
    if status == AgentChildSessionStatus::Waiting {
        return "等待中";
    }
    match kind {
        AgentChildSessionKind::WorkflowPlanning => "正在规划",
        AgentChildSessionKind::Agent | AgentChildSessionKind::WorkflowExecution => "运行中",
    }
}

fn known_sessions<'a>(sessions: impl IntoIterator<Item = &'a Info>) -> Vec<&'a Info> {
    let mut seen = HashSet::<&str>::new();
    sessions.into_iter().filter(|session| seen.insert(session.id.as_str())).collect()
}

fn plan_run_controls(chat: &[ChatMessage], cached_chat_lookup: &impl Fn(&str) -> Option<Arc<[ChatMessage]>>, sessions: &[&Info]) -> HashMap<String, PlanRunSessionControl> {
    let mut controls = HashMap::new();
    for message in chat {
        for control in plan_run_session_controls_from_content(&message.content) {
            let previous_task_path = controls
                .get(&control.session_id)
                .and_then(|previous: &PlanRunSessionControl| previous.task_path.clone());
            let mut control = control;
            if control.task_path.is_none() {
                control.task_path = previous_task_path;
            }
            controls.insert(control.session_id.clone(), control);
        }
    }
    for session in sessions {
        let Some(chat) = cached_chat_lookup(&session.id) else {
            continue;
        };
        for message in chat.iter() {
            for control in plan_run_session_controls_from_content(&message.content) {
                let previous_task_path = controls
                    .get(&control.session_id)
                    .and_then(|previous| previous.task_path.clone());
                let mut control = control;
                if control.task_path.is_none() {
                    control.task_path = previous_task_path;
                }
                controls.insert(control.session_id.clone(), control);
            }
        }
    }
    controls
}

fn append_running_descendants(
    active_session_id: Option<&str>, current_chat: &[ChatMessage],
    cached_chat_lookup: &impl Fn(&str) -> Option<Arc<[ChatMessage]>>,
    requesting: &impl Fn(&str) -> bool, session_title: &impl Fn(&str, &str) -> String,
    parent_session_id: &str,
    depth: usize,
    sessions: &[&Info],
    controls: &HashMap<String, PlanRunSessionControl>,
    visited: &mut HashSet<String>,
    projected: &mut Vec<AgentChildSession>,
) {
    let cached_chat = cached_chat_lookup(parent_session_id);
    let chat = if active_session_id == Some(parent_session_id) {
        current_chat
    } else {
        cached_chat.as_deref().unwrap_or_default()
    };
    let agent_children =
        agent_child_session_projection(Some(parent_session_id), chat, sessions.iter().copied());
    for mut child in agent_children {
        if !visited.insert(child.session_id.clone()) {
            continue;
        }
        child.depth = depth;
        let child_session_id = child.session_id.clone();
        projected.push(child);
        append_running_descendants(
            active_session_id, current_chat, cached_chat_lookup, requesting, session_title,
            &child_session_id,
            depth.saturating_add(1),
            sessions,
            controls,
            visited,
            projected,
        );
    }

    for session in sessions {
        let belongs_to_parent =
            session.parent_id.as_deref().map(str::trim) == Some(parent_session_id);
        if !belongs_to_parent
            || !session.is_plan_run()
            || !requesting(&session.id)
            || !visited.insert(session.id.clone())
        {
            continue;
        }

        let is_planning = session.title.trim().starts_with(PLANNING_TITLE_PREFIX);
        let control = controls.get(&session.id);
        projected.push(AgentChildSession {
            session_id: session.id.clone(),
            agent: "子智能体".to_string(),
            task: if is_planning { String::new() } else { "工作流执行".to_string() },
            title: if is_planning {
                planning_row_label(&session.title, &session.id)
            } else {
                session_title(&session.id, &session.title)
            },
            directory: session.directory.clone(),
            kind: if is_planning {
                AgentChildSessionKind::WorkflowPlanning
            } else {
                AgentChildSessionKind::WorkflowExecution
            },
            status: control
                .map(|control| control.status)
                .unwrap_or(AgentChildSessionStatus::Running),
            depth,
            plan_run_task_id: control.map(|control| control.task_id.clone()),
            task_path: control.and_then(|control| control.task_path.clone()),
        });
        append_running_descendants(
            active_session_id, current_chat, cached_chat_lookup, requesting, session_title,
            &session.id,
            depth.saturating_add(1),
            sessions,
            controls,
            visited,
            projected,
        );
    }
}

/// 合并当前会话及其运行中后代，并按层级生成输入面板投影。
pub fn running_agent_sessions<'a>(
    active_session_id: Option<&str>, current_chat: &[ChatMessage], sessions: impl Iterator<Item = &'a Info> + Clone,
    cached_chat_lookup: &impl Fn(&str) -> Option<Arc<[ChatMessage]>>,
    requesting: &impl Fn(&str) -> bool, session_title: &impl Fn(&str, &str) -> String,
) -> Vec<AgentChildSession> {
    let Some(parent_session_id) =
        active_session_id.map(str::trim).filter(|id| !id.is_empty())
    else {
        return Vec::new();
    };
    // Most conversations have no running descendants. Do not build global ID maps
    // or scan other sessions' cached transcripts just to return an empty badge.
    let has_agent_children = !agent_child_session_projection(
        Some(parent_session_id),
        current_chat,
        std::iter::empty::<&Info>(),
    )
    .is_empty();
    let has_plan_children =
        sessions.clone().any(|session| {
            session.parent_id.as_deref().map(str::trim) == Some(parent_session_id)
                && session.is_plan_run()
                && requesting(&session.id)
        });
    if !has_agent_children && !has_plan_children {
        return Vec::new();
    }
    let sessions = known_sessions(sessions);
    let controls = plan_run_controls(current_chat, cached_chat_lookup, &sessions);
    let mut visited = HashSet::from([parent_session_id.to_string()]);
    let mut projected = Vec::new();
    append_running_descendants(
        active_session_id, current_chat, cached_chat_lookup, requesting, session_title,
        parent_session_id,
        0,
        &sessions,
        &controls,
        &mut visited,
        &mut projected,
    );
    projected
}

pub fn agent_sessions_badge_label(i18n: I18n, count: usize) -> String {
    desktop_i18n::__!(i18n, "%u 个工作中", count).expect("valid agent count template")
}

pub fn agent_sessions_badge_style(
    theme: &Theme,
    status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    let primary = theme.extended_palette().primary.base.color;
    let hovered = matches!(
        status,
        iced::widget::button::Status::Hovered | iced::widget::button::Status::Pressed
    );
    let background_alpha = if theme_is_dark(theme) {
        if hovered { 0.24 } else { 0.16 }
    } else if hovered {
        0.16
    } else {
        0.10
    };

    iced::widget::button::Style {
        background: Some(Background::Color(primary.scale_alpha(background_alpha))),
        text_color: if theme_is_dark(theme) {
            theme.palette().text.scale_alpha(0.94)
        } else {
            primary.scale_alpha(0.92)
        },
        border: Border {
            width: 1.0,
            color: primary.scale_alpha(if hovered { 0.42 } else { 0.28 }),
            radius: 999.0.into(),
        },
        ..Default::default()
    }
}

pub fn agent_sessions_popover_style(theme: &Theme) -> iced::widget::container::Style {
    let dark = theme_is_dark(theme);
    let background =
        if dark { Color::from_rgb8(30, 32, 38) } else { Color::from_rgb8(250, 251, 253) };
    let border = if dark { Color::from_rgb8(66, 70, 80) } else { Color::from_rgb8(218, 223, 231) };

    iced::widget::container::Style {
        background: Some(Background::Color(background)),
        border: Border { width: 1.0, color: border, radius: 12.0.into() },
        text_color: Some(theme.palette().text.scale_alpha(0.96)),
        shadow: iced::Shadow {
            color: Color::BLACK.scale_alpha(if dark { 0.36 } else { 0.14 }),
            offset: iced::Vector::new(0.0, 6.0),
            blur_radius: 18.0,
        },
        ..Default::default()
    }
}

pub fn agent_sessions_row_style(theme: &Theme) -> iced::widget::container::Style {
    let dark = theme_is_dark(theme);
    iced::widget::container::Style {
        background: Some(Background::Color(theme.palette().text.scale_alpha(if dark {
            0.055
        } else {
            0.035
        }))),
        text_color: Some(theme.palette().text.scale_alpha(0.94)),
        border: Border {
            width: 1.0,
            color: theme.palette().text.scale_alpha(if dark { 0.11 } else { 0.08 }),
            radius: 9.0.into(),
        },
        ..Default::default()
    }
}

pub fn agent_sessions_stop_style(
    theme: &Theme,
    status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    let danger = theme.extended_palette().danger.base.color;
    let hovered = matches!(
        status,
        iced::widget::button::Status::Hovered | iced::widget::button::Status::Pressed
    );
    iced::widget::button::Style {
        background: Some(Background::Color(danger.scale_alpha(if hovered { 0.20 } else { 0.10 }))),
        text_color: danger,
        border: Border {
            width: 1.0,
            color: danger.scale_alpha(if hovered { 0.42 } else { 0.24 }),
            radius: 7.0.into(),
        },
        ..Default::default()
    }
}

/// 仅为仍在运行的会话生成停止消息。
pub fn agent_session_stop_message(child: &AgentChildSession) -> Option<Action> {
    if !matches!(
        child.kind,
        AgentChildSessionKind::WorkflowExecution | AgentChildSessionKind::WorkflowPlanning
    ) {
        return None;
    }
    let task_id = child.plan_run_task_id.as_deref().map(str::trim).filter(|id| !id.is_empty())?;
    Some(Action::StopPlanRun(task_id.to_string()))
}

/// 返回会话可执行操作的辅助说明，无操作时返回 `None`。
pub fn agent_session_tooltip_text(child: &AgentChildSession) -> Option<&str> {
    child.task_path.as_deref().map(str::trim).filter(|path| !path.is_empty()).or_else(|| {
        let directory = child.directory.trim();
        (!directory.is_empty()).then_some(directory)
    })
}

pub fn agent_session_row<'a, Message: Clone + 'static>(i18n: I18n, frame: usize, child: &AgentChildSession, emit: fn(Action) -> Message) -> Element<'a, Message> {
    let running_color = Color::from_rgb8(96, 165, 250);
    let content: Element<'a, Message> = if child.kind == AgentChildSessionKind::WorkflowPlanning {
        text(
            desktop_i18n::__!(
                i18n,
                "规划 · %s · 计划任务",
                child.plan_run_task_id.as_deref().unwrap_or(&child.session_id)
            )
            .expect("valid planning title template"),
        )
        .size(12)
        .width(Length::Fill)
        .wrapping(iced::widget::text::Wrapping::None)
        .style(|theme: &Theme| iced::widget::text::Style {
            color: Some(theme.palette().text.scale_alpha(0.96)),
        })
        .into()
    } else {
        let title = compact_and_truncate(&child.title, TITLE_MAX_CHARS);
        let task = compact_and_truncate(
            if child.kind == AgentChildSessionKind::WorkflowExecution {
                i18n.__("工作流执行")
            } else {
                &child.task
            },
            TASK_MAX_CHARS,
        );
        let agent = compact_and_truncate(
            if child.kind == AgentChildSessionKind::WorkflowExecution {
                i18n.__("子智能体")
            } else {
                &child.agent
            },
            22,
        );
        let title_line = row![
            text(title).size(12).width(Length::Fill).style(|theme: &Theme| {
                iced::widget::text::Style { color: Some(theme.palette().text.scale_alpha(0.96)) }
            }),
            container(text(agent).size(9)).padding([2, 6]).style(move |_theme: &Theme| {
                iced::widget::container::Style {
                    background: Some(Background::Color(running_color.scale_alpha(0.14))),
                    border: Border { width: 0.0, color: Color::TRANSPARENT, radius: 999.0.into() },
                    text_color: Some(running_color),
                    ..Default::default()
                }
            }),
        ]
        .spacing(8)
        .align_y(Alignment::Center);
        let status_label = agent_session_status_label(child.kind, child.status);
        let running = row![
            text(spinner_frame(frame)).size(10).style(
                move |_theme: &Theme| iced::widget::text::Style { color: Some(running_color) }
            ),
            text(i18n.__(status_label)).size(10).style(
                move |_theme: &Theme| { iced::widget::text::Style { color: Some(running_color) } }
            ),
            text(task).size(10).width(Length::Fill).style(|theme: &Theme| {
                iced::widget::text::Style { color: Some(theme.palette().text.scale_alpha(0.62)) }
            }),
        ]
        .spacing(5)
        .align_y(Alignment::Center);
        column![title_line, running].spacing(4).width(Length::Fill).into()
    };
    let status_card =
        container(content).width(Length::Fill).padding([9, 10]).style(agent_sessions_row_style);
    let mut session_row =
        row![Space::new().width(Length::Fixed((child.depth as f32) * 14.0)), status_card]
            .spacing(6)
            .align_y(Alignment::Center)
            .width(Length::Fill);
    if let Some(stop_message) = agent_session_stop_message(child) {
        session_row = session_row.push(
            button(text(i18n.__("停止")).size(10))
                .padding([5, 7])
                .style(agent_sessions_stop_style)
                .on_press(emit(stop_message)),
        );
    }
    let session_row: Element<'a, Message> = session_row.into();
    match agent_session_tooltip_text(child) {
        None => session_row,
        Some(tooltip_text) => {
            let tooltip = container(
                text(tooltip_text.to_string())
                    .size(12)
                    .wrapping(iced::widget::text::Wrapping::WordOrGlyph),
            )
            .padding([6, 10])
            .max_width(320.0)
            .style(desktop_ui_controls::tooltip::tooltip_dark_style);
            Tooltip::new(session_row, tooltip, TooltipPosition::Top).gap(6.0).into()
        }
    }
}

/// 构建按父子层级排列的运行中代理会话弹层。
pub fn agent_sessions_popover<'a, Message: Clone + 'static>(
    i18n: I18n, frame: usize, children: &[AgentChildSession], emit: fn(Action) -> Message,
) -> Element<'a, Message> {
    let header = row![
        text(i18n.__("工作中的子代理会话"))
            .size(13)
            .style(|theme: &Theme| {
                iced::widget::text::Style { color: Some(theme.palette().text.scale_alpha(0.96)) }
            }),
        Space::new().width(Length::Fill),
        text(children.len().to_string()).size(11).style(|theme: &Theme| {
            iced::widget::text::Style {
                color: Some(theme.extended_palette().primary.base.color.scale_alpha(0.92)),
            }
        }),
    ]
    .align_y(Alignment::Center);

    let mut list = column![].spacing(6);
    for child in children {
        list = list.push(agent_session_row(i18n, frame, child, emit));
    }

    let scrolling = scrollable(container(list).padding(iced::Padding {
        top: 0.0,
        right: 5.0,
        bottom: 0.0,
        left: 0.0,
    }))
    .id(iced::widget::Id::new("input_panel_agent_sessions_scroll"))
    .direction(Direction::Vertical(
        Scrollbar::new().width(SCROLLBAR_WIDTH).scroller_width(SCROLLBAR_WIDTH),
    ))
    .height(Length::Fixed(POPOVER_MAX_HEIGHT));

    container(column![header, scrolling].spacing(10))
        .width(Length::Fixed(POPOVER_WIDTH))
        .padding(10)
        .style(agent_sessions_popover_style)
        .into()
}

/// 构建输入面板的会话计数入口；没有运行会话时不显示。
pub fn agent_sessions_entry<'a, Message: Clone + 'static>(
    i18n: I18n, frame: usize, show: bool, children: &[AgentChildSession], emit: fn(Action) -> Message,
) -> Option<Element<'a, Message>> {
    if children.is_empty() {
        return None;
    }

    let running_color = Color::from_rgb8(96, 165, 250);
    let badge = button(
        row![
            text(spinner_frame(frame)).size(11).style(
                move |_theme: &Theme| iced::widget::text::Style { color: Some(running_color) }
            ),
            text(agent_sessions_badge_label(
                i18n,
                children.len()
            ))
            .size(11),
        ]
        .spacing(6)
        .align_y(Alignment::Center),
    )
    .padding([5, 10])
    .style(agent_sessions_badge_style)
    .on_press(emit(Action::ToggleAgentSessionsPopover));

    let anchor = row![badge, Space::new().width(Length::Fill)]
        .align_y(Alignment::Center)
        .width(Length::Fill);
    let popover = if show {
        agent_sessions_popover(i18n, frame, children, emit)
    } else {
        Space::new().into()
    };

    Some(
        AboveOverlay::new(anchor, popover)
            .show(show)
            .gap(6.0)
            .on_close(emit(Action::CloseAgentSessionsPopover))
            .into(),
    )
}

#[cfg(test)]
#[path="agent_sessions_tests.rs"]
mod tests;
