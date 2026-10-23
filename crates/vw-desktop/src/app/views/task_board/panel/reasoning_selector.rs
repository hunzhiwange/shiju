//! 任务看板推理强度选择器。

use std::fmt;

use iced::widget::pick_list;
use iced::{Element, Length};
use vw_gateway_client::vw_api_types::ReasoningEffortDto;

use crate::app::components::input_panel::reasoning_selector::{
    model_reasoning_effort_options, reasoning_effort_label, reasoning_effort_options,
};
use crate::app::components::system_settings_common::{
    settings_pick_list_menu_style, settings_pick_list_style,
};
use crate::app::message::TaskBoardMessage;
use crate::app::{App, Message};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TaskBoardReasoningEffortOption(Option<ReasoningEffortDto>);

impl fmt::Display for TaskBoardReasoningEffortOption {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(reasoning_effort_label(self.0))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TaskBoardReasoningSelectorKind {
    Draft,
    Edit,
    Bulk,
}

impl TaskBoardReasoningSelectorKind {
    fn current(self, app: &App) -> Option<ReasoningEffortDto> {
        match self {
            Self::Draft | Self::Edit => app.task_board_draft.reasoning_effort,
            Self::Bulk => app.task_board_bulk_reasoning_effort,
        }
    }

    fn agent<'a>(self, app: &'a App) -> Option<&'a str> {
        match self {
            Self::Draft | Self::Edit => app.task_board_draft.acp_agent.as_deref(),
            Self::Bulk => app.task_board_bulk_acp_agent.as_deref(),
        }
    }

    fn select_message(self, value: Option<ReasoningEffortDto>) -> TaskBoardMessage {
        match self {
            Self::Draft => TaskBoardMessage::UpdateDraftReasoningEffort(value),
            Self::Edit => TaskBoardMessage::UpdateEditingTaskReasoningEffort(value),
            Self::Bulk => TaskBoardMessage::UpdateBulkReasoningEffort(value),
        }
    }
}

pub(crate) fn build_reasoning_effort_selector(
    app: &App,
    is_edit_mode: bool,
) -> Option<Element<'_, Message>> {
    let kind = if is_edit_mode {
        TaskBoardReasoningSelectorKind::Edit
    } else {
        TaskBoardReasoningSelectorKind::Draft
    };
    build_selector(app, kind)
}

pub(crate) fn build_bulk_reasoning_effort_selector(app: &App) -> Option<Element<'_, Message>> {
    build_selector(app, TaskBoardReasoningSelectorKind::Bulk)
}

fn build_selector(app: &App, kind: TaskBoardReasoningSelectorKind) -> Option<Element<'_, Message>> {
    let model = match kind {
        TaskBoardReasoningSelectorKind::Draft | TaskBoardReasoningSelectorKind::Edit => {
            &app.task_board_draft.model
        }
        TaskBoardReasoningSelectorKind::Bulk => &app.task_board_bulk_model_input,
    };
    let options = kind
        .agent(app)
        .map(|agent| reasoning_effort_options(agent).to_vec())
        .unwrap_or_else(|| model_reasoning_effort_options(app, model))
        .iter()
        .copied()
        .map(TaskBoardReasoningEffortOption)
        .collect::<Vec<_>>();
    if options.len() <= 1 {
        return None;
    }
    let selected = TaskBoardReasoningEffortOption(kind.current(app));

    Some(
        pick_list(options, Some(selected), move |option| {
            Message::TaskBoard(kind.select_message(option.0))
        })
        .placeholder("推理强度")
        .padding([6, 10])
        .text_size(14)
        .style(settings_pick_list_style)
        .menu_style(settings_pick_list_menu_style)
        .width(Length::Fill)
        .into(),
    )
}

#[cfg(test)]
#[path = "reasoning_selector_tests.rs"]
mod reasoning_selector_tests;
