//! 展示代理子会话与计划运行会话的状态弹层和控制入口。

use std::collections::{HashMap, HashSet};

use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::tooltip::{Position as TooltipPosition, Tooltip};
use iced::widget::{Space, button, column, container, row, scrollable, text};
use iced::{Alignment, Background, Border, Color, Element, Length, Theme};

use crate::app::components::overlays::AboveOverlay;
use crate::app::components::status_animation::spinner_frame;
use crate::app::session::{
    AgentChildSession, AgentChildSessionKind, AgentChildSessionStatus, PlanRunSessionControl,
    agent_child_session_projection, plan_run_session_controls_from_content,
};
use crate::app::{App, Message, message};
use vw_shared::session::info::Info;

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
pub(super) fn agent_session_status_label(
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

fn known_sessions(app: &App) -> Vec<&Info> {
    let mut seen = HashSet::new();
    app.sessions
        .iter()
        .chain(app.project_sessions.values().flatten())
        .filter(|session| seen.insert(session.id.clone()))
        .collect()
}

fn session_chat(app: &App, session_id: &str) -> Vec<crate::app::models::ChatMessage> {
    if app.active_session_id.as_deref() == Some(session_id) {
        app.chat.clone()
    } else {
        app.cached_chat_messages(session_id)
            .map(|messages| messages.iter().cloned().collect())
            .unwrap_or_default()
    }
}

fn plan_run_controls(app: &App, sessions: &[&Info]) -> HashMap<String, PlanRunSessionControl> {
    let mut controls = HashMap::new();
    for message in &app.chat {
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
        let Some(chat) = app.cached_chat_messages(&session.id) else {
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
    app: &App,
    parent_session_id: &str,
    depth: usize,
    sessions: &[&Info],
    controls: &HashMap<String, PlanRunSessionControl>,
    visited: &mut HashSet<String>,
    projected: &mut Vec<AgentChildSession>,
) {
    let chat = session_chat(app, parent_session_id);
    let agent_children =
        agent_child_session_projection(Some(parent_session_id), &chat, sessions.iter().copied());
    for mut child in agent_children {
        if !visited.insert(child.session_id.clone()) {
            continue;
        }
        child.depth = depth;
        let child_session_id = child.session_id.clone();
        projected.push(child);
        append_running_descendants(
            app,
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
            || !app.get_session_runtime(&session.id).is_requesting
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
                session.title.clone()
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
            app,
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
pub(crate) fn running_agent_sessions(app: &App) -> Vec<AgentChildSession> {
    let Some(parent_session_id) =
        app.active_session_id.as_deref().map(str::trim).filter(|id| !id.is_empty())
    else {
        return Vec::new();
    };
    let sessions = known_sessions(app);
    let controls = plan_run_controls(app, &sessions);
    let mut visited = HashSet::from([parent_session_id.to_string()]);
    let mut projected = Vec::new();
    append_running_descendants(
        app,
        parent_session_id,
        0,
        &sessions,
        &controls,
        &mut visited,
        &mut projected,
    );
    projected
}

pub(super) fn agent_sessions_badge_label(count: usize) -> String {
    format!("{count} 个工作中")
}

pub(super) fn agent_sessions_badge_style(
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

pub(super) fn agent_sessions_popover_style(theme: &Theme) -> iced::widget::container::Style {
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

pub(super) fn agent_sessions_row_style(
    theme: &Theme,
    status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    let dark = theme_is_dark(theme);
    let primary = theme.extended_palette().primary.base.color;
    let background = match status {
        iced::widget::button::Status::Hovered | iced::widget::button::Status::Pressed => {
            primary.scale_alpha(if dark { 0.18 } else { 0.10 })
        }
        iced::widget::button::Status::Disabled => {
            theme.palette().text.scale_alpha(if dark { 0.035 } else { 0.025 })
        }
        iced::widget::button::Status::Active => {
            theme.palette().text.scale_alpha(if dark { 0.055 } else { 0.035 })
        }
    };

    iced::widget::button::Style {
        background: Some(Background::Color(background)),
        text_color: theme.palette().text.scale_alpha(0.94),
        border: Border {
            width: 1.0,
            color: theme.palette().text.scale_alpha(if dark { 0.11 } else { 0.08 }),
            radius: 9.0.into(),
        },
        ..Default::default()
    }
}

pub(super) fn agent_sessions_stop_style(
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

/// 为可打开的子会话生成导航消息；缺少路径时不提供操作。
pub(super) fn agent_session_open_message(child: &AgentChildSession) -> Option<Message> {
    let project_path = child.directory.trim();
    let session_id = child.session_id.trim();
    if project_path.is_empty() || session_id.is_empty() {
        return None;
    }

    Some(Message::Chat(message::ChatMessage::OpenAgentSession {
        project_path: project_path.to_string(),
        session_id: session_id.to_string(),
    }))
}

/// 仅为仍在运行的会话生成停止消息。
pub(super) fn agent_session_stop_message(child: &AgentChildSession) -> Option<Message> {
    if !matches!(
        child.kind,
        AgentChildSessionKind::WorkflowExecution | AgentChildSessionKind::WorkflowPlanning
    ) {
        return None;
    }
    let task_id = child.plan_run_task_id.as_deref().map(str::trim).filter(|id| !id.is_empty())?;
    Some(Message::Chat(message::ChatMessage::StopPlanRun(task_id.to_string())))
}

/// 返回会话可执行操作的辅助说明，无操作时返回 `None`。
pub(super) fn agent_session_tooltip_text(child: &AgentChildSession) -> Option<&str> {
    child.task_path.as_deref().map(str::trim).filter(|path| !path.is_empty()).or_else(|| {
        let directory = child.directory.trim();
        (!directory.is_empty()).then_some(directory)
    })
}

fn agent_session_row<'a>(app: &App, child: &AgentChildSession) -> Element<'a, Message> {
    let running_color = Color::from_rgb8(96, 165, 250);
    let content: Element<'a, Message> = if child.kind == AgentChildSessionKind::WorkflowPlanning {
        text(child.title.clone())
            .size(12)
            .width(Length::Fill)
            .wrapping(iced::widget::text::Wrapping::None)
            .style(|theme: &Theme| iced::widget::text::Style {
                color: Some(theme.palette().text.scale_alpha(0.96)),
            })
            .into()
    } else {
        let title = compact_and_truncate(&child.title, TITLE_MAX_CHARS);
        let task = compact_and_truncate(&child.task, TASK_MAX_CHARS);
        let agent = compact_and_truncate(&child.agent, 22);
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
            text(spinner_frame(app.status_animation_frame)).size(10).style(
                move |_theme: &Theme| iced::widget::text::Style { color: Some(running_color) }
            ),
            text(status_label).size(10).style(move |_theme: &Theme| {
                iced::widget::text::Style { color: Some(running_color) }
            }),
            text(task).size(10).width(Length::Fill).style(|theme: &Theme| {
                iced::widget::text::Style { color: Some(theme.palette().text.scale_alpha(0.62)) }
            }),
        ]
        .spacing(5)
        .align_y(Alignment::Center);
        column![title_line, running].spacing(4).width(Length::Fill).into()
    };
    let row_button =
        button(content).width(Length::Fill).padding([9, 10]).style(agent_sessions_row_style);

    let row_button: Element<'a, Message> = match agent_session_open_message(child) {
        Some(message) => row_button.on_press(message).into(),
        None => row_button.into(),
    };
    let mut session_row =
        row![Space::new().width(Length::Fixed((child.depth as f32) * 14.0)), row_button,]
            .spacing(6)
            .align_y(Alignment::Center)
            .width(Length::Fill);
    if let Some(stop_message) = agent_session_stop_message(child) {
        session_row = session_row.push(
            button(text("停止").size(10))
                .padding([5, 7])
                .style(agent_sessions_stop_style)
                .on_press(stop_message),
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
            .style(super::styles::tooltip_dark_style);
            Tooltip::new(session_row, tooltip, TooltipPosition::Top).gap(6.0).into()
        }
    }
}

/// 构建按父子层级排列的运行中代理会话弹层。
pub(super) fn agent_sessions_popover<'a>(
    app: &App,
    children: &[AgentChildSession],
) -> Element<'a, Message> {
    let header = row![
        text("工作中的子代理会话").size(13).style(|theme: &Theme| {
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
        list = list.push(agent_session_row(app, child));
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
pub(super) fn agent_sessions_entry<'a>(
    app: &App,
    children: &[AgentChildSession],
) -> Option<Element<'a, Message>> {
    if children.is_empty() {
        return None;
    }

    let running_color = Color::from_rgb8(96, 165, 250);
    let badge = button(
        row![
            text(spinner_frame(app.status_animation_frame)).size(11).style(
                move |_theme: &Theme| iced::widget::text::Style { color: Some(running_color) }
            ),
            text(agent_sessions_badge_label(children.len())).size(11),
        ]
        .spacing(6)
        .align_y(Alignment::Center),
    )
    .padding([5, 10])
    .style(agent_sessions_badge_style)
    .on_press(Message::Chat(message::ChatMessage::ToggleAgentSessionsPopover));

    let anchor = row![badge, Space::new().width(Length::Fill)]
        .align_y(Alignment::Center)
        .width(Length::Fill);
    let popover = agent_sessions_popover(app, children);

    Some(
        AboveOverlay::new(anchor, popover)
            .show(app.show_agent_sessions_popover)
            .gap(6.0)
            .on_close(Message::Chat(message::ChatMessage::CloseAgentSessionsPopover))
            .into(),
    )
}
