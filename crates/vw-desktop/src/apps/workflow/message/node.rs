//! Desktop mapping for workflow node updates.
use super::*;
pub(super) fn handle(app: &mut App, message: WorkflowMessage) -> Option<Task<Message>> {
    desktop_workflow_ui::message::node::handle(&mut app.workflow_state, app.window_size, message)
        .map(|task| task.map(Message::WorkflowTool))
}
