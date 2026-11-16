//! 工作流画布消息的测试，覆盖平移、缩放、拖拽和连接生命周期。
//! 用例重点验证交互快照、坐标换算及取消操作的优先级。
use serde_yaml::Value;

use crate::message::{self, WorkflowMessage};
use desktop_workflow_model::*;
use desktop_workflow_state::state::*;
use iced::{Point, Vector};

fn new_app() -> WorkflowState {
    WorkflowState::default()
}

fn node(id: &str, position: Point) -> desktop_workflow_model::WorkflowNode {
    desktop_workflow_model::WorkflowNode {
        id: id.to_string(),
        block_type: "llm".to_string(),
        title: id.to_string(),
        description: String::new(),
        position,
        size: iced::Size::new(120.0, 80.0),
        parent_id: None,
        selected: false,
        source_side: desktop_workflow_model::WorkflowHandleSide::Right,
        target_side: desktop_workflow_model::WorkflowHandleSide::Left,
        source_handles: vec![desktop_workflow_model::WorkflowHandle {
            id: "source".to_string(),
            label: "输出".to_string(),
            kind: desktop_workflow_model::WorkflowHandleKind::Source,
        }],
        target_handles: vec![desktop_workflow_model::WorkflowHandle {
            id: "target".to_string(),
            label: "输入".to_string(),
            kind: desktop_workflow_model::WorkflowHandleKind::Target,
        }],
        z_index: 0.0,
        raw_node: Value::Null,
    }
}

fn edge() -> desktop_workflow_model::WorkflowEdge {
    desktop_workflow_model::WorkflowEdge {
        id: "a-source--b-target".to_string(),
        source: "a".to_string(),
        target: "b".to_string(),
        source_handle: Some("source".to_string()),
        target_handle: Some("target".to_string()),
        source_type: "llm".to_string(),
        target_type: "llm".to_string(),
        selected: false,
        z_index: 0.0,
        raw_edge: Value::Null,
    }
}

fn endpoint(
    node_id: &str,
    kind: desktop_workflow_model::WorkflowHandleKind,
) -> WorkflowConnectionEndpoint {
    WorkflowConnectionEndpoint {
        node_id: node_id.to_string(),
        handle_id: match kind {
            desktop_workflow_model::WorkflowHandleKind::Source => "source",
            desktop_workflow_model::WorkflowHandleKind::Target => "target",
        }
        .to_string(),
        kind,
    }
}

#[test]
fn selection_pan_and_zoom_messages_update_canvas_state() {
    let mut app = new_app();
    app.document.nodes = vec![node("a", Point::new(10.0, 20.0))];
    app.zoom_menu_open = true;

    assert!(
        message::canvas::handle(
            &mut app,
            (1200.0, 800.0),
            WorkflowMessage::SelectNode("a".to_string())
        )
        .is_some()
    );
    assert_eq!(app.selected_node_id.as_deref(), Some("a"));

    message::canvas::handle(
        &mut app,
        (1200.0, 800.0),
        WorkflowMessage::SelectEdge("edge".to_string()),
    );
    assert_eq!(app.selected_edge_id.as_deref(), Some("edge"));
    assert!(app.selected_node_id.is_none());

    message::canvas::handle(&mut app, (1200.0, 800.0), WorkflowMessage::ClearSelection);
    assert!(app.selected_edge_id.is_none());

    message::canvas::handle(
        &mut app,
        (1200.0, 800.0),
        WorkflowMessage::PanBy(Vector::new(8.0, -3.0)),
    );
    assert_eq!(app.pan, Vector::new(8.0, -3.0));

    message::canvas::handle(
        &mut app,
        (1200.0, 800.0),
        WorkflowMessage::Zoom(2.0, Some(Point::new(100.0, 100.0))),
    );
    assert_eq!(app.zoom, 0.1);
    assert!(!app.zoom_menu_open);

    message::canvas::handle(&mut app, (1200.0, 800.0), WorkflowMessage::ZoomSet(0.01));
    assert_eq!(app.zoom, 0.1);

    message::canvas::handle(&mut app, (1200.0, 800.0), WorkflowMessage::ToggleZoomMenu);
    assert!(app.zoom_menu_open);
}

#[test]
fn drag_and_connection_messages_update_interaction_state() {
    let mut app = new_app();
    app.document.nodes =
        vec![node("a", Point::new(10.0, 20.0)), node("b", Point::new(260.0, 20.0))];
    let source = endpoint("a", desktop_workflow_model::WorkflowHandleKind::Source);
    let target = endpoint("b", desktop_workflow_model::WorkflowHandleKind::Target);

    message::canvas::handle(
        &mut app,
        (1200.0, 800.0),
        WorkflowMessage::NodeDragStart("a".to_string()),
    );
    assert_eq!(app.dragging_node_id.as_deref(), Some("a"));
    message::canvas::handle(
        &mut app,
        (1200.0, 800.0),
        WorkflowMessage::NodeDragged("a".to_string(), Vector::new(5.0, 6.0)),
    );
    assert_eq!(app.document.node("a").unwrap().position, Point::new(15.0, 26.0));
    message::canvas::handle(&mut app, (1200.0, 800.0), WorkflowMessage::FinishNodeDrag);
    assert!(app.dragging_node_id.is_none());

    message::canvas::handle(
        &mut app,
        (1200.0, 800.0),
        WorkflowMessage::StartConnection(source, Point::new(100.0, 100.0)),
    );
    assert!(app.connection_draft.is_some());
    message::canvas::handle(
        &mut app,
        (1200.0, 800.0),
        WorkflowMessage::UpdateConnectionCursor(Point::new(120.0, 130.0)),
    );
    assert_eq!(app.connection_draft.as_ref().unwrap().cursor_world, Point::new(120.0, 130.0));
    message::canvas::handle(&mut app, (1200.0, 800.0), WorkflowMessage::FinishConnection(target));
    assert_eq!(app.document.edges.len(), 1);

    let source = endpoint("a", desktop_workflow_model::WorkflowHandleKind::Source);
    message::canvas::handle(
        &mut app,
        (1200.0, 800.0),
        WorkflowMessage::StartConnection(source, Point::new(0.0, 0.0)),
    );
    message::canvas::handle(&mut app, (1200.0, 800.0), WorkflowMessage::CancelConnection);
    assert!(app.connection_draft.is_none());
}

#[test]
fn context_delete_undo_redo_and_error_messages_are_dispatched() {
    let mut app = new_app();
    app.document.nodes =
        vec![node("a", Point::new(10.0, 20.0)), node("b", Point::new(260.0, 20.0))];
    app.document.edges = vec![edge()];

    message::canvas::handle(
        &mut app,
        (1200.0, 800.0),
        WorkflowMessage::OpenCanvasContextMenu(
            WorkflowCanvasContextMenuTarget::Node("a".to_string()),
            Point::new(20.0, 30.0),
            Point::new(10.0, 15.0),
        ),
    );
    assert!(app.context_menu.is_some());
    assert_eq!(app.selected_node_id.as_deref(), Some("a"));

    message::canvas::handle(
        &mut app,
        (1200.0, 800.0),
        WorkflowMessage::CanvasContextMenuScrollCaptured,
    );
    assert!(app.context_menu.is_some());

    message::canvas::handle(&mut app, (1200.0, 800.0), WorkflowMessage::CloseCanvasContextMenu);
    assert!(app.context_menu.is_none());

    message::canvas::handle(
        &mut app,
        (1200.0, 800.0),
        WorkflowMessage::DeleteEdgeById("a-source--b-target".to_string()),
    );
    assert!(app.document.edges.is_empty());

    message::canvas::handle(
        &mut app,
        (1200.0, 800.0),
        WorkflowMessage::DeleteNodeById("b".to_string()),
    );
    assert!(app.document.node("b").is_none());

    app.set_error("boom".to_string());
    message::canvas::handle(&mut app, (1200.0, 800.0), WorkflowMessage::DismissError);
    assert!(app.error_message.is_none());

    assert!(message::canvas::handle(&mut app, (1200.0, 800.0), WorkflowMessage::Undo).is_some());
    assert!(message::canvas::handle(&mut app, (1200.0, 800.0), WorkflowMessage::Redo).is_some());
    assert!(
        message::canvas::handle(&mut app, (1200.0, 800.0), WorkflowMessage::LoadSavedApps)
            .is_none()
    );
}

#[test]
fn zoom_fit_and_cancel_interaction_messages_are_dispatched() {
    let mut app = new_app();
    app.pan = Vector::new(9.0, 9.0);
    app.zoom = 2.0;
    app.zoom_menu_open = true;

    assert!(message::canvas::handle(&mut app, (1200.0, 800.0), WorkflowMessage::ZoomFit).is_some());
    assert_eq!(app.pan, Vector::new(120.0, 120.0));
    assert_eq!(app.zoom, 1.0);

    app.zoom_menu_open = true;
    message::canvas::handle(&mut app, (1200.0, 800.0), WorkflowMessage::CancelInteraction);
    assert!(!app.zoom_menu_open);
    assert_eq!(app.status_message.as_deref(), Some("已关闭浮层菜单"));
}

#[test]
fn duplicate_and_selected_delete_messages_update_state_and_record_errors() {
    let mut app = new_app();
    app.document.nodes =
        vec![node("a", Point::new(10.0, 20.0)), node("b", Point::new(260.0, 20.0))];
    app.document.edges = vec![edge()];

    message::canvas::handle(&mut app, (1200.0, 800.0), WorkflowMessage::DuplicateSelectedNode);
    assert_eq!(app.error_message.as_deref(), Some("请先选择一个节点"));

    message::canvas::handle(
        &mut app,
        (1200.0, 800.0),
        WorkflowMessage::SelectNode("a".to_string()),
    );
    message::canvas::handle(&mut app, (1200.0, 800.0), WorkflowMessage::DuplicateSelectedNode);
    assert_eq!(app.document.nodes.len(), 3);
    assert_ne!(app.selected_node_id.as_deref(), Some("a"));
    assert_eq!(app.status_message.as_deref(), Some("已复制节点 a"));

    message::canvas::handle(
        &mut app,
        (1200.0, 800.0),
        WorkflowMessage::SelectEdge("a-source--b-target".to_string()),
    );
    message::canvas::handle(&mut app, (1200.0, 800.0), WorkflowMessage::DeleteSelectedEdge);
    assert!(app.document.edges.is_empty());
    assert!(app.selected_edge_id.is_none());

    message::canvas::handle(
        &mut app,
        (1200.0, 800.0),
        WorkflowMessage::SelectNode("b".to_string()),
    );
    message::canvas::handle(&mut app, (1200.0, 800.0), WorkflowMessage::DeleteSelectedNode);
    assert!(app.document.node("b").is_none());
    assert!(app.selected_node_id.is_none());
}

#[test]
fn jump_to_node_message_focuses_existing_node_and_reports_missing_node() {
    let mut app = new_app();
    app.document.nodes = vec![node("a", Point::new(40.0, 60.0))];

    assert!(
        message::canvas::handle(
            &mut app,
            (1200.0, 800.0),
            WorkflowMessage::JumpToNode("a".to_string())
        )
        .is_some()
    );
    assert_eq!(app.selected_node_id.as_deref(), Some("a"));
    assert_eq!(app.status_message.as_deref(), Some("已定位到节点 a"));

    message::canvas::handle(
        &mut app,
        (1200.0, 800.0),
        WorkflowMessage::JumpToNode("missing".to_string()),
    );
    assert_eq!(app.error_message.as_deref(), Some("目标节点不存在"));
}
