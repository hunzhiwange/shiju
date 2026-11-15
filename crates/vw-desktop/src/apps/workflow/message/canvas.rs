//! Desktop mapping for workflow canvas updates.
use super::*;
pub(super) fn handle(app: &mut App, message: WorkflowMessage) -> Option<Task<Message>> {
    desktop_workflow_ui::message::canvas::handle(&mut app.workflow_state, app.window_size, message)
        .map(|task| task.map(Message::WorkflowTool))
}
