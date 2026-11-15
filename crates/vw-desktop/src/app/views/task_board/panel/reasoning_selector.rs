use crate::app::{App, Message};
fn build(app: &App, edit: bool, bulk: bool) -> Option<iced::Element<'static, Message>> {
 use desktop_task_board::panel::reasoning_selector as domain;
 let model = if bulk { &app.task_board_bulk_model_input } else { &app.task_board_draft.model };
 let agent = if bulk { app.task_board_bulk_acp_agent.as_deref() } else { app.task_board_draft.acp_agent.as_deref() };
 let options = if agent.is_none() { crate::app::components::input_panel::reasoning_selector::model_reasoning_effort_options(app, model) } else { Vec::new() };
 let input = domain::ReasoningView { draft: &app.task_board_draft, bulk_agent: app.task_board_bulk_acp_agent.as_deref(), bulk_effort: app.task_board_bulk_reasoning_effort, model_options: &options, desktop_locale: app.desktop_locale };
 (if bulk { domain::build_bulk_reasoning_effort_selector(input) } else { domain::build_reasoning_effort_selector(input, edit) }).map(|view| view.map(Message::TaskBoard))
}
pub(crate) fn build_reasoning_effort_selector(app: &App, edit: bool) -> Option<iced::Element<'_, Message>> { build(app, edit, false) }
pub(crate) fn build_bulk_reasoning_effort_selector(app: &App) -> Option<iced::Element<'_, Message>> { build(app, false, true) }

#[cfg(test)]
#[path="reasoning_selector_tests.rs"]
mod reasoning_selector_tests;
