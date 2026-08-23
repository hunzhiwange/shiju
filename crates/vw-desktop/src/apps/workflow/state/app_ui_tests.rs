//! 工作流应用界面状态的测试，覆盖面板开关、菜单互斥和编辑器焦点。
//! 用例验证界面瞬时状态切换不会意外标记文档为已修改。
#[test]
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("app_ui_tests"));
}

use super::*;
use crate::apps::workflow::model::{WorkflowHandleSide, WorkflowViewport};
use iced::Size;

#[test]
fn organize_active_app_separates_overlapped_connected_nodes() {
    let document = WorkflowDocument {
        nodes: vec![test_node("start", "start"), test_node("answer", "answer")],
        edges: vec![test_edge("start", "answer")],
        ..WorkflowDocument::default()
    };
    let snapshot = test_snapshot(&document);
    let mut state = WorkflowState {
        apps: vec![WorkflowAppEntry {
            id: "app".to_string(),
            local_uuid: None,
            meta: WorkflowAppMeta::default(),
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
        document,
        saved_snapshot: Some(snapshot),
        ..WorkflowState::default()
    };

    state.organize_active_app((1280.0, 860.0)).expect("layout should succeed");

    let start = state.document.node("start").expect("start node should exist");
    let answer = state.document.node("answer").expect("answer node should exist");
    assert!(answer.position.x > start.position.x + start.size.width);
    assert_ne!(answer.position.y, 0.0);
    assert!(state.active_is_dirty);
    assert_eq!(state.undo_stack.len(), 1);
}

#[test]
fn organized_node_positions_returns_empty_for_empty_document_and_layers_chain_edges() {
    assert!(organized_node_positions(&WorkflowDocument::default()).is_empty());

    let document = WorkflowDocument {
        nodes: vec![
            WorkflowNode { size: Size::new(100.0, 80.0), ..test_node("a", "start") },
            WorkflowNode { size: Size::new(220.0, 140.0), ..test_node("b", "llm") },
            WorkflowNode { size: Size::new(120.0, 120.0), ..test_node("c", "answer") },
        ],
        edges: vec![test_edge("a", "b"), test_edge("b", "c")],
        ..WorkflowDocument::default()
    };

    let positions = organized_node_positions(&document);

    assert_eq!(positions.get("a"), Some(&Point::new(-180.0, 120.0)));
    assert!(positions["b"].x > positions["a"].x);
    assert!(positions["c"].x > positions["b"].x);
    assert_eq!(positions["b"].y, 120.0);
    assert_eq!(positions["c"].y, 120.0);
}

#[test]
fn open_context_menu_selects_target_and_closes_other_interactions() {
    let document = WorkflowDocument {
        nodes: vec![test_node("start", "start"), test_node("answer", "answer")],
        edges: vec![test_edge("start", "answer")],
        ..WorkflowDocument::default()
    };
    let mut state = state_with_document(document);
    state.quick_insert_panel_open = true;
    state.action_menu_open = true;
    state.zoom_menu_open = true;
    state.connection_draft = Some(WorkflowConnectionDraft {
        from: WorkflowConnectionEndpoint {
            node_id: "start".to_string(),
            handle_id: "source".to_string(),
            kind: WorkflowHandleKind::Source,
        },
        cursor_world: Point::new(0.0, 0.0),
    });

    state.open_context_menu(
        WorkflowCanvasContextMenuTarget::Node("answer".to_string()),
        Point::new(1.0, 2.0),
        Point::new(3.0, 4.0),
    );

    assert_eq!(state.selected_node_id.as_deref(), Some("answer"));
    assert!(state.selected_edge_id.is_none());
    assert!(!state.quick_insert_panel_open);
    assert!(!state.action_menu_open);
    assert!(!state.zoom_menu_open);
    assert!(state.connection_draft.is_none());
    assert!(state.document.node("answer").unwrap().selected);

    state.open_context_menu(
        WorkflowCanvasContextMenuTarget::Edge("start-answer".to_string()),
        Point::new(5.0, 6.0),
        Point::new(7.0, 8.0),
    );
    assert_eq!(state.selected_edge_id.as_deref(), Some("start-answer"));
    assert!(state.selected_node_id.is_none());
    assert!(state.document.edge("start-answer").unwrap().selected);

    state.open_context_menu(
        WorkflowCanvasContextMenuTarget::Canvas,
        Point::new(9.0, 10.0),
        Point::new(11.0, 12.0),
    );
    assert!(state.selected_node_id.is_none());
    assert!(state.selected_edge_id.is_none());

    state.close_context_menu();
    assert!(state.context_menu.is_none());
}

#[test]
fn context_menu_new_node_position_uses_canvas_world_or_node_right_side() {
    let document = WorkflowDocument {
        nodes: vec![WorkflowNode {
            position: Point::new(20.0, 30.0),
            size: Size::new(200.0, 120.0),
            ..test_node("start", "start")
        }],
        ..WorkflowDocument::default()
    };
    let mut state = state_with_document(document);

    assert_eq!(state.context_menu_new_node_position(), Point::new(80.0, 80.0));

    state.open_context_menu(
        WorkflowCanvasContextMenuTarget::Canvas,
        Point::new(0.0, 0.0),
        Point::new(300.0, 400.0),
    );
    assert_eq!(state.context_menu_new_node_position(), Point::new(300.0, 400.0));
    assert!(state.context_menu_auto_connect_source_node_id().is_none());

    state.open_context_menu(
        WorkflowCanvasContextMenuTarget::NodeInsert("start".to_string()),
        Point::new(0.0, 0.0),
        Point::new(500.0, 600.0),
    );
    assert_eq!(state.context_menu_new_node_position(), Point::new(340.0, 48.0));
    assert_eq!(state.context_menu_auto_connect_source_node_id().as_deref(), Some("start"));

    state.open_context_menu(
        WorkflowCanvasContextMenuTarget::Node("missing".to_string()),
        Point::new(0.0, 0.0),
        Point::new(700.0, 800.0),
    );
    assert_eq!(state.context_menu_new_node_position(), Point::new(700.0, 800.0));
}

#[test]
fn context_menu_edge_uses_world_and_node_insert_selects_node() {
    let document = WorkflowDocument {
        nodes: vec![test_node("start", "start")],
        edges: vec![test_edge("start", "missing")],
        ..WorkflowDocument::default()
    };
    let mut state = state_with_document(document);

    state.open_context_menu(
        WorkflowCanvasContextMenuTarget::Edge("start-missing".to_string()),
        Point::new(1.0, 2.0),
        Point::new(33.0, 44.0),
    );

    assert_eq!(state.context_menu_new_node_position(), Point::new(33.0, 44.0));
    assert!(state.context_menu_auto_connect_source_node_id().is_none());

    state.open_context_menu(
        WorkflowCanvasContextMenuTarget::NodeInsert("start".to_string()),
        Point::new(5.0, 6.0),
        Point::new(7.0, 8.0),
    );

    assert_eq!(state.selected_node_id.as_deref(), Some("start"));
    assert!(state.selected_edge_id.is_none());
    assert!(state.document.node("start").unwrap().selected);
}

#[test]
fn editor_setters_are_noops_without_editor_and_update_draft_when_present() {
    let mut state = WorkflowState::default();

    state.set_editor_name("Ignored".to_string());
    state.set_editor_description("Ignored".to_string());
    state.set_editor_icon("Ignored".to_string());
    state.set_editor_use_icon_as_answer_icon(true);
    state.set_editor_max_active_requests_input("9".to_string());
    assert!(state.app_editor.is_none());

    state.open_create_editor();
    state.set_editor_name("Name".to_string());
    state.set_editor_description("Description".to_string());
    state.set_editor_icon("Icon".to_string());
    state.set_editor_use_icon_as_answer_icon(false);
    state.set_editor_max_active_requests_input("7".to_string());

    let editor = state.app_editor.as_ref().expect("editor should remain open");
    assert_eq!(editor.name, "Name");
    assert_eq!(editor.description, "Description");
    assert_eq!(editor.icon, "Icon");
    assert!(!editor.use_icon_as_answer_icon);
    assert_eq!(editor.max_active_requests_input, "7");
}

#[test]
fn submit_editor_validates_input_creates_app_and_edits_active_app() {
    let mut state = WorkflowState::default();
    assert!(state.submit_editor((800.0, 600.0), loaded_workflow("Ignored")).is_ok());

    state.app_editor = Some(WorkflowAppEditorDraft {
        mode: WorkflowAppEditorMode::Create,
        name: "  Created App  ".to_string(),
        description: "  Desc  ".to_string(),
        icon: "  🚀  ".to_string(),
        use_icon_as_answer_icon: true,
        max_active_requests_input: "3".to_string(),
    });
    state
        .submit_editor((800.0, 600.0), loaded_workflow("Loaded"))
        .expect("create editor should submit");

    assert_eq!(state.source_name, "Created App");
    assert_eq!(state.apps.len(), 1);
    assert_eq!(state.active_meta().unwrap().name, "Created App");
    assert_eq!(state.active_meta().unwrap().description, "Desc");
    assert_eq!(state.active_meta().unwrap().icon, "🚀");
    assert_eq!(state.active_meta().unwrap().max_active_requests, 3);
    assert!(state.active_meta().unwrap().use_icon_as_answer_icon);
    assert!(state.app_editor.is_none());
    assert_eq!(state.status_message.as_deref(), Some("已更新应用信息"));

    state.open_edit_editor(None);
    state.set_editor_name("  Edited App  ".to_string());
    state.set_editor_description("  Edited Desc  ".to_string());
    state.set_editor_icon("   ".to_string());
    state.set_editor_use_icon_as_answer_icon(false);
    state.set_editor_max_active_requests_input("5".to_string());
    state
        .submit_editor((800.0, 600.0), loaded_workflow("Unused"))
        .expect("edit editor should submit");

    assert_eq!(state.source_name, "Edited App");
    assert_eq!(state.active_meta().unwrap().name, "Edited App");
    assert_eq!(state.active_meta().unwrap().description, "Edited Desc");
    assert_eq!(state.active_meta().unwrap().icon, "🤖");
    assert_eq!(state.active_meta().unwrap().max_active_requests, 5);
    assert!(state.active_is_dirty);
    assert_eq!(state.undo_stack.len(), 1);
}

#[test]
fn submit_editor_reports_invalid_name_or_max_requests_and_marks_inactive_edits_dirty() {
    let document = WorkflowDocument {
        nodes: vec![test_node("start", "start")],
        ..WorkflowDocument::default()
    };
    let snapshot = test_snapshot(&document);
    let mut state = WorkflowState {
        apps: vec![
            WorkflowAppEntry {
                id: "active".to_string(),
                local_uuid: None,
                meta: WorkflowAppMeta { name: "Active".to_string(), ..Default::default() },
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
            },
            WorkflowAppEntry {
                id: "inactive".to_string(),
                local_uuid: None,
                meta: WorkflowAppMeta { name: "Inactive".to_string(), ..Default::default() },
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
            },
        ],
        active_app_id: Some("active".to_string()),
        source_name: "Active".to_string(),
        document,
        saved_snapshot: Some(snapshot),
        ..WorkflowState::default()
    };

    state.app_editor = Some(WorkflowAppEditorDraft {
        mode: WorkflowAppEditorMode::Edit("active".to_string()),
        name: "Valid".to_string(),
        description: String::new(),
        icon: "🤖".to_string(),
        use_icon_as_answer_icon: false,
        max_active_requests_input: "-1".to_string(),
    });
    assert_eq!(
        state.submit_editor((800.0, 600.0), loaded_workflow("Unused")).unwrap_err(),
        "最大活跃请求数必须是非负整数"
    );

    state.app_editor.as_mut().unwrap().max_active_requests_input = "0".to_string();
    state.app_editor.as_mut().unwrap().name = "   ".to_string();
    assert_eq!(
        state.submit_editor((800.0, 600.0), loaded_workflow("Unused")).unwrap_err(),
        "应用名称不能为空"
    );

    state.app_editor = Some(WorkflowAppEditorDraft {
        mode: WorkflowAppEditorMode::Edit("inactive".to_string()),
        name: "Renamed Inactive".to_string(),
        description: "Desc".to_string(),
        icon: "I".to_string(),
        use_icon_as_answer_icon: true,
        max_active_requests_input: "2".to_string(),
    });
    state
        .submit_editor((800.0, 600.0), loaded_workflow("Unused"))
        .expect("inactive edit should submit");

    let inactive = state.apps.iter().find(|app| app.id == "inactive").unwrap();
    assert_eq!(inactive.meta.name, "Renamed Inactive");
    assert!(inactive.is_dirty);
    assert_eq!(state.source_name, "Active");
    assert!(state.undo_stack.is_empty());
}

#[test]
fn submit_editor_for_missing_target_still_closes_editor_without_dirtying_active_app() {
    let document = WorkflowDocument {
        nodes: vec![test_node("start", "start")],
        ..WorkflowDocument::default()
    };
    let mut state = state_with_document(document);
    state.action_menu_open = true;
    state.app_editor = Some(WorkflowAppEditorDraft {
        mode: WorkflowAppEditorMode::Edit("missing".to_string()),
        name: "Missing".to_string(),
        description: "Desc".to_string(),
        icon: "M".to_string(),
        use_icon_as_answer_icon: true,
        max_active_requests_input: "1".to_string(),
    });

    state
        .submit_editor((800.0, 600.0), loaded_workflow("Unused"))
        .expect("missing target edit should be accepted as a closed editor");

    assert!(state.app_editor.is_none());
    assert!(!state.action_menu_open);
    assert_eq!(state.status_message.as_deref(), Some("已更新应用信息"));
    assert_eq!(state.apps[0].meta.name, "App");
    assert!(!state.active_is_dirty);
    assert!(state.undo_stack.is_empty());
}

#[test]
fn organize_active_app_reports_missing_active_or_empty_document() {
    let mut state = WorkflowState::default();
    assert_eq!(state.organize_active_app((800.0, 600.0)).unwrap_err(), "请先打开一个应用");

    let document = WorkflowDocument::default();
    let snapshot = test_snapshot(&document);
    state.apps = vec![WorkflowAppEntry {
        id: "app".to_string(),
        local_uuid: None,
        meta: WorkflowAppMeta::default(),
        source_path: None,
        raw_root: Value::Null,
        document,
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
    }];
    state.active_app_id = Some("app".to_string());
    state.saved_snapshot = Some(snapshot);
    assert_eq!(state.organize_active_app((800.0, 600.0)).unwrap_err(), "当前应用没有可整理的节点");
}

#[test]
fn organize_active_app_handles_cycles_missing_edges_and_resets_transient_ui_state() {
    let document = WorkflowDocument {
        nodes: vec![
            WorkflowNode { size: Size::new(120.0, 90.0), ..test_node("a", "start") },
            WorkflowNode { size: Size::new(360.0, 160.0), ..test_node("b", "llm") },
            WorkflowNode { size: Size::new(180.0, 80.0), ..test_node("c", "answer") },
        ],
        edges: vec![test_edge("a", "b"), test_edge("b", "a"), test_edge("missing", "c")],
        ..WorkflowDocument::default()
    };
    let mut state = state_with_document(document);
    state.context_menu = Some(WorkflowCanvasContextMenu {
        target: WorkflowCanvasContextMenuTarget::Canvas,
        anchor: Point::new(1.0, 1.0),
        world: Point::new(2.0, 2.0),
    });
    state.quick_insert_panel_open = true;
    state.action_menu_open = true;
    state.zoom_menu_open = true;
    state.dragging_node_id = Some("a".to_string());
    state.drag_pending_snapshot = Some(test_snapshot(&state.document));

    state.organize_active_app((1280.0, 860.0)).expect("layout should succeed");

    let a = state.document.node("a").expect("a node should exist");
    let b = state.document.node("b").expect("b node should exist");
    let c = state.document.node("c").expect("c node should exist");
    assert_ne!(a.position, b.position);
    assert_eq!(c.position, Point::new(-180.0, 120.0));
    assert!(state.context_menu.is_none());
    assert!(!state.quick_insert_panel_open);
    assert!(!state.action_menu_open);
    assert!(!state.zoom_menu_open);
    assert!(state.dragging_node_id.is_none());
    assert!(state.drag_pending_snapshot.is_none());
    assert_eq!(state.status_message.as_deref(), Some("已整理应用节点位置"));
}

#[test]
fn persist_active_snapshot_updates_entry_and_active_entry_snapshot_returns_clone() {
    let document = WorkflowDocument {
        nodes: vec![test_node("start", "start")],
        ..WorkflowDocument::default()
    };
    let mut state = state_with_document(document.clone());
    state.local_uuid = Some("uuid".to_string());
    state.source_path = Some("/tmp/app.yml".to_string());
    state.source_name = "Renamed".to_string();
    state.pan = Vector::new(10.0, 20.0);
    state.zoom = 2.0;
    state.selected_node_id = Some("start".to_string());
    state.environment_variables.push(WorkflowEnvironmentVariable {
        id: "env".to_string(),
        name: "env".to_string(),
        value_type: "string".to_string(),
        value: Value::String("value".to_string()),
        description: String::new(),
        raw_variable: Value::Null,
    });
    state.saved_snapshot = None;

    let entry = state.active_entry_snapshot().expect("active entry should exist");

    assert_eq!(entry.local_uuid.as_deref(), Some("uuid"));
    assert_eq!(entry.source_path.as_deref(), Some("/tmp/app.yml"));
    assert_eq!(entry.meta.name, "Renamed");
    assert_eq!(entry.pan, Vector::new(10.0, 20.0));
    assert_eq!(entry.zoom, 2.0);
    assert_eq!(entry.selected_node_id.as_deref(), Some("start"));
    assert_eq!(entry.environment_variables.len(), 1);
    assert_eq!(state.apps[0].saved_snapshot.meta.name, "Renamed");
}

#[test]
fn persist_active_snapshot_preserves_explicit_saved_snapshot_and_handles_missing_active() {
    let document = WorkflowDocument {
        nodes: vec![test_node("start", "start")],
        ..WorkflowDocument::default()
    };
    let explicit_snapshot = WorkflowHistorySnapshot {
        meta: WorkflowAppMeta { name: "Saved".to_string(), ..Default::default() },
        document: document.clone(),
        environment_variables: Vec::new(),
        conversation_variables: Vec::new(),
        pan: Vector::new(1.0, 2.0),
        zoom: 1.25,
        selected_node_id: Some("start".to_string()),
        selected_edge_id: None,
    };
    let mut state = state_with_document(document);
    state.saved_snapshot = Some(explicit_snapshot.clone());
    state.source_name = "App".to_string();

    state.persist_active_snapshot();

    assert_eq!(state.apps[0].saved_snapshot, explicit_snapshot);

    state.active_app_id = None;
    state.source_name = "Ignored".to_string();
    state.persist_active_snapshot();
    assert_eq!(state.apps[0].meta.name, "App");
    assert!(state.active_entry_snapshot().is_none());
}

fn test_node(id: &str, block_type: &str) -> WorkflowNode {
    WorkflowNode {
        id: id.to_string(),
        block_type: block_type.to_string(),
        title: id.to_string(),
        description: String::new(),
        position: Point::new(0.0, 0.0),
        size: Size::new(180.0, 120.0),
        parent_id: None,
        selected: false,
        source_side: WorkflowHandleSide::Right,
        target_side: WorkflowHandleSide::Left,
        source_handles: Vec::new(),
        target_handles: Vec::new(),
        z_index: 0.0,
        raw_node: Value::Null,
    }
}

fn test_edge(source: &str, target: &str) -> WorkflowEdge {
    WorkflowEdge {
        id: format!("{source}-{target}"),
        source: source.to_string(),
        target: target.to_string(),
        source_handle: Some("source".to_string()),
        target_handle: Some("target".to_string()),
        source_type: "start".to_string(),
        target_type: "answer".to_string(),
        selected: false,
        z_index: 0.0,
        raw_edge: Value::Null,
    }
}

fn test_snapshot(document: &WorkflowDocument) -> WorkflowHistorySnapshot {
    WorkflowHistorySnapshot {
        meta: WorkflowAppMeta::default(),
        document: document.clone(),
        environment_variables: Vec::new(),
        conversation_variables: Vec::new(),
        pan: Vector::new(0.0, 0.0),
        zoom: 1.0,
        selected_node_id: None,
        selected_edge_id: None,
    }
}

fn state_with_document(document: WorkflowDocument) -> WorkflowState {
    let snapshot = test_snapshot(&document);
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

fn loaded_workflow(name: &str) -> LoadedWorkflow {
    LoadedWorkflow {
        local_uuid: Some("loaded-uuid".to_string()),
        source_path: Some("/tmp/loaded.yml".to_string()),
        source_name: name.to_string(),
        app_meta: WorkflowAppMeta { name: name.to_string(), ..Default::default() },
        document: WorkflowDocument {
            name: name.to_string(),
            nodes: vec![test_node("loaded-start", "start")],
            edges: Vec::new(),
            viewport: WorkflowViewport { x: 30.0, y: 40.0, zoom: 2.0 },
        },
        environment_variables: Vec::new(),
        conversation_variables: Vec::new(),
        had_viewport: true,
        raw_root: Value::Null,
    }
}
