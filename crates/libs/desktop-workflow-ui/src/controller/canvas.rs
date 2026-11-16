//! Desktop mapping for workflow canvas updates.
use super::*;
pub(crate) fn handle(app: &mut Context<'_>, message: WorkflowMessage) -> Option<Task<Output>> {
    crate::message::canvas::handle(&mut app.workflow_state, app.window_size, message)
        .map(|task| task.map(Output::Workflow))
}
