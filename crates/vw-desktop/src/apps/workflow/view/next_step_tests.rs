//! 工作流下一步视图测试模块，覆盖下一步区域、连接列表、按钮和失败分支能力推导行为。

use super::*;
use iced::{Point, Size, widget::text_editor};
use serde_yaml::Value;

use crate::apps::workflow::model::{
    WorkflowDocument, WorkflowHandle, WorkflowHandleKind, WorkflowHandleSide,
};
use crate::apps::workflow::state::{WorkflowNodeEditorDraft, WorkflowNodeEditorValidation};

fn workflow_node(id: &str, block_type: &str, source_handles: Vec<WorkflowHandle>) -> WorkflowNode {
    WorkflowNode {
        id: id.to_string(),
        block_type: block_type.to_string(),
        title: "代码节点".to_string(),
        description: String::new(),
        position: Point::new(0.0, 0.0),
        size: Size::new(180.0, 120.0),
        parent_id: None,
        selected: false,
        source_side: WorkflowHandleSide::Right,
        target_side: WorkflowHandleSide::Left,
        source_handles,
        target_handles: Vec::new(),
        z_index: 0.0,
        raw_node: Value::Null,
    }
}

fn workflow_edge(
    id: &str,
    source: &str,
    target: &str,
    source_handle: Option<&str>,
) -> WorkflowEdge {
    WorkflowEdge {
        id: id.to_string(),
        source: source.to_string(),
        target: target.to_string(),
        source_handle: source_handle.map(str::to_string),
        target_handle: Some("target".to_string()),
        source_type: "code".to_string(),
        target_type: "answer".to_string(),
        selected: false,
        z_index: 0.0,
        raw_edge: Value::Null,
    }
}

fn workflow_state_with_document(document: WorkflowDocument) -> WorkflowState {
    WorkflowState { document, ..WorkflowState::default() }
}

fn code_editor_with_error_strategy(node_id: &str, error_strategy: &str) -> WorkflowNodeEditorDraft {
    WorkflowNodeEditorDraft {
        mode: WorkflowNodeEditorMode::Edit(node_id.to_string()),
        active_tab: WorkflowNodeEditorTab::Basic,
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

fn create_editor(block_type: &str) -> WorkflowNodeEditorDraft {
    WorkflowNodeEditorDraft {
        mode: WorkflowNodeEditorMode::Create,
        active_tab: WorkflowNodeEditorTab::Basic,
        block_type: block_type.to_string(),
        title: "新节点".to_string(),
        description: String::new(),
        description_editor: text_editor::Content::with_text(""),
        position: Point::new(0.0, 0.0),
        visual_draft: None,
        validation: WorkflowNodeEditorValidation::default(),
        show_raw_data_editor: false,
        raw_data_editor: text_editor::Content::with_text("{}"),
        hovered_start_variable_index: None,
        start_variable_focus_index: None,
        start_variable_editor: None,
    }
}

#[test]
fn build_node_next_step_existing_item_handles_empty_and_custom_description() {
    let edge = workflow_edge("edge_1", "code_1", "answer_1", None);
    let empty_description = workflow_node("answer_1", "answer", Vec::new());
    let described_node = WorkflowNode {
        description: "  生成最终回复  ".to_string(),
        ..workflow_node("answer_2", "answer", Vec::new())
    };

    let empty_description_element = build_node_next_step_existing_item(
        crate::app::i18n::I18n::new(Default::default()),
        &edge,
        &empty_description,
    );
    let described_element = build_node_next_step_existing_item(
        crate::app::i18n::I18n::new(Default::default()),
        &edge,
        &described_node,
    );

    let _ = std::hint::black_box((empty_description_element, described_element));
}

#[test]
fn build_node_next_step_section_handles_create_mode() {
    let state = WorkflowState::default();
    let editor = create_editor("answer");

    let element = build_node_next_step_section(
        crate::app::i18n::I18n::new(Default::default()),
        &state,
        &editor,
    );

    let _ = std::hint::black_box(element);
}

#[test]
fn build_node_next_step_section_handles_edit_mode_without_fail_branch() {
    let state = workflow_state_with_document(WorkflowDocument {
        nodes: vec![
            workflow_node("code_1", "code", Vec::new()),
            workflow_node("answer_1", "answer", Vec::new()),
        ],
        edges: vec![workflow_edge("edge_1", "code_1", "answer_1", None)],
        ..WorkflowDocument::default()
    });
    let editor = code_editor_with_error_strategy("code_1", "none");

    let element = build_node_next_step_section(
        crate::app::i18n::I18n::new(Default::default()),
        &state,
        &editor,
    );

    let _ = std::hint::black_box(element);
}

#[test]
fn build_node_next_step_section_handles_saved_fail_branch() {
    let state = workflow_state_with_document(WorkflowDocument {
        nodes: vec![
            workflow_node(
                "code_1",
                "code",
                vec![WorkflowHandle {
                    id: "fail-branch".to_string(),
                    label: "异常".to_string(),
                    kind: WorkflowHandleKind::Source,
                }],
            ),
            workflow_node("answer_1", "answer", Vec::new()),
        ],
        edges: vec![workflow_edge("edge_1", "code_1", "answer_1", Some("fail-branch"))],
        ..WorkflowDocument::default()
    });
    let editor = code_editor_with_error_strategy("code_1", "none");

    let element = build_node_next_step_section(
        crate::app::i18n::I18n::new(Default::default()),
        &state,
        &editor,
    );

    let _ = std::hint::black_box(element);
}

#[test]
fn build_node_next_step_section_handles_draft_fail_branch() {
    let state = workflow_state_with_document(WorkflowDocument {
        nodes: vec![workflow_node("code_1", "code", Vec::new())],
        ..WorkflowDocument::default()
    });
    let editor = code_editor_with_error_strategy("code_1", "fail-branch");

    let element = build_node_next_step_section(
        crate::app::i18n::I18n::new(Default::default()),
        &state,
        &editor,
    );

    let _ = std::hint::black_box(element);
}

#[test]
fn node_next_step_supports_fail_branch_from_saved_handle_or_matching_draft_only() {
    let saved_handle_state = workflow_state_with_document(WorkflowDocument {
        nodes: vec![workflow_node(
            "code_1",
            "code",
            vec![WorkflowHandle {
                id: "fail-branch".to_string(),
                label: "异常".to_string(),
                kind: WorkflowHandleKind::Source,
            }],
        )],
        ..WorkflowDocument::default()
    });
    let no_handle_state = workflow_state_with_document(WorkflowDocument {
        nodes: vec![workflow_node("code_1", "code", Vec::new())],
        ..WorkflowDocument::default()
    });
    let fail_branch_editor = code_editor_with_error_strategy("code_1", "fail-branch");
    let wrong_node_editor = code_editor_with_error_strategy("other", "fail-branch");
    let create_editor = create_editor("code");

    assert!(node_next_step_supports_fail_branch(
        &saved_handle_state,
        &code_editor_with_error_strategy("code_1", "none"),
        "code_1",
    ));
    assert!(node_next_step_supports_fail_branch(&no_handle_state, &fail_branch_editor, "code_1",));
    assert!(!node_next_step_supports_fail_branch(&no_handle_state, &wrong_node_editor, "code_1",));
    assert!(!node_next_step_supports_fail_branch(&no_handle_state, &create_editor, "code_1",));
}

#[test]
fn build_node_next_step_connection_list_handles_empty_and_dangling_edges() {
    let state = workflow_state_with_document(WorkflowDocument {
        nodes: vec![workflow_node("code_1", "code", Vec::new())],
        edges: vec![workflow_edge("edge_1", "code_1", "missing", None)],
        ..WorkflowDocument::default()
    });

    let default_list = build_node_next_step_connection_list(
        crate::app::i18n::I18n::new(Default::default()),
        &state,
        "code_1",
        None,
        "空",
    );
    let fail_branch_list = build_node_next_step_connection_list(
        crate::app::i18n::I18n::new(Default::default()),
        &state,
        "code_1",
        Some("fail-branch"),
        "空",
    );

    let _ = std::hint::black_box((default_list, fail_branch_list));
}

#[test]
fn build_node_next_step_connection_list_filters_default_and_fail_branch_edges() {
    let state = workflow_state_with_document(WorkflowDocument {
        nodes: vec![
            workflow_node("code_1", "code", Vec::new()),
            workflow_node("answer_1", "answer", Vec::new()),
            workflow_node("answer_2", "answer", Vec::new()),
        ],
        edges: vec![
            workflow_edge("edge_1", "code_1", "answer_1", None),
            workflow_edge("edge_2", "code_1", "answer_2", Some("fail-branch")),
        ],
        ..WorkflowDocument::default()
    });

    let default_list = build_node_next_step_connection_list(
        crate::app::i18n::I18n::new(Default::default()),
        &state,
        "code_1",
        None,
        "空",
    );
    let fail_branch_list = build_node_next_step_connection_list(
        crate::app::i18n::I18n::new(Default::default()),
        &state,
        "code_1",
        Some("fail-branch"),
        "空",
    );

    let _ = std::hint::black_box((default_list, fail_branch_list));
}

#[test]
fn build_node_next_step_branch_section_builds_card_content() {
    let existing_content: Element<'_, Message> = container(text("已有")).into();
    let add_content: Element<'_, Message> = container(text("添加")).into();

    let element = build_node_next_step_branch_section(
        "异常时",
        "代码节点进入异常分支后，会从这里继续流转。",
        existing_content,
        add_content,
    );

    let _ = std::hint::black_box(element);
}

#[test]
fn build_start_next_step_button_group_builds_wrapped_buttons() {
    let state = WorkflowState::default();

    let default_group = build_start_next_step_button_group(&state, "code_1".to_string(), None);
    let handle_group =
        build_start_next_step_button_group(&state, "code_1".to_string(), Some("fail-branch"));

    let _ = std::hint::black_box((default_group, handle_group));
}

#[test]
fn build_start_next_step_button_uses_default_and_handle_messages() {
    let node_type = supported_node_types()
        .iter()
        .copied()
        .find(|node_type| node_type.block_type == "answer")
        .expect("answer node type should exist");

    let default_button = build_start_next_step_button("code_1".to_string(), None, node_type);
    let handle_button =
        build_start_next_step_button("code_1".to_string(), Some("fail-branch"), node_type);

    let _ = std::hint::black_box((default_button, handle_button));
}

#[test]
fn next_step_supports_fail_branch_from_saved_node_handles() {
    let state = WorkflowState {
        document: WorkflowDocument {
            nodes: vec![workflow_node(
                "code_1",
                "code",
                vec![WorkflowHandle {
                    id: "fail-branch".to_string(),
                    label: "异常".to_string(),
                    kind: WorkflowHandleKind::Source,
                }],
            )],
            ..WorkflowDocument::default()
        },
        ..WorkflowState::default()
    };
    let editor = code_editor_with_error_strategy("code_1", "none");

    assert!(node_next_step_supports_fail_branch(&state, &editor, "code_1"));
}

#[test]
fn next_step_does_not_support_fail_branch_when_handle_and_draft_do_not_match() {
    let state = workflow_state_with_document(WorkflowDocument {
        nodes: vec![workflow_node("code_1", "code", Vec::new())],
        ..WorkflowDocument::default()
    });
    let editor = code_editor_with_error_strategy("code_1", "none");
    let wrong_node_editor = code_editor_with_error_strategy("other", "fail-branch");
    let create_mode_editor = create_editor("code");

    assert!(!node_next_step_supports_fail_branch(&state, &editor, "code_1"));
    assert!(!node_next_step_supports_fail_branch(&state, &wrong_node_editor, "code_1"));
    assert!(!node_next_step_supports_fail_branch(&state, &create_mode_editor, "code_1"));
}

#[test]
fn next_step_supports_fail_branch_from_unsaved_code_editor_draft() {
    let state = WorkflowState {
        document: WorkflowDocument {
            nodes: vec![workflow_node("code_1", "code", Vec::new())],
            ..WorkflowDocument::default()
        },
        ..WorkflowState::default()
    };
    let editor = code_editor_with_error_strategy("code_1", "fail-branch");

    assert!(node_next_step_supports_fail_branch(&state, &editor, "code_1"));
}
