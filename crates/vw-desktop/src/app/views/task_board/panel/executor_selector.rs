//! 任务看板侧栏面板模块，负责任务编辑、执行器选择和日志弹窗界面。
//!
//! 本模块只负责视图组合与样式适配，不持有业务状态，也不扩大外部能力边界。

use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::{Space, button, column, container, row, scrollable, svg, text};
use iced::{Alignment, Background, Border, Color, Element, Length, Theme};

use crate::app::assets::{self, Icon};
use crate::app::components::input_panel::icons::acp_agent_icon;
use crate::app::components::overlays::BelowOverlay;
use crate::app::message::TaskBoardMessage;
use crate::app::{App, Message};

use super::styles::{pill_button_style, popover_style};

const ACP_SELECTOR_MAX_HEIGHT: f32 = 240.0;
const ACP_SELECTOR_SCROLLBAR_WIDTH: f32 = 4.0;
const ACP_SELECTOR_LIST_RIGHT_PADDING: f32 = 5.0;
const DEFAULT_TASK_EXECUTOR_LABEL: &str = "未使用 ACP";

#[derive(Clone, Copy)]
enum TaskBoardExecutorSelectorKind {
    Draft,
    Edit,
    Bulk,
}

impl TaskBoardExecutorSelectorKind {
    fn current_executor(&self, app: &App) -> Option<String> {
        match self {
            Self::Draft | Self::Edit => app.task_board_draft.acp_agent.clone(),
            Self::Bulk => app.task_board_bulk_acp_agent.clone(),
        }
    }

    fn toggle_message(&self) -> TaskBoardMessage {
        match self {
            Self::Draft | Self::Edit => TaskBoardMessage::ToggleExecutorPopover,
            Self::Bulk => TaskBoardMessage::ToggleBulkExecutorPopover,
        }
    }

    fn close_message(&self) -> TaskBoardMessage {
        match self {
            Self::Draft | Self::Edit => TaskBoardMessage::ToggleExecutorPopover,
            Self::Bulk => TaskBoardMessage::CloseBulkExecutorPopover,
        }
    }

    fn is_open(&self, app: &App) -> bool {
        match self {
            Self::Draft | Self::Edit => app.task_board_executor_popover,
            Self::Bulk => app.task_board_bulk_executor_popover,
        }
    }

    fn select_message(&self, executor: Option<String>) -> TaskBoardMessage {
        match self {
            Self::Draft => TaskBoardMessage::UpdateDraftExecutor(executor),
            Self::Edit => TaskBoardMessage::UpdateEditingTaskExecutor(executor),
            Self::Bulk => TaskBoardMessage::BulkExecutorSelected(executor),
        }
    }
}

fn executor_option_button_style(
    theme: &Theme,
    status: iced::widget::button::Status,
    selected: bool,
) -> iced::widget::button::Style {
    let hovered = matches!(status, iced::widget::button::Status::Hovered);
    let p = theme.extended_palette();

    let bg = if hovered {
        Some(Background::Color(p.background.weak.color.scale_alpha(0.35)))
    } else if selected {
        Some(Background::Color(Color::from_rgba(
            theme.palette().primary.r,
            theme.palette().primary.g,
            theme.palette().primary.b,
            0.10,
        )))
    } else {
        None
    };

    iced::widget::button::Style {
        background: bg,
        border: Border { radius: 6.0.into(), width: 0.0, color: theme.palette().primary },
        text_color: theme.palette().text,
        ..Default::default()
    }
}

/// 构建对应界面片段。
///
/// # 参数
/// - `app`: 当前视图构建所需的状态、配置或消息。
/// - `is_edit_mode`: 当前视图构建所需的状态、配置或消息。
///
/// # 返回
/// 返回可直接嵌入父级视图的 Iced 控件或样式闭包。
///
/// # 错误
/// 此函数不返回 `Result`；不可用状态会通过空视图、禁用控件或回退文案表达。
pub fn build_executor_selector<'a>(app: &'a App, is_edit_mode: bool) -> Element<'a, Message> {
    let kind = if is_edit_mode {
        TaskBoardExecutorSelectorKind::Edit
    } else {
        TaskBoardExecutorSelectorKind::Draft
    };
    build_selector(app, kind)
}

/// 构建对应界面片段。
///
/// # 参数
/// - `app`: 当前视图构建所需的状态、配置或消息。
///
/// # 返回
/// 返回可直接嵌入父级视图的 Iced 控件或样式闭包。
///
/// # 错误
/// 此函数不返回 `Result`；不可用状态会通过空视图、禁用控件或回退文案表达。
pub fn build_bulk_executor_selector<'a>(app: &'a App) -> Element<'a, Message> {
    build_selector(app, TaskBoardExecutorSelectorKind::Bulk)
}

fn build_selector<'a>(app: &'a App, kind: TaskBoardExecutorSelectorKind) -> Element<'a, Message> {
    let current_executor = kind.current_executor(app);
    let executor_label_text =
        current_executor.as_deref().unwrap_or(DEFAULT_TASK_EXECUTOR_LABEL).to_string();

    let executor_toggle = button(
        row![
            acp_agent_icon(
                current_executor.as_deref().unwrap_or(DEFAULT_TASK_EXECUTOR_LABEL),
                14.0
            ),
            text(executor_label_text).size(14),
            svg::Svg::<iced::Theme>::new(assets::get_icon(Icon::ChevronDown))
                .width(Length::Fixed(14.0))
                .height(Length::Fixed(14.0))
                .style(|theme: &Theme, _| svg::Style {
                    color: Some(theme.palette().text.scale_alpha(0.65))
                })
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    )
    .style(pill_button_style)
    .padding([6, 10])
    .on_press(Message::TaskBoard(kind.toggle_message()));

    let executor_pop_content = build_executor_popover(current_executor, app, kind);

    BelowOverlay::new(executor_toggle, executor_pop_content)
        .show(kind.is_open(app))
        .gap(6.0)
        .on_close(Message::TaskBoard(kind.close_message()))
        .into()
}

fn build_executor_popover<'a>(
    current_executor: Option<String>,
    app: &'a App,
    kind: TaskBoardExecutorSelectorKind,
) -> Element<'a, Message> {
    let mut executor_list = column![].spacing(4);

    let default_selected = current_executor.is_none();
    let default_check: Element<'_, Message> = if default_selected {
        svg::Svg::<iced::Theme>::new(assets::get_icon(Icon::Check))
            .width(Length::Fixed(14.0))
            .height(Length::Fixed(14.0))
            .style(|theme: &Theme, _| svg::Style { color: Some(theme.palette().primary) })
            .into()
    } else {
        Space::new().width(Length::Fixed(14.0)).into()
    };

    let default_btn = button(
        row![
            acp_agent_icon(DEFAULT_TASK_EXECUTOR_LABEL, 14.0),
            text(DEFAULT_TASK_EXECUTOR_LABEL).size(13),
            Space::new().width(Length::Fill),
            default_check
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    )
    .padding([6, 10])
    .width(Length::Fill)
    .style(move |theme: &Theme, status: iced::widget::button::Status| {
        executor_option_button_style(theme, status, default_selected)
    })
    .on_press(Message::TaskBoard(kind.select_message(None)));

    executor_list = executor_list.push(default_btn);

    for backend in app.acp_agents.iter().cloned() {
        let selected = current_executor.as_ref() == Some(&backend);

        let check: Element<'_, Message> = if selected {
            svg::Svg::<iced::Theme>::new(assets::get_icon(Icon::Check))
                .width(Length::Fixed(14.0))
                .height(Length::Fixed(14.0))
                .style(|theme: &Theme, _| svg::Style { color: Some(theme.palette().primary) })
                .into()
        } else {
            Space::new().width(Length::Fixed(14.0)).into()
        };

        let select_btn = button(
            row![
                acp_agent_icon(&backend, 14.0),
                text(backend.clone()).size(13),
                Space::new().width(Length::Fill),
                check
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        )
        .padding([6, 10])
        .width(Length::Fill)
        .style(move |theme: &Theme, status: iced::widget::button::Status| {
            executor_option_button_style(theme, status, selected)
        })
        .on_press(Message::TaskBoard(kind.select_message(Some(backend))));

        executor_list = executor_list.push(select_btn);
    }

    container(
        scrollable(container(executor_list).padding(iced::Padding {
            top: 0.0,
            right: ACP_SELECTOR_LIST_RIGHT_PADDING,
            bottom: 0.0,
            left: 0.0,
        }))
        .id(iced::widget::Id::new("task_board_executor_selector_scroll"))
        .direction(Direction::Vertical(
            Scrollbar::new()
                .width(ACP_SELECTOR_SCROLLBAR_WIDTH)
                .scroller_width(ACP_SELECTOR_SCROLLBAR_WIDTH),
        ))
        .height(Length::Fixed(ACP_SELECTOR_MAX_HEIGHT)),
    )
    .style(popover_style)
    .padding(8)
    .width(Length::Fixed(180.0))
    .into()
}

#[cfg(test)]
#[path = "executor_selector_tests.rs"]
mod executor_selector_tests;
