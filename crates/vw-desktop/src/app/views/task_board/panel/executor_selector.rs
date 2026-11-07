//! 任务看板侧栏面板模块，负责任务编辑、执行器选择和日志弹窗界面。
//!
//! 本模块只负责视图组合与样式适配，不持有业务状态，也不扩大外部能力边界。

use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::{Space, button, column, container, row, scrollable, svg};
use iced::{Alignment, Element, Length, Theme};

use crate::app::assets::{self, Icon};
use crate::app::components::input_panel::icons::acp_agent_icon;
use crate::app::components::overlays::BelowOverlay;
use crate::app::components::system_settings_common::SettingsSelectLabel;
use crate::app::message::TaskBoardMessage;
use crate::app::{App, Message};

use super::styles::{popover_style, selector_option_style, selector_trigger_style};

const ACP_SELECTOR_MAX_HEIGHT: f32 = 240.0;
const ACP_SELECTOR_SCROLLBAR_WIDTH: f32 = 4.0;
const ACP_SELECTOR_LIST_GUTTER: f32 = 7.0;
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
    let executor_label_text = current_executor.as_deref().map(str::to_owned).unwrap_or_else(|| {
        crate::app::i18n::I18n::new(app.desktop_locale).__(DEFAULT_TASK_EXECUTOR_LABEL).to_string()
    });

    let open = kind.is_open(app);
    let executor_toggle = button(
        row![
            acp_agent_icon(
                current_executor.as_deref().unwrap_or(DEFAULT_TASK_EXECUTOR_LABEL),
                12.0
            ),
            SettingsSelectLabel::new(executor_label_text),
            svg::Svg::<iced::Theme>::new(assets::get_icon(Icon::ChevronDown))
                .width(Length::Fixed(12.0))
                .height(Length::Fixed(12.0))
                .style(|theme: &Theme, _| svg::Style {
                    color: Some(theme.palette().text.scale_alpha(0.65))
                })
        ]
        .spacing(6)
        .align_y(Alignment::Center),
    )
    .style(move |theme, status| selector_trigger_style(theme, status, open))
    .padding([6, 8])
    .width(Length::Fill)
    .height(38)
    .on_press(Message::TaskBoard(kind.toggle_message()));

    let executor_pop_content = build_executor_popover(current_executor, app, kind);

    BelowOverlay::new(executor_toggle, executor_pop_content)
        .show(open)
        .gap(6.0)
        .on_close(Message::TaskBoard(kind.close_message()))
        .into()
}

fn build_executor_popover<'a>(
    current_executor: Option<String>,
    app: &'a App,
    kind: TaskBoardExecutorSelectorKind,
) -> Element<'a, Message> {
    let default_label =
        crate::app::i18n::I18n::new(app.desktop_locale).__(DEFAULT_TASK_EXECUTOR_LABEL);
    let entries = std::iter::once((DEFAULT_TASK_EXECUTOR_LABEL, default_label, None)).chain(
        app.acp_agents.iter().map(|agent| (agent.as_str(), agent.as_str(), Some(agent.clone()))),
    );
    let executor_list = column(entries.map(|(icon, label, value)| {
        let selected = current_executor == value;
        let mark: Element<'_, Message> = if selected {
            svg::Svg::<iced::Theme>::new(assets::get_icon(Icon::Check))
                .width(14)
                .height(14)
                .style(|theme: &Theme, _| svg::Style { color: Some(theme.palette().primary) })
                .into()
        } else {
            Space::new().width(14).height(14).into()
        };
        button(
            row![acp_agent_icon(icon, 14.0), SettingsSelectLabel::new(label.to_owned()), mark]
                .spacing(8)
                .align_y(Alignment::Center),
        )
        .padding([6, 10])
        .width(Length::Fill)
        .height(36)
        .style(move |theme, status| selector_option_style(theme, status, selected))
        .on_press(Message::TaskBoard(kind.select_message(value)))
        .into()
    }))
    .spacing(2);

    container(
        scrollable(container(executor_list).padding([0.0, ACP_SELECTOR_LIST_GUTTER]))
            .id(iced::widget::Id::new("task_board_executor_selector_scroll"))
            .direction(Direction::Vertical(
                Scrollbar::new()
                    .width(ACP_SELECTOR_SCROLLBAR_WIDTH)
                    .scroller_width(ACP_SELECTOR_SCROLLBAR_WIDTH),
            ))
            .height(Length::Shrink),
    )
    .style(popover_style)
    .padding([6, 0])
    .max_height(ACP_SELECTOR_MAX_HEIGHT)
    .width(Length::Fixed(216.0))
    .into()
}

#[cfg(test)]
#[path = "executor_selector_tests.rs"]
mod executor_selector_tests;

#[cfg(test)]
#[path = "executor_selector_ui_refresh_35_tests.rs"]
mod executor_selector_ui_refresh_35_tests;
