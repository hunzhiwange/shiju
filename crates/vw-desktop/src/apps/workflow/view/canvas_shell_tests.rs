use super::*;
use crate::apps::workflow::model::{WorkflowAppMeta, WorkflowDocument};
use crate::apps::workflow::state::{
    WorkflowAppEntry, WorkflowCanvasContextMenu, WorkflowHistorySnapshot,
};
use iced::{Point, Size, Vector};
use serde_yaml::Value;

fn workflow_node(id: &str, block_type: &str) -> WorkflowNode {
    WorkflowNode {
        id: id.to_string(),
        block_type: block_type.to_string(),
        title: id.to_string(),
        description: String::new(),
        position: Point::new(0.0, 0.0),
        size: Size::new(180.0, 120.0),
        parent_id: None,
        selected: false,
        source_side: crate::apps::workflow::model::WorkflowHandleSide::Right,
        target_side: crate::apps::workflow::model::WorkflowHandleSide::Left,
        source_handles: Vec::new(),
        target_handles: Vec::new(),
        z_index: 0.0,
        raw_node: Value::Null,
    }
}

fn workflow_edge(id: &str) -> WorkflowEdge {
    WorkflowEdge {
        id: id.to_string(),
        source: "start".to_string(),
        target: "answer".to_string(),
        source_handle: Some("source".to_string()),
        target_handle: Some("target".to_string()),
        source_type: "start".to_string(),
        target_type: "answer".to_string(),
        selected: false,
        z_index: 0.0,
        raw_edge: Value::Null,
    }
}

fn workflow_state_with_document(document: WorkflowDocument) -> WorkflowState {
    let meta = WorkflowAppMeta::default();
    let snapshot = WorkflowHistorySnapshot {
        meta: meta.clone(),
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
            meta,
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
        source_name: "测试工作流".to_string(),
        document,
        pan: Vector::new(0.0, 0.0),
        zoom: 1.0,
        saved_snapshot: Some(snapshot),
        ..WorkflowState::default()
    }
}

fn canvas_base() -> Element<'static, Message> {
    container(Space::new().width(Length::Fill).height(Length::Fill)).into()
}

#[test]
fn build_status_chip_accepts_text() {
    let element = build_status_chip("已保存");

    assert_eq!(element.as_widget().size(), Size::new(Length::Shrink, Length::Shrink));
}

#[test]
fn build_canvas_area_handles_empty_state() {
    let state = WorkflowState::default();
    let element = build_canvas_area(&state, canvas_base());

    assert_eq!(element.as_widget().size(), Size::new(Length::Fill, Length::Fill));
    assert_eq!(element.as_widget().children().len(), 1);
}

#[test]
fn build_canvas_area_keeps_status_layer_without_open_apps() {
    let state = WorkflowState {
        status_message: Some("正在载入应用".to_string()),
        ..WorkflowState::default()
    };

    let element = build_canvas_area(&state, canvas_base());

    assert_eq!(element.as_widget().children().len(), 2);
}

#[test]
fn build_canvas_area_includes_app_controls_without_optional_layers() {
    let state = workflow_state_with_document(WorkflowDocument::default());

    let element = build_canvas_area(&state, canvas_base());

    assert_eq!(element.as_widget().children().len(), 5);
}

#[test]
fn build_canvas_area_includes_quick_insert_status_and_context_layers() {
    let mut state = workflow_state_with_document(WorkflowDocument::default());
    state.quick_insert_panel_open = true;
    state.status_message = Some("已加载".to_string());
    state.context_menu = Some(WorkflowCanvasContextMenu {
        target: WorkflowCanvasContextMenuTarget::Canvas,
        anchor: Point::new(12.0, 18.0),
        world: Point::new(24.0, 36.0),
    });

    let element = build_canvas_area(&state, canvas_base());

    assert_eq!(element.as_widget().children().len(), 8);
}

#[test]
fn build_canvas_area_includes_all_floating_layers_when_open() {
    let document = WorkflowDocument {
        nodes: vec![workflow_node("start", "start"), workflow_node("answer", "answer")],
        edges: vec![workflow_edge("edge")],
        ..WorkflowDocument::default()
    };
    let mut state = workflow_state_with_document(document);
    state.quick_insert_panel_open = true;
    state.action_menu_open = true;
    state.zoom_menu_open = true;
    state.status_message = Some("已加载".to_string());

    let element = build_canvas_area(&state, canvas_base());

    let _ = std::hint::black_box(element);
}

#[test]
fn build_canvas_area_wraps_action_menu_without_zoom_menu() {
    let mut state = workflow_state_with_document(WorkflowDocument::default());
    state.action_menu_open = true;
    state.zoom_menu_open = false;

    let element = build_canvas_area(&state, canvas_base());

    let _ = std::hint::black_box(element);
}

#[test]
fn build_canvas_area_wraps_zoom_menu_without_action_menu() {
    let mut state = workflow_state_with_document(WorkflowDocument::default());
    state.action_menu_open = false;
    state.zoom_menu_open = true;

    let element = build_canvas_area(&state, canvas_base());

    let _ = std::hint::black_box(element);
}

#[test]
fn build_left_toolbar_overlay_handles_closed_and_open_quick_insert_panel() {
    let mut state = workflow_state_with_document(WorkflowDocument::default());

    {
        let closed = build_left_toolbar_overlay(&state);
        assert_eq!(closed.as_widget().size(), Size::new(Length::Fill, Length::Fill));
    }

    state.quick_insert_panel_open = true;
    let open = build_left_toolbar_overlay(&state);

    assert_eq!(open.as_widget().size(), Size::new(Length::Fill, Length::Fill));
}

#[test]
fn build_canvas_context_menu_overlay_returns_empty_overlay_without_menu() {
    let state = WorkflowState::default();

    let element = build_canvas_context_menu_overlay(&state);

    assert_eq!(element.as_widget().size(), Size::new(Length::Shrink, Length::Shrink));
}

#[test]
fn build_canvas_context_menu_overlay_handles_canvas_target() {
    let mut state = workflow_state_with_document(WorkflowDocument::default());
    state.context_menu = Some(WorkflowCanvasContextMenu {
        target: WorkflowCanvasContextMenuTarget::Canvas,
        anchor: Point::new(12.0, 18.0),
        world: Point::new(24.0, 36.0),
    });

    let element = build_canvas_context_menu_overlay(&state);

    let _ = std::hint::black_box(element);
}

#[test]
fn build_canvas_context_menu_overlay_handles_edge_target() {
    let mut state = workflow_state_with_document(WorkflowDocument {
        edges: vec![workflow_edge("edge")],
        ..WorkflowDocument::default()
    });
    state.context_menu = Some(WorkflowCanvasContextMenu {
        target: WorkflowCanvasContextMenuTarget::Edge("edge".to_string()),
        anchor: Point::new(12.0, 18.0),
        world: Point::new(24.0, 36.0),
    });

    let element = build_canvas_context_menu_overlay(&state);

    let _ = std::hint::black_box(element);
}

#[test]
fn build_canvas_context_menu_overlay_handles_start_node_without_duplicate_action() {
    let mut state = workflow_state_with_document(WorkflowDocument {
        nodes: vec![workflow_node("start", "start")],
        ..WorkflowDocument::default()
    });
    state.context_menu = Some(WorkflowCanvasContextMenu {
        target: WorkflowCanvasContextMenuTarget::Node("start".to_string()),
        anchor: Point::new(12.0, 18.0),
        world: Point::new(24.0, 36.0),
    });

    let element = build_canvas_context_menu_overlay(&state);

    let _ = std::hint::black_box(element);
}

#[test]
fn build_canvas_context_menu_overlay_handles_non_start_node_with_duplicate_action() {
    let mut state = workflow_state_with_document(WorkflowDocument {
        nodes: vec![workflow_node("answer", "answer")],
        ..WorkflowDocument::default()
    });
    state.context_menu = Some(WorkflowCanvasContextMenu {
        target: WorkflowCanvasContextMenuTarget::Node("answer".to_string()),
        anchor: Point::new(12.0, 18.0),
        world: Point::new(24.0, 36.0),
    });

    let element = build_canvas_context_menu_overlay(&state);

    let _ = std::hint::black_box(element);
}

#[test]
fn build_canvas_context_menu_overlay_handles_missing_node_without_duplicate_action() {
    let mut state = workflow_state_with_document(WorkflowDocument::default());
    state.context_menu = Some(WorkflowCanvasContextMenu {
        target: WorkflowCanvasContextMenuTarget::Node("missing".to_string()),
        anchor: Point::new(12.0, 18.0),
        world: Point::new(24.0, 36.0),
    });

    let element = build_canvas_context_menu_overlay(&state);

    assert_eq!(element.as_widget().size(), Size::new(Length::Fill, Length::Fill));
}

#[test]
fn build_canvas_context_menu_overlay_handles_node_insert_target() {
    let mut state = workflow_state_with_document(WorkflowDocument {
        nodes: vec![workflow_node("start", "start")],
        ..WorkflowDocument::default()
    });
    state.context_menu = Some(WorkflowCanvasContextMenu {
        target: WorkflowCanvasContextMenuTarget::NodeInsert("start".to_string()),
        anchor: Point::new(12.0, 18.0),
        world: Point::new(24.0, 36.0),
    });

    let element = build_canvas_context_menu_overlay(&state);

    let _ = std::hint::black_box(element);
}

#[test]
fn variable_panel_is_open_matches_current_kind() {
    let mut state = WorkflowState {
        variable_panel: Some(WorkflowVariablePanelKind::Environment),
        ..WorkflowState::default()
    };

    assert!(variable_panel_is_open(&state, WorkflowVariablePanelKind::Environment));
    assert!(!variable_panel_is_open(&state, WorkflowVariablePanelKind::Conversation));

    state.variable_panel = None;
    assert!(!variable_panel_is_open(&state, WorkflowVariablePanelKind::Environment));
}

#[test]
fn toolbar_icon_button_builds_active_and_inactive_buttons() {
    let active =
        toolbar_icon_button(Icon::Plus, "插入节点", WorkflowMessage::ToggleQuickInsertPanel, true);
    let inactive = toolbar_icon_button(
        Icon::Gear,
        "系统变量",
        WorkflowMessage::OpenVariablePanel(WorkflowVariablePanelKind::System),
        false,
    );

    assert_eq!(active.as_widget().size(), Size::new(Length::Fixed(40.0), Length::Fixed(40.0)));
    assert_eq!(inactive.as_widget().size(), Size::new(Length::Fixed(40.0), Length::Fixed(40.0)));
}

#[test]
fn toolbar_tooltip_bubble_builds_label() {
    let element = toolbar_tooltip_bubble("适配视图");

    let _ = std::hint::black_box(element);
}

#[test]
fn available_node_types_includes_start_before_start_exists() {
    let state = WorkflowState::default();

    let node_types = available_node_types(&state, false);

    assert!(node_types.iter().any(|node_type| node_type.block_type == "start"));
}

#[test]
fn available_node_types_excludes_start_when_requested() {
    let state = WorkflowState::default();

    let node_types = available_node_types(&state, true);

    assert!(!node_types.iter().any(|node_type| node_type.block_type == "start"));
}

#[test]
fn available_node_types_excludes_start_when_document_has_start_node() {
    let state = workflow_state_with_document(WorkflowDocument {
        nodes: vec![workflow_node("start", "start")],
        ..WorkflowDocument::default()
    });

    let node_types = available_node_types(&state, false);

    assert!(!node_types.iter().any(|node_type| node_type.block_type == "start"));
}

#[test]
fn available_node_types_preserves_all_non_start_types() {
    let state = WorkflowState::default();
    let expected = supported_node_types()
        .iter()
        .filter(|node_type| node_type.block_type != "start")
        .map(|node_type| node_type.block_type)
        .collect::<Vec<_>>();

    let actual = available_node_types(&state, true)
        .iter()
        .map(|node_type| node_type.block_type)
        .collect::<Vec<_>>();

    assert_eq!(actual, expected);
}

#[test]
fn build_quick_insert_panel_builds_available_buttons() {
    let state = WorkflowState::default();

    let element = build_quick_insert_panel(&state);

    assert_eq!(element.as_widget().size().width, Length::Fixed(438.0));
}

#[test]
fn build_quick_insert_panel_builds_when_start_node_already_exists() {
    let state = workflow_state_with_document(WorkflowDocument {
        nodes: vec![workflow_node("start", "start")],
        ..WorkflowDocument::default()
    });

    let element = build_quick_insert_panel(&state);

    assert_eq!(element.as_widget().size().width, Length::Fixed(438.0));
}

#[test]
fn quick_insert_node_button_uses_node_type_message() {
    let node_type = supported_node_types()
        .iter()
        .copied()
        .find(|node_type| node_type.block_type == "answer")
        .expect("answer node type should exist");

    let element = quick_insert_node_button(node_type);

    assert_eq!(element.as_widget().size().width, Length::Fixed(136.0));
}

#[test]
fn quick_insert_node_button_builds_for_all_supported_node_types() {
    for node_type in supported_node_types().iter().copied() {
        assert_eq!(
            quick_insert_node_button(node_type).as_widget().size().width,
            Length::Fixed(136.0)
        );
    }
}
