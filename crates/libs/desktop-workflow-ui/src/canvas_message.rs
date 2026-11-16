use crate::message::WorkflowMessage;
use desktop_workflow_canvas::CanvasMessage;
pub fn map_message(message: CanvasMessage) -> WorkflowMessage {
    match message {
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
        CanvasMessage::UpdateConnectionCursor(point) => {
            WorkflowMessage::UpdateConnectionCursor(point)
        }
    }
}
