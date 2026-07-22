//! Workflow 节点操作测试，覆盖编辑器、节点插入、自动连线与失效边清理。

use super::*;
use crate::apps::workflow::model::{
    WorkflowDocument, WorkflowEdge, WorkflowHandle, WorkflowHandleKind,
};
use iced::{Point, Vector, widget::text_editor};
use serde_yaml::Value;

fn workflow_node(id: &str, block_type: &str, position: Point, z_index: f32) -> WorkflowNode {
    create_node_from_type(block_type, id.to_string(), position, z_index)
        .expect("node type should be supported")
}

fn workflow_edge(
    id: &str,
    source: &str,
    target: &str,
    source_handle: Option<&str>,
    target_handle: Option<&str>,
) -> WorkflowEdge {
    WorkflowEdge {
        id: id.to_string(),
        source: source.to_string(),
        target: target.to_string(),
        source_handle: source_handle.map(str::to_string),
        target_handle: target_handle.map(str::to_string),
        source_type: "start".to_string(),
        target_type: "answer".to_string(),
        selected: false,
        z_index: 1.0,
        raw_edge: Value::Null,
    }
}

fn workflow_state_with_nodes(nodes: Vec<WorkflowNode>) -> WorkflowState {
    WorkflowState {
        document: WorkflowDocument { nodes, ..WorkflowDocument::default() },
        pan: Vector::new(0.0, 0.0),
        zoom: 1.0,
        ..WorkflowState::default()
    }
}

fn code_editor_with_error_strategy(node_id: &str, error_strategy: &str) -> WorkflowNodeEditorDraft {
    WorkflowNodeEditorDraft {
        mode: WorkflowNodeEditorMode::Edit(node_id.to_string()),
        active_tab: WorkflowNodeEditorTab::Visual,
        block_type: "code".to_string(),
        title: "代码节点".to_string(),
        description: String::new(),
        description_editor: text_editor::Content::with_text(""),
        position: Point::new(0.0, 0.0),
        visual_draft: Some(WorkflowNodeVisualDraft::Code {
            language: "python3".to_string(),
            inputs: Vec::new(),
            code_editor: text_editor::Content::with_text("def main():\n    return {}\n"),
            outputs: Vec::new(),
            retry_config: WorkflowNodeRetryDraft {
                enabled: false,
                max_retries: 3,
                retry_interval: 500,
            },
            error_strategy: error_strategy.to_string(),
            default_value_editor: text_editor::Content::with_text("[]"),
        }),
        validation: WorkflowNodeEditorValidation::default(),
        show_raw_data_editor: false,
        raw_data_editor: text_editor::Content::with_text("{}"),
        hovered_start_variable_index: None,
        start_variable_focus_index: None,
        start_variable_editor: None,
    }
}

#[test]
fn open_create_node_editor_prepares_visual_draft_and_closes_panels() {
    let mut state = WorkflowState {
        context_menu: Some(WorkflowCanvasContextMenu {
            target: WorkflowCanvasContextMenuTarget::Canvas,
            anchor: Point::new(1.0, 2.0),
            world: Point::new(3.0, 4.0),
        }),
        quick_insert_panel_open: true,
        action_menu_open: true,
        zoom_menu_open: true,
        variable_panel: Some(WorkflowVariablePanelKind::System),
        app_editor: Some(WorkflowAppEditorDraft {
            mode: WorkflowAppEditorMode::Create,
            name: "应用".to_string(),
            description: String::new(),
            icon: "A".to_string(),
            use_icon_as_answer_icon: false,
            max_active_requests_input: "0".to_string(),
        }),
        ..WorkflowState::default()
    };

    state
        .open_create_node_editor("answer", Point::new(10.0, 20.0))
        .expect("answer editor should open");

    let editor = state.node_editor.as_ref().expect("node editor should exist");
    assert_eq!(editor.mode, WorkflowNodeEditorMode::Create);
    assert_eq!(editor.active_tab, WorkflowNodeEditorTab::Visual);
    assert_eq!(editor.block_type, "answer");
    assert_eq!(editor.title, "回复");
    assert_eq!(editor.position, Point::new(10.0, 20.0));
    assert!(matches!(editor.visual_draft, Some(WorkflowNodeVisualDraft::Answer { .. })));
    assert!(state.context_menu.is_none());
    assert!(!state.quick_insert_panel_open);
    assert!(!state.action_menu_open);
    assert!(!state.zoom_menu_open);
    assert!(state.variable_panel.is_none());
    assert!(state.app_editor.is_none());
}

#[test]
fn open_create_node_editor_uses_description_tab_for_unknown_node_type() {
    let mut state = WorkflowState::default();

    state
        .open_create_node_editor("custom-unknown", Point::new(10.0, 20.0))
        .expect("unknown node editor should still open");

    let editor = state.node_editor.as_ref().expect("node editor should exist");
    assert_eq!(editor.active_tab, WorkflowNodeEditorTab::Description);
    assert!(editor.visual_draft.is_none());
    assert_eq!(editor.block_type, "custom-unknown");
}

#[test]
fn open_create_node_editor_rejects_second_start_node() {
    let start = workflow_node("start_1", "start", Point::ORIGIN, 0.0);
    let mut state = workflow_state_with_nodes(vec![start]);

    let result = state.open_create_node_editor("start", Point::ORIGIN);

    assert_eq!(result, Err("开始节点只能有一个".to_string()));
    assert!(state.node_editor.is_none());
}

#[test]
fn open_edit_node_editor_uses_selected_node_when_id_is_absent() {
    let answer = workflow_node("answer_1", "answer", Point::new(5.0, 6.0), 2.0);
    let mut state = workflow_state_with_nodes(vec![answer]);
    state.selected_node_id = Some("answer_1".to_string());

    state.open_edit_node_editor(None).expect("selected node should open");

    let editor = state.node_editor.as_ref().expect("node editor should exist");
    assert_eq!(editor.mode, WorkflowNodeEditorMode::Edit("answer_1".to_string()));
    assert_eq!(editor.block_type, "answer");
    assert_eq!(editor.title, "回复");
    assert_eq!(editor.position, Point::new(5.0, 6.0));
    assert!(matches!(editor.visual_draft, Some(WorkflowNodeVisualDraft::Answer { .. })));
}

#[test]
fn open_edit_node_editor_with_explicit_id_closes_floating_editors() {
    let mut answer = workflow_node("answer_1", "answer", Point::new(5.0, 6.0), 2.0);
    answer.description = "现有说明".to_string();
    let mut state = WorkflowState {
        document: WorkflowDocument { nodes: vec![answer], ..WorkflowDocument::default() },
        context_menu: Some(WorkflowCanvasContextMenu {
            target: WorkflowCanvasContextMenuTarget::Node("answer_1".to_string()),
            anchor: Point::new(1.0, 2.0),
            world: Point::new(3.0, 4.0),
        }),
        quick_insert_panel_open: true,
        action_menu_open: true,
        zoom_menu_open: true,
        variable_panel: Some(WorkflowVariablePanelKind::Environment),
        app_editor: Some(WorkflowAppEditorDraft {
            mode: WorkflowAppEditorMode::Create,
            name: "应用草稿".to_string(),
            description: String::new(),
            icon: "A".to_string(),
            use_icon_as_answer_icon: false,
            max_active_requests_input: "0".to_string(),
        }),
        ..WorkflowState::default()
    };

    state.open_edit_node_editor(Some("answer_1")).expect("explicit node should open");

    let editor = state.node_editor.as_ref().expect("node editor should exist");
    assert_eq!(editor.mode, WorkflowNodeEditorMode::Edit("answer_1".to_string()));
    assert_eq!(editor.description, "现有说明");
    assert_eq!(editor.description_editor.text(), "现有说明");
    assert!(state.context_menu.is_none());
    assert!(!state.quick_insert_panel_open);
    assert!(!state.action_menu_open);
    assert!(!state.zoom_menu_open);
    assert!(state.variable_panel.is_none());
    assert!(state.app_editor.is_none());
}

#[test]
fn open_edit_node_editor_returns_clear_errors() {
    let mut state = WorkflowState::default();

    assert_eq!(state.open_edit_node_editor(None), Err("请先选择一个节点".to_string()));

    state.selected_node_id = Some("missing".to_string());
    assert_eq!(state.open_edit_node_editor(None), Err("目标节点不存在".to_string()));
}

#[test]
fn close_node_editor_and_set_active_tab_only_mutate_existing_editor() {
    let mut state = WorkflowState::default();

    state.set_node_editor_active_tab(WorkflowNodeEditorTab::AdvancedDsl);
    assert!(state.node_editor.is_none());

    state.open_create_node_editor("answer", Point::ORIGIN).expect("answer editor should open");
    state.set_node_editor_active_tab(WorkflowNodeEditorTab::Description);
    assert_eq!(
        state.node_editor.as_ref().map(|editor| editor.active_tab),
        Some(WorkflowNodeEditorTab::Description)
    );

    state.close_node_editor();
    assert!(state.node_editor.is_none());
}

#[test]
fn insert_node_immediately_adds_selected_node_and_status() {
    let mut state = WorkflowState {
        connection_draft: Some(WorkflowConnectionDraft {
            from: WorkflowConnectionEndpoint {
                node_id: "source".to_string(),
                handle_id: "source".to_string(),
                kind: WorkflowHandleKind::Source,
            },
            cursor_world: Point::new(9.0, 9.0),
        }),
        context_menu: Some(WorkflowCanvasContextMenu {
            target: WorkflowCanvasContextMenuTarget::Canvas,
            anchor: Point::ORIGIN,
            world: Point::ORIGIN,
        }),
        quick_insert_panel_open: true,
        action_menu_open: true,
        zoom_menu_open: true,
        ..WorkflowState::default()
    };

    state
        .insert_node_immediately("answer", Point::new(30.0, 40.0))
        .expect("answer node should insert");

    assert_eq!(state.document.nodes.len(), 1);
    let node = &state.document.nodes[0];
    assert!(node.id.starts_with("answer-node-"));
    assert_eq!(node.position, Point::new(30.0, 40.0));
    assert_eq!(node.z_index, 1.0);
    assert_eq!(state.selected_node_id.as_deref(), Some(node.id.as_str()));
    assert!(node.selected);
    assert!(state.selected_edge_id.is_none());
    assert!(state.connection_draft.is_none());
    assert!(state.context_menu.is_none());
    assert!(!state.quick_insert_panel_open);
    assert!(!state.action_menu_open);
    assert!(!state.zoom_menu_open);
    assert_eq!(state.status_message.as_deref(), Some("已插入 回复 节点"));
}

#[test]
fn insert_node_immediately_rejects_second_start_node() {
    let start = workflow_node("start_1", "start", Point::ORIGIN, 0.0);
    let mut state = workflow_state_with_nodes(vec![start]);

    assert_eq!(
        state.insert_node_immediately("start", Point::new(30.0, 40.0)),
        Err("开始节点只能有一个".to_string())
    );
    assert_eq!(state.document.nodes.len(), 1);
}

#[test]
fn create_context_node_auto_connects_from_node_insert_menu() {
    let start = workflow_node("start_1", "start", Point::new(10.0, 20.0), 0.0);
    let mut state = workflow_state_with_nodes(vec![start]);
    state.context_menu = Some(WorkflowCanvasContextMenu {
        target: WorkflowCanvasContextMenuTarget::NodeInsert("start_1".to_string()),
        anchor: Point::new(2.0, 3.0),
        world: Point::new(90.0, 100.0),
    });

    state.create_context_node("answer").expect("context node should insert");

    assert_eq!(state.document.nodes.len(), 2);
    assert_eq!(state.document.edges.len(), 1);
    let inserted = state
        .document
        .nodes
        .iter()
        .find(|node| node.id != "start_1")
        .expect("inserted node should exist");
    assert_eq!(
        inserted.position,
        Point::new(10.0 + state.document.nodes[0].size.width + 120.0, 38.0)
    );
    let edge = &state.document.edges[0];
    assert_eq!(edge.source, "start_1");
    assert_eq!(edge.target, inserted.id);
    assert_eq!(edge.source_handle.as_deref(), Some("source"));
    assert_eq!(edge.target_handle.as_deref(), Some("target"));
    assert_eq!(state.selected_node_id.as_deref(), Some(inserted.id.as_str()));
    assert_eq!(state.status_message.as_deref(), Some("已新增下游 回复 节点并自动关联"));
}

#[test]
fn create_context_node_from_canvas_inserts_without_auto_connect() {
    let mut state = WorkflowState::default();
    state.context_menu = Some(WorkflowCanvasContextMenu {
        target: WorkflowCanvasContextMenuTarget::Canvas,
        anchor: Point::new(2.0, 3.0),
        world: Point::new(90.0, 100.0),
    });

    state.create_context_node("answer").expect("context node should insert");

    assert_eq!(state.document.nodes.len(), 1);
    assert!(state.document.edges.is_empty());
    assert_eq!(state.document.nodes[0].position, Point::new(90.0, 100.0));
    assert_eq!(state.status_message.as_deref(), Some("已插入 回复 节点"));
}

#[test]
fn create_context_node_keeps_inserted_node_selected_when_auto_connect_fails() {
    let mut source = workflow_node("answer_1", "answer", Point::new(10.0, 20.0), 0.0);
    source.source_handles.clear();
    let mut state = workflow_state_with_nodes(vec![source]);
    state.context_menu = Some(WorkflowCanvasContextMenu {
        target: WorkflowCanvasContextMenuTarget::NodeInsert("answer_1".to_string()),
        anchor: Point::new(2.0, 3.0),
        world: Point::new(90.0, 100.0),
    });

    state.create_context_node("answer").expect("context node should insert");

    assert_eq!(state.document.nodes.len(), 2);
    assert!(state.document.edges.is_empty());
    let inserted = state
        .document
        .nodes
        .iter()
        .find(|node| node.id != "answer_1")
        .expect("inserted node should exist");
    assert_eq!(state.selected_node_id.as_deref(), Some(inserted.id.as_str()));
    assert_eq!(
        state.status_message.as_deref(),
        Some("已新增 回复 节点，但自动关联失败：源节点没有可用输出句柄")
    );
}

#[test]
fn duplicate_selected_node_clones_offset_and_rejects_start_node() {
    let start = workflow_node("start_1", "start", Point::ORIGIN, 0.0);
    let answer = workflow_node("answer_1", "answer", Point::new(11.0, 22.0), 5.0);
    let mut state = workflow_state_with_nodes(vec![start, answer]);

    state.selected_node_id = Some("start_1".to_string());
    assert_eq!(
        state.duplicate_selected_node(),
        Err("开始节点只能有一个，不能复制开始节点".to_string())
    );

    state.selected_node_id = Some("answer_1".to_string());
    state.duplicate_selected_node().expect("answer node should duplicate");

    assert_eq!(state.document.nodes.len(), 3);
    let duplicated = state
        .document
        .nodes
        .iter()
        .find(|node| node.id != "start_1" && node.id != "answer_1")
        .expect("duplicated node should exist");
    assert!(duplicated.id.starts_with("answer-node-"));
    assert_eq!(duplicated.position, Point::new(47.0, 58.0));
    assert_eq!(duplicated.z_index, 6.0);
    assert!(duplicated.selected);
    assert_eq!(state.selected_node_id.as_deref(), Some(duplicated.id.as_str()));
    assert_eq!(state.status_message.as_deref(), Some("已复制节点 回复"));
}

#[test]
fn duplicate_selected_node_requires_existing_selection() {
    let mut state = WorkflowState::default();

    assert_eq!(state.duplicate_selected_node(), Err("请先选择一个节点".to_string()));

    state.selected_node_id = Some("missing".to_string());
    assert_eq!(state.duplicate_selected_node(), Err("目标节点不存在".to_string()));
}

#[test]
fn open_downstream_node_picker_retargets_context_menu_and_selection() {
    let answer = workflow_node("answer_1", "answer", Point::new(15.0, 25.0), 0.0);
    let expected_world = Point::new(answer.position.x + answer.size.width + 120.0, 43.0);
    let mut state = workflow_state_with_nodes(vec![answer]);
    state.context_menu = Some(WorkflowCanvasContextMenu {
        target: WorkflowCanvasContextMenuTarget::Node("answer_1".to_string()),
        anchor: Point::new(7.0, 8.0),
        world: Point::new(1.0, 2.0),
    });

    state.open_downstream_node_picker("answer_1").expect("downstream picker should open");

    let menu = state.context_menu.as_ref().expect("context menu should exist");
    assert_eq!(menu.target, WorkflowCanvasContextMenuTarget::NodeInsert("answer_1".to_string()));
    assert_eq!(menu.anchor, Point::new(7.0, 8.0));
    assert_eq!(menu.world, expected_world);
    assert_eq!(state.selected_node_id.as_deref(), Some("answer_1"));
    assert!(state.document.nodes[0].selected);
}

#[test]
fn open_downstream_node_picker_requires_menu_and_node() {
    let mut state = WorkflowState::default();

    assert_eq!(state.open_downstream_node_picker("missing"), Err("右键菜单已关闭".to_string()));

    state.context_menu = Some(WorkflowCanvasContextMenu {
        target: WorkflowCanvasContextMenuTarget::Canvas,
        anchor: Point::ORIGIN,
        world: Point::ORIGIN,
    });
    assert_eq!(state.open_downstream_node_picker("missing"), Err("目标节点不存在".to_string()));
}

#[test]
fn insert_downstream_node_creates_node_and_default_edge() {
    let start = workflow_node("start_1", "start", Point::new(10.0, 20.0), 3.0);
    let mut state = workflow_state_with_nodes(vec![start]);
    state.context_menu = Some(WorkflowCanvasContextMenu {
        target: WorkflowCanvasContextMenuTarget::NodeInsert("start_1".to_string()),
        anchor: Point::ORIGIN,
        world: Point::ORIGIN,
    });
    state.quick_insert_panel_open = true;

    state.insert_downstream_node("start_1", "answer").expect("downstream node should insert");

    assert_eq!(state.document.nodes.len(), 2);
    assert_eq!(state.document.edges.len(), 1);
    let inserted = state
        .document
        .nodes
        .iter()
        .find(|node| node.id != "start_1")
        .expect("inserted node should exist");
    assert_eq!(inserted.position, Point::new(370.0, 38.0));
    assert_eq!(inserted.z_index, 4.0);
    assert_eq!(state.selected_node_id.as_deref(), Some("start_1"));
    assert!(state.context_menu.is_none());
    assert!(!state.quick_insert_panel_open);
    assert_eq!(state.status_message.as_deref(), Some("已在 开始 后新增 回复 节点"));
}

#[test]
fn insert_downstream_node_clears_connection_draft_and_all_floating_menus() {
    let start = workflow_node("start_1", "start", Point::new(10.0, 20.0), 3.0);
    let mut state = workflow_state_with_nodes(vec![start]);
    state.selected_edge_id = Some("edge_1".to_string());
    state.connection_draft = Some(WorkflowConnectionDraft {
        from: WorkflowConnectionEndpoint {
            node_id: "start_1".to_string(),
            handle_id: "source".to_string(),
            kind: WorkflowHandleKind::Source,
        },
        cursor_world: Point::new(20.0, 30.0),
    });
    state.context_menu = Some(WorkflowCanvasContextMenu {
        target: WorkflowCanvasContextMenuTarget::NodeInsert("start_1".to_string()),
        anchor: Point::ORIGIN,
        world: Point::ORIGIN,
    });
    state.quick_insert_panel_open = true;
    state.action_menu_open = true;
    state.zoom_menu_open = true;

    state.insert_downstream_node("start_1", "answer").expect("downstream node should insert");

    assert_eq!(state.document.nodes.len(), 2);
    assert_eq!(state.document.edges.len(), 1);
    assert_eq!(state.selected_node_id.as_deref(), Some("start_1"));
    assert!(state.selected_edge_id.is_none());
    assert!(state.connection_draft.is_none());
    assert!(state.context_menu.is_none());
    assert!(!state.quick_insert_panel_open);
    assert!(!state.action_menu_open);
    assert!(!state.zoom_menu_open);
}

#[test]
fn insert_downstream_node_from_handle_uses_synthetic_fail_branch_from_editor() {
    let mut code = workflow_node("code_1", "code", Point::new(0.0, 0.0), 0.0);
    code.source_handles.clear();
    let mut state = workflow_state_with_nodes(vec![code]);
    state.node_editor = Some(code_editor_with_error_strategy("code_1", "fail-branch"));

    state
        .insert_downstream_node_from_handle("code_1", "fail-branch", "answer")
        .expect("synthetic fail branch should connect");

    assert_eq!(state.document.edges.len(), 1);
    assert_eq!(state.document.edges[0].source_handle.as_deref(), Some("fail-branch"));
}

#[test]
fn insert_downstream_node_from_handle_rejects_synthetic_fail_branch_when_editor_mismatch() {
    let mut code = workflow_node("code_1", "code", Point::new(0.0, 0.0), 0.0);
    code.source_handles.clear();
    let mut state = workflow_state_with_nodes(vec![code]);
    state.node_editor = Some(code_editor_with_error_strategy("other_code", "fail-branch"));

    state
        .insert_downstream_node_from_handle("code_1", "fail-branch", "answer")
        .expect("node insert should survive connection failure");

    assert_eq!(state.document.nodes.len(), 2);
    assert!(state.document.edges.is_empty());
    assert_eq!(
        state.status_message.as_deref(),
        Some("已新增 回复 节点，但自动关联失败：源节点不存在输出句柄 fail-branch")
    );
}

#[test]
fn insert_downstream_node_from_handle_uses_existing_explicit_source_handle() {
    let mut start = workflow_node("start_1", "start", Point::new(0.0, 0.0), 0.0);
    start.source_handles.push(WorkflowHandle {
        id: "custom".to_string(),
        label: "自定义".to_string(),
        kind: WorkflowHandleKind::Source,
    });
    let mut state = workflow_state_with_nodes(vec![start]);

    state
        .insert_downstream_node_from_handle("start_1", "custom", "answer")
        .expect("explicit source handle should connect");

    assert_eq!(state.document.edges.len(), 1);
    assert_eq!(state.document.edges[0].source_handle.as_deref(), Some("custom"));
    assert_eq!(state.selected_node_id.as_deref(), Some("start_1"));
}

#[test]
fn insert_downstream_node_reports_insert_success_when_auto_connect_fails() {
    let mut source = workflow_node("answer_1", "answer", Point::ORIGIN, 0.0);
    source.source_handles.clear();
    let mut state = workflow_state_with_nodes(vec![source]);

    state
        .insert_downstream_node("answer_1", "answer")
        .expect("node insert should survive connection failure");

    assert_eq!(state.document.nodes.len(), 2);
    assert!(state.document.edges.is_empty());
    assert_eq!(
        state.status_message.as_deref(),
        Some("已新增 回复 节点，但自动关联失败：源节点没有可用输出句柄")
    );
}

#[test]
fn insert_downstream_node_rejects_missing_source_and_second_start() {
    let start = workflow_node("start_1", "start", Point::ORIGIN, 0.0);
    let mut state = workflow_state_with_nodes(vec![start]);

    assert_eq!(
        state.insert_downstream_node("missing", "answer"),
        Err("目标节点不存在".to_string())
    );
    assert_eq!(
        state.insert_downstream_node("start_1", "start"),
        Err("开始节点只能有一个".to_string())
    );
}

#[test]
fn connect_nodes_with_source_handle_rejects_invalid_paths_and_duplicates() {
    let start = workflow_node("start_1", "start", Point::ORIGIN, 0.0);
    let answer = workflow_node("answer_1", "answer", Point::new(300.0, 0.0), 0.0);
    let mut state = workflow_state_with_nodes(vec![start, answer]);

    assert_eq!(
        state.connect_nodes_by_default_handles("start_1", "start_1"),
        Err("暂不支持节点自身回环连线".to_string())
    );
    assert_eq!(
        state.connect_nodes_by_default_handles("missing", "answer_1"),
        Err("源节点不存在，无法自动连线".to_string())
    );
    assert_eq!(
        state.connect_nodes_by_default_handles("start_1", "missing"),
        Err("目标节点不存在，无法自动连线".to_string())
    );
    assert_eq!(
        state.connect_nodes_with_source_handle("start_1", Some("missing"), "answer_1"),
        Err("源节点不存在输出句柄 missing".to_string())
    );

    state
        .connect_nodes_by_default_handles("start_1", "answer_1")
        .expect("default handles should connect");
    assert_eq!(state.document.edges.len(), 1);
    assert_eq!(
        state.connect_nodes_by_default_handles("start_1", "answer_1"),
        Err("这条连线已经存在".to_string())
    );
}

#[test]
fn connect_nodes_with_source_handle_rejects_missing_handles() {
    let mut start = workflow_node("start_1", "start", Point::ORIGIN, 0.0);
    start.source_handles.clear();
    let mut answer = workflow_node("answer_1", "answer", Point::new(300.0, 0.0), 0.0);
    answer.target_handles.clear();
    let mut state = workflow_state_with_nodes(vec![start, answer]);

    assert_eq!(
        state.connect_nodes_by_default_handles("start_1", "answer_1"),
        Err("源节点没有可用输出句柄".to_string())
    );

    state.document.nodes[0].source_handles.push(WorkflowHandle {
        id: "source".to_string(),
        label: "输出".to_string(),
        kind: WorkflowHandleKind::Source,
    });
    assert_eq!(
        state.connect_nodes_by_default_handles("start_1", "answer_1"),
        Err("目标节点没有可用输入句柄".to_string())
    );
}

#[test]
fn connect_nodes_with_source_handle_adds_edge_with_next_z_index() {
    let start = workflow_node("start_1", "start", Point::ORIGIN, 0.0);
    let answer = workflow_node("answer_1", "answer", Point::new(300.0, 0.0), 0.0);
    let mut state = workflow_state_with_nodes(vec![start, answer]);
    state.document.edges =
        vec![workflow_edge("existing", "old_source", "old_target", Some("source"), Some("target"))];
    state.document.edges[0].z_index = 4.0;

    state
        .connect_nodes_by_default_handles("start_1", "answer_1")
        .expect("default handles should connect");

    let edge = state
        .document
        .edges
        .iter()
        .find(|edge| edge.id != "existing")
        .expect("new edge should exist");
    assert_eq!(edge.source, "start_1");
    assert_eq!(edge.target, "answer_1");
    assert_eq!(edge.source_type, "start");
    assert_eq!(edge.target_type, "answer");
    assert_eq!(edge.z_index, 5.0);
}

#[test]
fn prune_invalid_edges_for_node_handles_removes_bad_edges_and_clears_ui_refs() {
    let start = workflow_node("start_1", "start", Point::ORIGIN, 0.0);
    let answer = workflow_node("answer_1", "answer", Point::new(300.0, 0.0), 0.0);
    let mut state = workflow_state_with_nodes(vec![start, answer]);
    state.document.edges = vec![
        workflow_edge("valid", "start_1", "answer_1", Some("source"), Some("target")),
        workflow_edge("missing_source", "start_1", "answer_1", Some("gone"), Some("target")),
        workflow_edge("missing_target", "start_1", "answer_1", Some("source"), None),
    ];
    state.selected_edge_id = Some("missing_source".to_string());
    state.context_menu = Some(WorkflowCanvasContextMenu {
        target: WorkflowCanvasContextMenuTarget::Edge("missing_target".to_string()),
        anchor: Point::ORIGIN,
        world: Point::ORIGIN,
    });

    let removed = state.prune_invalid_edges_for_node_handles("start_1");

    assert_eq!(removed, 1);
    assert_eq!(state.document.edges.len(), 2);
    assert!(state.document.edges.iter().any(|edge| edge.id == "valid"));
    assert!(state.document.edges.iter().any(|edge| edge.id == "missing_target"));
    assert!(state.selected_edge_id.is_none());
    assert!(state.context_menu.is_some());

    let removed = state.prune_invalid_edges_for_node_handles("answer_1");

    assert_eq!(removed, 1);
    assert_eq!(state.document.edges.len(), 1);
    assert_eq!(state.document.edges[0].id, "valid");
    assert!(state.context_menu.is_none());
}

#[test]
fn prune_invalid_edges_for_node_handles_removes_missing_source_handle() {
    let start = workflow_node("start_1", "start", Point::ORIGIN, 0.0);
    let answer = workflow_node("answer_1", "answer", Point::new(300.0, 0.0), 0.0);
    let mut state = workflow_state_with_nodes(vec![start, answer]);
    state.document.edges = vec![
        workflow_edge("missing_source_handle", "start_1", "answer_1", None, Some("target")),
        workflow_edge("unrelated", "other", "answer_1", None, Some("target")),
    ];

    let removed = state.prune_invalid_edges_for_node_handles("start_1");

    assert_eq!(removed, 1);
    assert_eq!(state.document.edges.len(), 1);
    assert_eq!(state.document.edges[0].id, "unrelated");
}

#[test]
fn prune_invalid_edges_for_node_handles_noops_for_missing_or_valid_node() {
    let start = workflow_node("start_1", "start", Point::ORIGIN, 0.0);
    let answer = workflow_node("answer_1", "answer", Point::new(300.0, 0.0), 0.0);
    let mut state = workflow_state_with_nodes(vec![start, answer]);
    state.document.edges =
        vec![workflow_edge("valid", "start_1", "answer_1", Some("source"), Some("target"))];

    assert_eq!(state.prune_invalid_edges_for_node_handles("missing"), 0);
    assert_eq!(state.prune_invalid_edges_for_node_handles("start_1"), 0);
    assert_eq!(state.document.edges.len(), 1);
}
