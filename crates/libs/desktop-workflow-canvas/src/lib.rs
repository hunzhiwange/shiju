//! Workflow canvas rendering, interaction, and SVG export.
mod canvas;
pub use canvas::{WorkflowCanvas, WorkflowCanvasState, export_svg};

use desktop_workflow_model::WorkflowConnectionEndpoint;
use desktop_workflow_state::app_ui::WorkflowCanvasContextMenuTarget;
use iced::{Point, Vector};

#[derive(Debug, Clone)]
pub enum CanvasMessage {
    StartConnection(WorkflowConnectionEndpoint, Point),
    NodeDragStart(String),
    SelectEdge(String),
    ClearSelection,
    OpenCanvasContextMenu(WorkflowCanvasContextMenuTarget, Point, Point),
    FinishConnection(WorkflowConnectionEndpoint),
    CancelConnection,
    FinishNodeDrag,
    PanBy(Vector),
    NodeDragged(String, Vector),
    UpdateConnectionCursor(Point),
}
