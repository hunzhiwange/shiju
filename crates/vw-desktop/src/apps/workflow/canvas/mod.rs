//! Desktop mapping for canvas interaction messages.
pub use desktop_workflow_canvas::{WorkflowCanvas, export_svg};
use desktop_workflow_canvas::CanvasMessage;
use super::message::WorkflowMessage;
use crate::app::Message;

pub(super) fn map_message(message: CanvasMessage) -> Message {
    let message = match message {
        CanvasMessage::StartConnection(endpoint, point) => {
            WorkflowMessage::StartConnection(endpoint, point)
        }
        CanvasMessage::NodeDragStart(id) => WorkflowMessage::NodeDragStart(id),
        CanvasMessage::SelectEdge(id) => WorkflowMessage::SelectEdge(id),
        CanvasMessage::ClearSelection => WorkflowMessage::ClearSelection,
        CanvasMessage::OpenCanvasContextMenu(target, anchor, world) => {
            WorkflowMessage::OpenCanvasContextMenu(target, anchor, world)
        }
        CanvasMessage::FinishConnection(endpoint) => WorkflowMessage::FinishConnection(endpoint),
        CanvasMessage::CancelConnection => WorkflowMessage::CancelConnection,
        CanvasMessage::FinishNodeDrag => WorkflowMessage::FinishNodeDrag,
        CanvasMessage::PanBy(delta) => WorkflowMessage::PanBy(delta),
        CanvasMessage::NodeDragged(id, delta) => WorkflowMessage::NodeDragged(id, delta),
        CanvasMessage::UpdateConnectionCursor(point) => WorkflowMessage::UpdateConnectionCursor(point),
    };
    Message::WorkflowTool(message)
}
