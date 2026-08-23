//! 工作流画布状态操作的测试，覆盖节点拖拽、连接和交互取消。
//! 快照复用与层级断言确保连续指针事件只产生一次语义修改。
#[test]
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("canvas_ops_tests"));
}

use super::*;
use crate::apps::workflow::model::{WorkflowHandle, WorkflowHandleSide};
use iced::Size;

#[test]
fn pan_and_zoom_operations_update_viewport_close_menus_and_dirty_state() {
    let mut state = state_with_document(WorkflowDocument {
        nodes: vec![test_node("start", "start", Point::new(0.0, 0.0))],
        ..WorkflowDocument::default()
    });
    state.context_menu = Some(context_menu(WorkflowCanvasContextMenuTarget::Canvas));
    state.zoom_menu_open = true;

    state.pan_by(Vector::new(10.0, -5.0));

    assert_eq!(state.pan, Vector::new(10.0, -5.0));
    assert!(state.context_menu.is_none());
    assert!(state.active_is_dirty);

    state.pan = Vector::new(0.0, 0.0);
    state.zoom = 1.0;
    state.context_menu = Some(context_menu(WorkflowCanvasContextMenuTarget::Canvas));
    state.zoom_menu_open = true;
    state.zoom_by(2.0, Some(Point::new(100.0, 100.0)), (800.0, 600.0));
    assert_eq!(state.zoom, 2.0);
    assert_eq!(state.pan, Vector::new(-100.0, -100.0));
    assert!(state.context_menu.is_none());
    assert!(!state.zoom_menu_open);

    state.zoom_set(0.05, (800.0, 600.0));
    assert_eq!(state.zoom, 0.1);
    assert!((state.pan.x - 375.0).abs() < 0.001);
    assert!((state.pan.y - 280.0).abs() < 0.001);

    state.zoom_to_fit((1280.0, 860.0));
    assert!((0.1..=4.0).contains(&state.zoom));
    assert!(state.context_menu.is_none());
}

#[test]
fn zoom_to_fit_uses_default_viewport_for_empty_document() {
    let mut state = state_with_document(WorkflowDocument::default());
    state.pan = Vector::new(9.0, 9.0);
    state.zoom = 2.0;

    state.zoom_to_fit((800.0, 600.0));

    assert_eq!(state.pan, Vector::new(120.0, 120.0));
    assert_eq!(state.zoom, 1.0);
}

#[test]
fn zoom_by_clamps_both_limits_and_uses_window_center_by_default() {
    let mut state = state_with_document(WorkflowDocument::default());
    state.pan = Vector::new(10.0, -20.0);
    state.zoom = 2.0;

    state.zoom_by(0.0, None, (800.0, 600.0));

    assert_eq!(state.zoom, 0.1);
    assert!((state.pan.x - 380.5).abs() < 0.001);
    assert!((state.pan.y - 284.0).abs() < 0.001);

    state.zoom_by(100.0, None, (800.0, 600.0));

    assert_eq!(state.zoom, 4.0);
    assert!((state.pan.x + 380.0).abs() < 0.001);
    assert!((state.pan.y + 340.0).abs() < 0.001);
}

#[test]
fn dragging_and_moving_node_moves_descendants_and_commits_one_undo_snapshot() {
    let document = WorkflowDocument {
        nodes: vec![
            test_node("parent", "loop", Point::new(10.0, 20.0)),
            WorkflowNode {
                parent_id: Some("parent".to_string()),
                ..test_node("child", "answer", Point::new(30.0, 40.0))
            },
        ],
        ..WorkflowDocument::default()
    };
    let mut state = state_with_document(document);

    state.start_node_drag("parent");
    assert_eq!(state.dragging_node_id.as_deref(), Some("parent"));
    assert!(state.drag_pending_snapshot.is_some());
    assert_eq!(state.selected_node_id.as_deref(), Some("parent"));

    state.move_node("parent", Vector::new(5.0, -10.0));
    assert_eq!(state.document.node("parent").unwrap().position, Point::new(15.0, 10.0));
    assert_eq!(state.document.node("child").unwrap().position, Point::new(35.0, 30.0));
    assert_eq!(state.undo_stack.len(), 1);
    assert!(state.drag_pending_snapshot.is_none());
    assert_eq!(state.status_message.as_deref(), Some("已更新节点布局"));
    assert!(state.active_is_dirty);

    state.move_node("parent", Vector::new(0.0, 0.0));
    assert_eq!(state.undo_stack.len(), 1);
    state.finish_node_drag();
    assert!(state.dragging_node_id.is_none());
    assert!(state.drag_pending_snapshot.is_none());
}

#[test]
fn moving_missing_node_closes_context_without_dirtying_document() {
    let mut state = state_with_document(WorkflowDocument::default());
    state.context_menu = Some(context_menu(WorkflowCanvasContextMenuTarget::Canvas));

    state.move_node("missing", Vector::new(4.0, 5.0));

    assert!(state.context_menu.is_none());
    assert_eq!(state.status_message.as_deref(), Some("已更新节点布局"));
    assert!(!state.active_is_dirty);
}

#[test]
fn node_drag_reuses_snapshot_until_a_different_node_starts_dragging() {
    let document = WorkflowDocument {
        nodes: vec![
            test_node("first", "answer", Point::new(0.0, 0.0)),
            test_node("second", "answer", Point::new(200.0, 0.0)),
        ],
        ..WorkflowDocument::default()
    };
    let mut state = state_with_document(document);

    state.start_node_drag("first");
    let first_snapshot = state.drag_pending_snapshot.clone().expect("first drag snapshot");
    state.start_node_drag("first");

    assert_eq!(state.drag_pending_snapshot.as_ref(), Some(&first_snapshot));
    assert!(state.undo_stack.is_empty());

    state.start_node_drag("second");

    assert_eq!(state.dragging_node_id.as_deref(), Some("second"));
    assert_eq!(state.selected_node_id.as_deref(), Some("second"));
    assert_eq!(
        state
            .drag_pending_snapshot
            .as_ref()
            .and_then(|snapshot| snapshot.selected_node_id.as_deref()),
        Some("second")
    );
}

#[test]
fn connection_lifecycle_creates_updates_deduplicates_and_reports_invalid_paths() {
    let document = WorkflowDocument {
        nodes: vec![
            test_node("start", "start", Point::new(0.0, 0.0)),
            test_node("answer", "answer", Point::new(300.0, 0.0)),
        ],
        ..WorkflowDocument::default()
    };
    let mut state = state_with_document(document);

    state.start_connection(
        endpoint("start", "source", WorkflowHandleKind::Source),
        Point::new(1.0, 1.0),
    );
    assert_eq!(state.selected_node_id.as_deref(), Some("start"));
    assert_eq!(state.status_message.as_deref(), Some("拖到目标句柄以创建连线"));
    state.update_connection_cursor(Point::new(9.0, 10.0));
    assert_eq!(state.connection_draft.as_ref().unwrap().cursor_world, Point::new(9.0, 10.0));
    state.finish_connection(endpoint("answer", "target", WorkflowHandleKind::Target));

    assert_eq!(state.document.edges.len(), 1);
    assert_eq!(state.selected_edge_id.as_deref(), Some(state.document.edges[0].id.as_str()));
    assert!(state.selected_node_id.is_none());
    assert_eq!(state.document.edges[0].source, "start");
    assert_eq!(state.document.edges[0].target, "answer");
    assert_eq!(state.document.edges[0].source_type, "start");
    assert_eq!(state.document.edges[0].target_type, "answer");
    assert!(state.document.edges[0].selected);
    assert_eq!(state.status_message.as_deref(), Some("已连接 start -> answer"));

    state.start_connection(
        endpoint("start", "source", WorkflowHandleKind::Source),
        Point::new(0.0, 0.0),
    );
    state.finish_connection(endpoint("answer", "target", WorkflowHandleKind::Target));
    assert_eq!(state.document.edges.len(), 1);
    assert_eq!(state.status_message.as_deref(), Some("这条连线已经存在"));

    state.start_connection(
        endpoint("start", "source", WorkflowHandleKind::Source),
        Point::new(0.0, 0.0),
    );
    state.finish_connection(endpoint("answer", "source", WorkflowHandleKind::Source));
    assert_eq!(state.status_message.as_deref(), Some("连线需要从输出句柄连接到输入句柄"));

    state.start_connection(
        endpoint("answer", "source", WorkflowHandleKind::Source),
        Point::new(0.0, 0.0),
    );
    state.finish_connection(endpoint("answer", "target", WorkflowHandleKind::Target));
    assert_eq!(state.status_message.as_deref(), Some("暂不支持节点自身回环连线"));

    state.start_connection(
        endpoint("missing", "source", WorkflowHandleKind::Source),
        Point::new(0.0, 0.0),
    );
    state.finish_connection(endpoint("answer", "target", WorkflowHandleKind::Target));
    assert_eq!(state.status_message.as_deref(), Some("源节点不存在，无法创建连线"));

    state.start_connection(
        endpoint("start", "source", WorkflowHandleKind::Source),
        Point::new(0.0, 0.0),
    );
    state.finish_connection(endpoint("missing", "target", WorkflowHandleKind::Target));
    assert_eq!(state.status_message.as_deref(), Some("目标节点不存在，无法创建连线"));

    state.finish_connection(endpoint("answer", "target", WorkflowHandleKind::Target));
    assert!(state.connection_draft.is_none());
}

#[test]
fn connection_cursor_and_cancellation_are_noops_without_a_draft() {
    let mut state = state_with_document(WorkflowDocument::default());
    state.status_message = Some("unchanged".to_string());

    state.update_connection_cursor(Point::new(12.0, 34.0));
    state.cancel_connection();

    assert!(state.connection_draft.is_none());
    assert_eq!(state.status_message.as_deref(), Some("unchanged"));
}

#[test]
fn finish_connection_accepts_reverse_endpoint_order_and_increments_highest_z_index() {
    let document = WorkflowDocument {
        nodes: vec![
            test_node("start", "start", Point::new(0.0, 0.0)),
            test_node("answer", "answer", Point::new(300.0, 0.0)),
            test_node("other", "answer", Point::new(600.0, 0.0)),
        ],
        edges: vec![WorkflowEdge { z_index: 7.5, ..test_edge("start", "other", "existing") }],
        ..WorkflowDocument::default()
    };
    let mut state = state_with_document(document);

    state.start_connection(
        endpoint("answer", "target", WorkflowHandleKind::Target),
        Point::new(0.0, 0.0),
    );
    state.finish_connection(endpoint("start", "source", WorkflowHandleKind::Source));

    assert_eq!(state.document.edges.len(), 2);
    let created = state.document.edges.last().expect("created edge");
    assert_eq!(created.source, "start");
    assert_eq!(created.target, "answer");
    assert_eq!(created.z_index, 8.5);
    assert_eq!(state.status_message.as_deref(), Some("已连接 start -> answer"));
}

#[test]
fn cancel_connection_and_cancel_interaction_follow_priority_order() {
    let mut state = state_with_document(WorkflowDocument {
        nodes: vec![test_node("start", "start", Point::new(0.0, 0.0))],
        ..WorkflowDocument::default()
    });

    state.connection_draft = Some(WorkflowConnectionDraft {
        from: endpoint("start", "source", WorkflowHandleKind::Source),
        cursor_world: Point::new(0.0, 0.0),
    });
    state.cancel_connection();
    assert!(state.connection_draft.is_none());
    assert_eq!(state.status_message.as_deref(), Some("已取消连线"));

    state.action_menu_open = true;
    state.zoom_menu_open = true;
    state.quick_insert_panel_open = true;
    state.cancel_interaction();
    assert_eq!(state.status_message.as_deref(), Some("已关闭浮层菜单"));
    assert!(!state.action_menu_open);
    assert!(!state.zoom_menu_open);
    assert!(!state.quick_insert_panel_open);

    state.context_menu = Some(context_menu(WorkflowCanvasContextMenuTarget::Canvas));
    state.cancel_interaction();
    assert_eq!(state.status_message.as_deref(), Some("已关闭右键菜单"));
    assert!(state.context_menu.is_none());

    state.connection_draft = Some(WorkflowConnectionDraft {
        from: endpoint("start", "source", WorkflowHandleKind::Source),
        cursor_world: Point::new(0.0, 0.0),
    });
    state.cancel_interaction();
    assert_eq!(state.status_message.as_deref(), Some("已取消连线"));

    state.variable_editor = Some(WorkflowVariableEditorDraft {
        mode: WorkflowVariableEditorMode::CreateEnvironment,
        name: "env".to_string(),
        description: String::new(),
        value_type: "string".to_string(),
        raw_value_editor: text_editor::Content::with_text("value"),
    });
    state.cancel_interaction();
    assert_eq!(state.status_message.as_deref(), Some("已关闭变量编辑器"));
    assert!(state.variable_editor.is_none());

    state.variable_panel = Some(WorkflowVariablePanelKind::System);
    state.cancel_interaction();
    assert_eq!(state.status_message.as_deref(), Some("已关闭变量面板"));
    assert!(state.variable_panel.is_none());

    state.selected_node_id = Some("start".to_string());
    state.sync_selection_flags();
    state.cancel_interaction();
    assert_eq!(state.status_message.as_deref(), Some("已清除选择"));
    assert!(state.selected_node_id.is_none());
}

#[test]
fn cancel_interaction_is_noop_when_no_interaction_is_active() {
    let mut state = state_with_document(WorkflowDocument::default());
    state.status_message = Some("unchanged".to_string());

    state.cancel_interaction();

    assert_eq!(state.status_message.as_deref(), Some("unchanged"));
}

#[test]
fn delete_selected_edge_handles_absent_selection_and_removes_context_target() {
    let document = WorkflowDocument {
        nodes: vec![
            test_node("start", "start", Point::new(0.0, 0.0)),
            test_node("answer", "answer", Point::new(300.0, 0.0)),
        ],
        edges: vec![test_edge("start", "answer", "edge")],
        ..WorkflowDocument::default()
    };
    let mut state = state_with_document(document);

    assert!(!state.delete_selected_edge());
    state.selected_edge_id = Some("edge".to_string());
    state.context_menu =
        Some(context_menu(WorkflowCanvasContextMenuTarget::Edge("edge".to_string())));
    state.sync_selection_flags();

    assert!(state.delete_selected_edge());
    assert!(state.document.edges.is_empty());
    assert!(state.selected_edge_id.is_none());
    assert!(state.context_menu.is_none());
    assert_eq!(state.status_message.as_deref(), Some("已删除连线 start -> answer"));
    assert!(state.active_is_dirty);
}

#[test]
fn delete_selected_edge_keeps_missing_target_selection_and_document() {
    let document = WorkflowDocument {
        nodes: vec![
            test_node("start", "start", Point::new(0.0, 0.0)),
            test_node("answer", "answer", Point::new(300.0, 0.0)),
        ],
        edges: vec![test_edge("start", "answer", "existing")],
        ..WorkflowDocument::default()
    };
    let mut state = state_with_document(document);
    state.selected_edge_id = Some("missing".to_string());
    state.context_menu =
        Some(context_menu(WorkflowCanvasContextMenuTarget::Edge("missing".to_string())));

    assert!(!state.delete_selected_edge());

    assert_eq!(state.document.edges.len(), 1);
    assert_eq!(state.selected_edge_id.as_deref(), Some("missing"));
    assert!(state.context_menu.is_some());
    assert_eq!(state.undo_stack.len(), 1);
    assert!(state.status_message.is_none());
}

#[test]
fn delete_selected_node_removes_descendants_edges_editors_drafts_and_context() {
    let document = WorkflowDocument {
        nodes: vec![
            test_node("parent", "loop", Point::new(0.0, 0.0)),
            WorkflowNode {
                parent_id: Some("parent".to_string()),
                ..test_node("child", "answer", Point::new(20.0, 20.0))
            },
            test_node("outside", "answer", Point::new(400.0, 0.0)),
        ],
        edges: vec![test_edge("child", "outside", "edge")],
        ..WorkflowDocument::default()
    };
    let mut state = state_with_document(document);

    assert!(!state.delete_selected_node());
    state.selected_node_id = Some("missing".to_string());
    assert!(!state.delete_selected_node());
    assert!(state.selected_node_id.is_none());

    state.selected_node_id = Some("parent".to_string());
    state.connection_draft = Some(WorkflowConnectionDraft {
        from: endpoint("child", "source", WorkflowHandleKind::Source),
        cursor_world: Point::new(0.0, 0.0),
    });
    state.node_editor = Some(node_editor_editing("child"));
    state.context_menu =
        Some(context_menu(WorkflowCanvasContextMenuTarget::Node("child".to_string())));
    state.sync_selection_flags();

    assert!(state.delete_selected_node());

    assert!(state.document.node("parent").is_none());
    assert!(state.document.node("child").is_none());
    assert!(state.document.node("outside").is_some());
    assert!(state.document.edges.is_empty());
    assert!(state.selected_node_id.is_none());
    assert!(state.selected_edge_id.is_none());
    assert!(state.connection_draft.is_none());
    assert!(state.node_editor.is_none());
    assert!(state.context_menu.is_none());
    assert_eq!(
        state.status_message.as_deref(),
        Some("已删除节点 parent，包含 1 个子节点，并移除 1 条连线")
    );
}

#[test]
fn delete_selected_node_keeps_unrelated_interactions_and_clears_node_insert_context() {
    let document = WorkflowDocument {
        nodes: vec![
            test_node("victim", "answer", Point::new(0.0, 0.0)),
            test_node("outside", "answer", Point::new(300.0, 0.0)),
        ],
        ..WorkflowDocument::default()
    };
    let mut state = state_with_document(document);
    state.selected_node_id = Some("victim".to_string());
    state.connection_draft = Some(WorkflowConnectionDraft {
        from: endpoint("outside", "source", WorkflowHandleKind::Source),
        cursor_world: Point::new(0.0, 0.0),
    });
    let mut editor = node_editor_editing("victim");
    editor.mode = WorkflowNodeEditorMode::Create;
    state.node_editor = Some(editor);
    state.context_menu =
        Some(context_menu(WorkflowCanvasContextMenuTarget::NodeInsert("victim".to_string())));

    assert!(state.delete_selected_node());

    assert!(state.document.node("victim").is_none());
    assert!(state.document.node("outside").is_some());
    assert!(state.connection_draft.is_some());
    assert!(
        state
            .node_editor
            .as_ref()
            .is_some_and(|editor| matches!(&editor.mode, WorkflowNodeEditorMode::Create))
    );
    assert!(state.context_menu.is_none());
    assert_eq!(state.status_message.as_deref(), Some("已删除节点 victim"));
}

#[test]
fn delete_selected_node_removes_edges_targeting_the_removed_node() {
    let document = WorkflowDocument {
        nodes: vec![
            test_node("outside", "start", Point::new(0.0, 0.0)),
            test_node("victim", "answer", Point::new(300.0, 0.0)),
        ],
        edges: vec![test_edge("outside", "victim", "incoming")],
        ..WorkflowDocument::default()
    };
    let mut state = state_with_document(document);
    state.selected_node_id = Some("victim".to_string());

    assert!(state.delete_selected_node());

    assert!(state.document.edges.is_empty());
    assert_eq!(state.status_message.as_deref(), Some("已删除节点 victim，并移除 1 条连线"));
}

#[test]
fn undo_and_redo_restore_history_snapshots() {
    let document = WorkflowDocument {
        nodes: vec![test_node("start", "start", Point::new(0.0, 0.0))],
        ..WorkflowDocument::default()
    };
    let mut state = state_with_document(document);

    assert!(!state.undo());
    assert!(!state.redo());

    state.start_node_drag("start");
    state.move_node("start", Vector::new(50.0, 0.0));
    state.finish_node_drag();
    assert_eq!(state.document.node("start").unwrap().position, Point::new(50.0, 0.0));
    assert!(state.undo());
    assert_eq!(state.document.node("start").unwrap().position, Point::new(0.0, 0.0));
    assert_eq!(state.status_message.as_deref(), Some("已撤销上一步"));
    assert!(state.redo());
    assert_eq!(state.document.node("start").unwrap().position, Point::new(50.0, 0.0));
    assert_eq!(state.status_message.as_deref(), Some("已重做上一步"));
}

#[test]
fn undo_and_redo_trim_the_opposite_history_stack() {
    let document = WorkflowDocument {
        nodes: vec![test_node("start", "start", Point::new(0.0, 0.0))],
        ..WorkflowDocument::default()
    };
    let mut undo_state = state_with_document(document.clone());
    let base = undo_state.current_history_snapshot().expect("active snapshot");
    undo_state.redo_stack = (0..WORKFLOW_HISTORY_LIMIT)
        .map(|index| {
            let mut snapshot = base.clone();
            snapshot.pan = Vector::new(index as f32, 0.0);
            snapshot
        })
        .collect();
    let mut previous = base.clone();
    previous.pan = Vector::new(99.0, 0.0);
    undo_state.undo_stack.push(previous);

    assert!(undo_state.undo());
    assert_eq!(undo_state.redo_stack.len(), WORKFLOW_HISTORY_LIMIT);
    assert_eq!(undo_state.redo_stack[0].pan, Vector::new(1.0, 0.0));
    assert_eq!(undo_state.pan, Vector::new(99.0, 0.0));

    let mut redo_state = state_with_document(document);
    let base = redo_state.current_history_snapshot().expect("active snapshot");
    redo_state.undo_stack = (0..WORKFLOW_HISTORY_LIMIT)
        .map(|index| {
            let mut snapshot = base.clone();
            snapshot.pan = Vector::new(index as f32, 0.0);
            snapshot
        })
        .collect();
    let mut next = base;
    next.pan = Vector::new(99.0, 0.0);
    redo_state.redo_stack.push(next);

    assert!(redo_state.redo());
    assert_eq!(redo_state.undo_stack.len(), WORKFLOW_HISTORY_LIMIT);
    assert_eq!(redo_state.undo_stack[0].pan, Vector::new(1.0, 0.0));
    assert_eq!(redo_state.pan, Vector::new(99.0, 0.0));
}

#[test]
fn undo_and_redo_return_false_without_an_active_app() {
    let document = WorkflowDocument {
        nodes: vec![test_node("start", "start", Point::new(0.0, 0.0))],
        ..WorkflowDocument::default()
    };
    let mut undo_state = state_with_document(document.clone());
    let undo_snapshot = undo_state.current_history_snapshot().expect("active snapshot");
    undo_state.undo_stack.push(undo_snapshot);
    undo_state.active_app_id = None;

    assert!(!undo_state.undo());
    assert_eq!(undo_state.document, document);

    let mut redo_state = state_with_document(document.clone());
    let redo_snapshot = redo_state.current_history_snapshot().expect("active snapshot");
    redo_state.redo_stack.push(redo_snapshot);
    redo_state.active_app_id = None;

    assert!(!redo_state.redo());
    assert_eq!(redo_state.document, document);
}

fn test_node(id: &str, block_type: &str, position: Point) -> WorkflowNode {
    WorkflowNode {
        id: id.to_string(),
        block_type: block_type.to_string(),
        title: id.to_string(),
        description: String::new(),
        position,
        size: Size::new(180.0, 120.0),
        parent_id: None,
        selected: false,
        source_side: WorkflowHandleSide::Right,
        target_side: WorkflowHandleSide::Left,
        source_handles: vec![WorkflowHandle {
            id: "source".to_string(),
            label: String::new(),
            kind: WorkflowHandleKind::Source,
        }],
        target_handles: if block_type == "start" {
            Vec::new()
        } else {
            vec![WorkflowHandle {
                id: "target".to_string(),
                label: String::new(),
                kind: WorkflowHandleKind::Target,
            }]
        },
        z_index: 0.0,
        raw_node: Value::Null,
    }
}

fn test_edge(source: &str, target: &str, id: &str) -> WorkflowEdge {
    WorkflowEdge {
        id: id.to_string(),
        source: source.to_string(),
        target: target.to_string(),
        source_handle: Some("source".to_string()),
        target_handle: Some("target".to_string()),
        source_type: source.to_string(),
        target_type: target.to_string(),
        selected: false,
        z_index: 0.0,
        raw_edge: Value::Null,
    }
}

fn state_with_document(document: WorkflowDocument) -> WorkflowState {
    let snapshot = WorkflowHistorySnapshot {
        meta: WorkflowAppMeta { name: "App".to_string(), ..Default::default() },
        document: document.clone(),
        environment_variables: Vec::new(),
        conversation_variables: Vec::new(),
        pan: Vector::new(0.0, 0.0),
        zoom: 1.0,
        selected_node_id: None,
        selected_edge_id: None,
    };
    WorkflowState {
        apps: vec![WorkflowAppEntry {
            id: "app".to_string(),
            local_uuid: None,
            meta: WorkflowAppMeta { name: "App".to_string(), ..Default::default() },
            source_path: None,
            raw_root: Value::Null,
            document: document.clone(),
            environment_variables: Vec::new(),
            conversation_variables: Vec::new(),
            pan: Vector::new(0.0, 0.0),
            zoom: 1.0,
            selected_node_id: None,
            selected_edge_id: None,
            connection_draft: None,
            is_dirty: false,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            saved_snapshot: snapshot.clone(),
        }],
        active_app_id: Some("app".to_string()),
        source_name: "App".to_string(),
        document,
        saved_snapshot: Some(snapshot),
        pan: Vector::new(0.0, 0.0),
        zoom: 1.0,
        ..WorkflowState::default()
    }
}

fn endpoint(
    node_id: &str,
    handle_id: &str,
    kind: WorkflowHandleKind,
) -> WorkflowConnectionEndpoint {
    WorkflowConnectionEndpoint {
        node_id: node_id.to_string(),
        handle_id: handle_id.to_string(),
        kind,
    }
}

fn context_menu(target: WorkflowCanvasContextMenuTarget) -> WorkflowCanvasContextMenu {
    WorkflowCanvasContextMenu { target, anchor: Point::new(0.0, 0.0), world: Point::new(0.0, 0.0) }
}

fn node_editor_editing(node_id: &str) -> WorkflowNodeEditorDraft {
    WorkflowNodeEditorDraft {
        mode: WorkflowNodeEditorMode::Edit(node_id.to_string()),
        active_tab: WorkflowNodeEditorTab::Description,
        block_type: "answer".to_string(),
        title: "answer".to_string(),
        description: String::new(),
        description_editor: text_editor::Content::with_text(""),
        position: Point::new(0.0, 0.0),
        visual_draft: None,
        validation: WorkflowNodeEditorValidation::default(),
        show_raw_data_editor: false,
        raw_data_editor: text_editor::Content::with_text(""),
        hovered_start_variable_index: None,
        start_variable_focus_index: None,
        start_variable_editor: None,
    }
}
