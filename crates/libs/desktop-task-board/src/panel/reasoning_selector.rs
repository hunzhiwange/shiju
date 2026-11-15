#[derive(Clone, Copy)]
pub struct ReasoningView<'a> {
 pub draft: &'a desktop_task_model::TaskDraft,
 pub bulk_agent: Option<&'a str>,
 pub bulk_effort: Option<ReasoningEffortDto>,
 pub model_options: &'a [Option<ReasoningEffortDto>],
 pub desktop_locale: config_ui::DesktopLocale,
}
// 任务看板推理强度选择器。

use std::fmt;

use iced::{Element, Length};
use vw_api_chat_identity::ReasoningEffortDto;

use desktop_chat_input::input_panel::reasoning_selector::{
    reasoning_effort_label, reasoning_effort_options,
};
use desktop_ui_controls::system_settings_common::settings_select;
use crate::message::TaskBoardMessage;
use crate::message::TaskBoardMessage as Message;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TaskBoardReasoningEffortOption(
    Option<ReasoningEffortDto>,
    config_ui::DesktopLocale,
);

impl fmt::Display for TaskBoardReasoningEffortOption {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(desktop_i18n::I18n::new(self.1).__(reasoning_effort_label(self.0)))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TaskBoardReasoningSelectorKind {
    Draft,
    Edit,
    Bulk,
}

impl TaskBoardReasoningSelectorKind {
    fn current(self, app: ReasoningView<'_>) -> Option<ReasoningEffortDto> {
        match self {
            Self::Draft | Self::Edit => app.draft.reasoning_effort,
            Self::Bulk => app.bulk_effort,
        }
    }

    fn agent<'a>(self, app: ReasoningView<'a>) -> Option<&'a str> {
        match self {
            Self::Draft | Self::Edit => app.draft.acp_agent.as_deref(),
            Self::Bulk => app.bulk_agent,
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

pub fn build_reasoning_effort_selector<'a>(
    app: ReasoningView<'a>,
    is_edit_mode: bool,
) -> Option<Element<'static, Message>> {
    let kind = if is_edit_mode {
        TaskBoardReasoningSelectorKind::Edit
    } else {
        TaskBoardReasoningSelectorKind::Draft
    };
    build_selector(app, kind)
}

pub fn build_bulk_reasoning_effort_selector<'a>(app: ReasoningView<'a>) -> Option<Element<'static, Message>> {
    build_selector(app, TaskBoardReasoningSelectorKind::Bulk)
}

fn build_selector<'a>(app: ReasoningView<'a>, kind: TaskBoardReasoningSelectorKind) -> Option<Element<'static, Message>> {
    let options = kind
        .agent(app)
        .map(|agent| reasoning_effort_options(agent).to_vec())
        .unwrap_or_else(|| app.model_options.to_vec())
        .iter()
        .copied()
        .map(|value| TaskBoardReasoningEffortOption(value, app.desktop_locale))
        .collect::<Vec<_>>();
    if options.len() <= 1 {
        return None;
    }
    let selected = TaskBoardReasoningEffortOption(kind.current(app), app.desktop_locale);

    Some(
        settings_select(options, Some(selected), move |option| {
            kind.select_message(option.0)
        })
        .placeholder(desktop_i18n::I18n::new(app.desktop_locale).__("推理强度"))
        .width(Length::Fill)
        .into(),
    )
}

#[cfg(test)]
#[path = "reasoning_selector_tests.rs"]
mod reasoning_selector_tests;
