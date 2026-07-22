use serde_yaml::Value;

use super::*;

fn new_app() -> App {
    App::new().0
}

fn node(id: &str, position: Point) -> super::super::model::WorkflowNode {
    super::super::model::WorkflowNode {
        id: id.to_string(),
        block_type: "llm".to_string(),
        title: id.to_string(),
        description: String::new(),
        position,
        size: iced::Size::new(120.0, 80.0),
        parent_id: None,
        selected: false,
        source_side: super::super::model::WorkflowHandleSide::Right,
        target_side: super::super::model::WorkflowHandleSide::Left,
        source_handles: vec![super::super::model::WorkflowHandle {
            id: "source".to_string(),
            label: "输出".to_string(),
            kind: super::super::model::WorkflowHandleKind::Source,
        }],
        target_handles: vec![super::super::model::WorkflowHandle {
            id: "target".to_string(),
            label: "输入".to_string(),
            kind: super::super::model::WorkflowHandleKind::Target,
        }],
        z_index: 0.0,
        raw_node: Value::Null,
    }
}

fn edge() -> super::super::model::WorkflowEdge {
    super::super::model::WorkflowEdge {
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
    kind: super::super::model::WorkflowHandleKind,
) -> WorkflowConnectionEndpoint {
    WorkflowConnectionEndpoint {
        node_id: node_id.to_string(),
        handle_id: match kind {
            super::super::model::WorkflowHandleKind::Source => "source",
            super::super::model::WorkflowHandleKind::Target => "target",
        }
        .to_string(),
        kind,
    }
}

#[test]
fn selection_pan_and_zoom_messages_update_canvas_state() {
    let mut app = new_app();
    app.workflow_state.document.nodes = vec![node("a", Point::new(10.0, 20.0))];
    app.workflow_state.zoom_menu_open = true;

    assert!(canvas::handle(&mut app, WorkflowMessage::SelectNode("a".to_string())).is_some());
    assert_eq!(app.workflow_state.selected_node_id.as_deref(), Some("a"));

    canvas::handle(&mut app, WorkflowMessage::SelectEdge("edge".to_string()));
    assert_eq!(app.workflow_state.selected_edge_id.as_deref(), Some("edge"));
    assert!(app.workflow_state.selected_node_id.is_none());

    canvas::handle(&mut app, WorkflowMessage::ClearSelection);
    assert!(app.workflow_state.selected_edge_id.is_none());

    canvas::handle(&mut app, WorkflowMessage::PanBy(Vector::new(8.0, -3.0)));
    assert_eq!(app.workflow_state.pan, Vector::new(8.0, -3.0));

    canvas::handle(&mut app, WorkflowMessage::Zoom(2.0, Some(Point::new(100.0, 100.0))));
    assert_eq!(app.workflow_state.zoom, 0.1);
    assert!(!app.workflow_state.zoom_menu_open);

    canvas::handle(&mut app, WorkflowMessage::ZoomSet(0.01));
    assert_eq!(app.workflow_state.zoom, 0.1);

    canvas::handle(&mut app, WorkflowMessage::ToggleZoomMenu);
    assert!(app.workflow_state.zoom_menu_open);
}

#[test]
fn drag_and_connection_messages_update_interaction_state() {
    let mut app = new_app();
    app.workflow_state.document.nodes =
        vec![node("a", Point::new(10.0, 20.0)), node("b", Point::new(260.0, 20.0))];
    let source = endpoint("a", super::super::model::WorkflowHandleKind::Source);
    let target = endpoint("b", super::super::model::WorkflowHandleKind::Target);

    canvas::handle(&mut app, WorkflowMessage::NodeDragStart("a".to_string()));
    assert_eq!(app.workflow_state.dragging_node_id.as_deref(), Some("a"));
    canvas::handle(&mut app, WorkflowMessage::NodeDragged("a".to_string(), Vector::new(5.0, 6.0)));
    assert_eq!(app.workflow_state.document.node("a").unwrap().position, Point::new(15.0, 26.0));
    canvas::handle(&mut app, WorkflowMessage::FinishNodeDrag);
    assert!(app.workflow_state.dragging_node_id.is_none());

    canvas::handle(&mut app, WorkflowMessage::StartConnection(source, Point::new(100.0, 100.0)));
    assert!(app.workflow_state.connection_draft.is_some());
    canvas::handle(&mut app, WorkflowMessage::UpdateConnectionCursor(Point::new(120.0, 130.0)));
    assert_eq!(
        app.workflow_state.connection_draft.as_ref().unwrap().cursor_world,
        Point::new(120.0, 130.0)
    );
    canvas::handle(&mut app, WorkflowMessage::FinishConnection(target));
    assert_eq!(app.workflow_state.document.edges.len(), 1);

    let source = endpoint("a", super::super::model::WorkflowHandleKind::Source);
    canvas::handle(&mut app, WorkflowMessage::StartConnection(source, Point::new(0.0, 0.0)));
    canvas::handle(&mut app, WorkflowMessage::CancelConnection);
    assert!(app.workflow_state.connection_draft.is_none());
}

#[test]
fn context_delete_undo_redo_and_error_messages_are_dispatched() {
    let mut app = new_app();
    app.workflow_state.document.nodes =
        vec![node("a", Point::new(10.0, 20.0)), node("b", Point::new(260.0, 20.0))];
    app.workflow_state.document.edges = vec![edge()];

    canvas::handle(
        &mut app,
        WorkflowMessage::OpenCanvasContextMenu(
            WorkflowCanvasContextMenuTarget::Node("a".to_string()),
            Point::new(20.0, 30.0),
            Point::new(10.0, 15.0),
        ),
    );
    assert!(app.workflow_state.context_menu.is_some());
    assert_eq!(app.workflow_state.selected_node_id.as_deref(), Some("a"));

    canvas::handle(&mut app, WorkflowMessage::CloseCanvasContextMenu);
    assert!(app.workflow_state.context_menu.is_none());

    canvas::handle(&mut app, WorkflowMessage::DeleteEdgeById("a-source--b-target".to_string()));
    assert!(app.workflow_state.document.edges.is_empty());

    canvas::handle(&mut app, WorkflowMessage::DeleteNodeById("b".to_string()));
    assert!(app.workflow_state.document.node("b").is_none());

    app.workflow_state.set_error("boom".to_string());
    canvas::handle(&mut app, WorkflowMessage::DismissError);
    assert!(app.workflow_state.error_message.is_none());

    assert!(canvas::handle(&mut app, WorkflowMessage::Undo).is_some());
    assert!(canvas::handle(&mut app, WorkflowMessage::Redo).is_some());
    assert!(canvas::handle(&mut app, WorkflowMessage::LoadSavedApps).is_none());
}

#[test]
fn zoom_fit_and_cancel_interaction_messages_are_dispatched() {
    let mut app = new_app();
    app.workflow_state.pan = Vector::new(9.0, 9.0);
    app.workflow_state.zoom = 2.0;
    app.workflow_state.zoom_menu_open = true;

    assert!(canvas::handle(&mut app, WorkflowMessage::ZoomFit).is_some());
    assert_eq!(app.workflow_state.pan, Vector::new(120.0, 120.0));
    assert_eq!(app.workflow_state.zoom, 1.0);

    app.workflow_state.zoom_menu_open = true;
    canvas::handle(&mut app, WorkflowMessage::CancelInteraction);
    assert!(!app.workflow_state.zoom_menu_open);
    assert_eq!(app.workflow_state.status_message.as_deref(), Some("已关闭浮层菜单"));
}

#[test]
fn duplicate_and_selected_delete_messages_update_state_and_record_errors() {
    let mut app = new_app();
    app.workflow_state.document.nodes =
        vec![node("a", Point::new(10.0, 20.0)), node("b", Point::new(260.0, 20.0))];
    app.workflow_state.document.edges = vec![edge()];

    canvas::handle(&mut app, WorkflowMessage::DuplicateSelectedNode);
    assert_eq!(app.workflow_state.error_message.as_deref(), Some("请先选择一个节点"));

    canvas::handle(&mut app, WorkflowMessage::SelectNode("a".to_string()));
    canvas::handle(&mut app, WorkflowMessage::DuplicateSelectedNode);
    assert_eq!(app.workflow_state.document.nodes.len(), 3);
    assert_ne!(app.workflow_state.selected_node_id.as_deref(), Some("a"));
    assert_eq!(app.workflow_state.status_message.as_deref(), Some("已复制节点 a"));

    canvas::handle(&mut app, WorkflowMessage::SelectEdge("a-source--b-target".to_string()));
    canvas::handle(&mut app, WorkflowMessage::DeleteSelectedEdge);
    assert!(app.workflow_state.document.edges.is_empty());
    assert!(app.workflow_state.selected_edge_id.is_none());

    canvas::handle(&mut app, WorkflowMessage::SelectNode("b".to_string()));
    canvas::handle(&mut app, WorkflowMessage::DeleteSelectedNode);
    assert!(app.workflow_state.document.node("b").is_none());
    assert!(app.workflow_state.selected_node_id.is_none());
}

#[test]
fn jump_to_node_message_focuses_existing_node_and_reports_missing_node() {
    let mut app = new_app();
    app.workflow_state.document.nodes = vec![node("a", Point::new(40.0, 60.0))];

    assert!(canvas::handle(&mut app, WorkflowMessage::JumpToNode("a".to_string())).is_some());
    assert_eq!(app.workflow_state.selected_node_id.as_deref(), Some("a"));
    assert_eq!(app.workflow_state.status_message.as_deref(), Some("已定位到节点 a"));

    canvas::handle(&mut app, WorkflowMessage::JumpToNode("missing".to_string()));
    assert_eq!(app.workflow_state.error_message.as_deref(), Some("目标节点不存在"));
}
