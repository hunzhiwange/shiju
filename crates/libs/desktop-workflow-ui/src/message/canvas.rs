//! 工作流画布消息处理，负责节点选择、拖拽和视口交互状态更新。

use super::*;
use desktop_workflow_state::state::WorkflowState;
use iced::Task;

/// 构建或更新 handle 相关行为。
///
/// 参数由调用方提供，返回值直接交给上层视图或状态处理；本函数不执行额外错误恢复。
pub fn handle(
    state: &mut WorkflowState,
    window_size: (f32, f32),
    message: WorkflowMessage,
) -> Option<Task<WorkflowMessage>> {
    Some(match message {
        WorkflowMessage::SelectNode(id) => {
            state.select_node(id);
            Task::none()
        }
        WorkflowMessage::SelectEdge(id) => {
            state.select_edge(id);
            Task::none()
        }
        WorkflowMessage::ClearSelection => {
            state.clear_selection();
            Task::none()
        }
        WorkflowMessage::PanBy(delta) => {
            state.pan_by(delta);
            Task::none()
        }
        WorkflowMessage::Zoom(factor, center_opt) => {
            state.zoom_by(factor, center_opt, window_size);
            Task::none()
        }
        WorkflowMessage::ZoomSet(zoom) => {
            state.zoom_set(zoom, window_size);
            Task::none()
        }
        WorkflowMessage::ZoomFit => {
            state.zoom_to_fit(window_size);
            Task::none()
        }
        WorkflowMessage::ToggleZoomMenu => {
            state.toggle_zoom_menu();
            Task::none()
        }
        WorkflowMessage::NodeDragStart(id) => {
            state.start_node_drag(&id);
            Task::none()
        }
        WorkflowMessage::NodeDragged(id, delta) => {
            state.move_node(&id, delta);
            Task::none()
        }
        WorkflowMessage::FinishNodeDrag => {
            state.finish_node_drag();
            Task::none()
        }
        WorkflowMessage::StartConnection(endpoint, cursor_world) => {
            state.start_connection(endpoint, cursor_world);
            Task::none()
        }
        WorkflowMessage::UpdateConnectionCursor(cursor_world) => {
            state.update_connection_cursor(cursor_world);
            Task::none()
        }
        WorkflowMessage::FinishConnection(endpoint) => {
            state.finish_connection(endpoint);
            Task::none()
        }
        WorkflowMessage::CancelConnection => {
            state.cancel_connection();
            Task::none()
        }
        WorkflowMessage::CancelInteraction => {
            state.cancel_interaction();
            Task::none()
        }
        WorkflowMessage::OpenCanvasContextMenu(target, anchor, world) => {
            state.open_context_menu(target, anchor, world);
            Task::none()
        }
        WorkflowMessage::CanvasContextMenuScrollCaptured => Task::none(),
        WorkflowMessage::CloseCanvasContextMenu => {
            state.close_context_menu();
            Task::none()
        }
        WorkflowMessage::DuplicateSelectedNode => {
            if let Err(error) = state.duplicate_selected_node() {
                state.set_error(error);
            }
            Task::none()
        }
        WorkflowMessage::DeleteSelectedNode => {
            state.delete_selected_node();
            Task::none()
        }
        WorkflowMessage::DeleteSelectedEdge => {
            state.delete_selected_edge();
            Task::none()
        }
        WorkflowMessage::DeleteNodeById(node_id) => {
            state.delete_node_by_id(&node_id);
            Task::none()
        }
        WorkflowMessage::DeleteEdgeById(edge_id) => {
            state.delete_edge_by_id(&edge_id);
            Task::none()
        }
        WorkflowMessage::JumpToNode(node_id) => {
            if let Err(error) = state.focus_node(&node_id, window_size) {
                state.set_error(error);
            }
            Task::none()
        }
        WorkflowMessage::Undo => {
            state.undo();
            Task::none()
        }
        WorkflowMessage::Redo => {
            state.redo();
            Task::none()
        }
        WorkflowMessage::DismissError => {
            state.clear_error();
            Task::none()
        }
        _ => return None,
    })
}
