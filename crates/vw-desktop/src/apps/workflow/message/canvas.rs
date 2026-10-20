//! 工作流画布消息处理，负责节点选择、拖拽和视口交互状态更新。

use super::*;

/// 构建或更新 handle 相关行为。
///
/// 参数由调用方提供，返回值直接交给上层视图或状态处理；本函数不执行额外错误恢复。
pub(super) fn handle(app: &mut App, message: WorkflowMessage) -> Option<Task<Message>> {
    Some(match message {
        WorkflowMessage::SelectNode(id) => {
            app.workflow_state.select_node(id);
            Task::none()
        }
        WorkflowMessage::SelectEdge(id) => {
            app.workflow_state.select_edge(id);
            Task::none()
        }
        WorkflowMessage::ClearSelection => {
            app.workflow_state.clear_selection();
            Task::none()
        }
        WorkflowMessage::PanBy(delta) => {
            app.workflow_state.pan_by(delta);
            Task::none()
        }
        WorkflowMessage::Zoom(factor, center_opt) => {
            app.workflow_state.zoom_by(factor, center_opt, app.window_size);
            Task::none()
        }
        WorkflowMessage::ZoomSet(zoom) => {
            app.workflow_state.zoom_set(zoom, app.window_size);
            Task::none()
        }
        WorkflowMessage::ZoomFit => {
            app.workflow_state.zoom_to_fit(app.window_size);
            Task::none()
        }
        WorkflowMessage::ToggleZoomMenu => {
            app.workflow_state.toggle_zoom_menu();
            Task::none()
        }
        WorkflowMessage::NodeDragStart(id) => {
            app.workflow_state.start_node_drag(&id);
            Task::none()
        }
        WorkflowMessage::NodeDragged(id, delta) => {
            app.workflow_state.move_node(&id, delta);
            Task::none()
        }
        WorkflowMessage::FinishNodeDrag => {
            app.workflow_state.finish_node_drag();
            Task::none()
        }
        WorkflowMessage::StartConnection(endpoint, cursor_world) => {
            app.workflow_state.start_connection(endpoint, cursor_world);
            Task::none()
        }
        WorkflowMessage::UpdateConnectionCursor(cursor_world) => {
            app.workflow_state.update_connection_cursor(cursor_world);
            Task::none()
        }
        WorkflowMessage::FinishConnection(endpoint) => {
            app.workflow_state.finish_connection(endpoint);
            Task::none()
        }
        WorkflowMessage::CancelConnection => {
            app.workflow_state.cancel_connection();
            Task::none()
        }
        WorkflowMessage::CancelInteraction => {
            app.workflow_state.cancel_interaction();
            Task::none()
        }
        WorkflowMessage::OpenCanvasContextMenu(target, anchor, world) => {
            app.workflow_state.open_context_menu(target, anchor, world);
            Task::none()
        }
        WorkflowMessage::CanvasContextMenuScrollCaptured => Task::none(),
        WorkflowMessage::CloseCanvasContextMenu => {
            app.workflow_state.close_context_menu();
            Task::none()
        }
        WorkflowMessage::DuplicateSelectedNode => {
            if let Err(error) = app.workflow_state.duplicate_selected_node() {
                app.workflow_state.set_error(error);
            }
            Task::none()
        }
        WorkflowMessage::DeleteSelectedNode => {
            app.workflow_state.delete_selected_node();
            Task::none()
        }
        WorkflowMessage::DeleteSelectedEdge => {
            app.workflow_state.delete_selected_edge();
            Task::none()
        }
        WorkflowMessage::DeleteNodeById(node_id) => {
            app.workflow_state.delete_node_by_id(&node_id);
            Task::none()
        }
        WorkflowMessage::DeleteEdgeById(edge_id) => {
            app.workflow_state.delete_edge_by_id(&edge_id);
            Task::none()
        }
        WorkflowMessage::JumpToNode(node_id) => {
            if let Err(error) = app.workflow_state.focus_node(&node_id, app.window_size) {
                app.workflow_state.set_error(error);
            }
            Task::none()
        }
        WorkflowMessage::Undo => {
            app.workflow_state.undo();
            Task::none()
        }
        WorkflowMessage::Redo => {
            app.workflow_state.redo();
            Task::none()
        }
        WorkflowMessage::DismissError => {
            app.workflow_state.clear_error();
            Task::none()
        }
        _ => return None,
    })
}
