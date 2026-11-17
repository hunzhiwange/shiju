//! Desktop canvas event projection.
pub use desktop_workflow_canvas::{WorkflowCanvas, export_svg};
#[cfg(test)]
pub(super) fn map_message(message: desktop_workflow_canvas::CanvasMessage) -> crate::app::Message {
    crate::app::Message::WorkflowTool(desktop_workflow_ui::canvas_message::map_message(message))
}
