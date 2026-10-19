//! 工作流节点编辑器视图测试模块，覆盖弹窗、标签页和编辑区构建路径。

use super::*;
use iced::{Point, widget::text_editor};

fn editor_with_tab(
    active_tab: WorkflowNodeEditorTab,
    visual_draft: Option<WorkflowNodeVisualDraft>,
) -> state::WorkflowNodeEditorDraft {
    state::WorkflowNodeEditorDraft {
        mode: WorkflowNodeEditorMode::Create,
        active_tab,
        block_type: "answer".to_string(),
        title: "回复节点".to_string(),
        description: "返回最终答案".to_string(),
        description_editor: text_editor::Content::with_text("返回最终答案"),
        position: Point::new(16.0, 24.0),
        visual_draft,
        validation: state::WorkflowNodeEditorValidation::default(),
        show_raw_data_editor: false,
        raw_data_editor: text_editor::Content::with_text("answer: hello"),
        hovered_start_variable_index: None,
        start_variable_focus_index: None,
        start_variable_editor: None,
    }
}

fn answer_visual() -> WorkflowNodeVisualDraft {
    WorkflowNodeVisualDraft::Answer { answer_editor: text_editor::Content::with_text("hello") }
}

fn state_with_editor(editor: state::WorkflowNodeEditorDraft) -> WorkflowState {
    WorkflowState { node_editor: Some(editor), ..WorkflowState::default() }
}

#[test]
fn modal_without_editor_builds_empty_placeholder() {
    let state = WorkflowState::default();

    let _element = build_node_editor_modal(&state);
}

#[test]
fn modal_builds_visual_tab_with_answer_editor() {
    let state =
        state_with_editor(editor_with_tab(WorkflowNodeEditorTab::Visual, Some(answer_visual())));

    let _element = build_node_editor_modal(&state);
}

#[test]
fn node_editor_drawer_uses_two_fifths_of_the_workflow_width() {
    assert_eq!(NODE_EDITOR_DRAWER_PORTION * 3, NODE_EDITOR_CANVAS_PORTION * 2);
}

#[test]
fn modal_falls_back_from_missing_visual_tab_to_description() {
    let state = state_with_editor(editor_with_tab(WorkflowNodeEditorTab::Visual, None));

    let _element = build_node_editor_modal(&state);
}

#[test]
fn modal_builds_description_basic_and_advanced_tabs() {
    for tab in [
        WorkflowNodeEditorTab::Description,
        WorkflowNodeEditorTab::Basic,
        WorkflowNodeEditorTab::AdvancedDsl,
    ] {
        let state = state_with_editor(editor_with_tab(tab, Some(answer_visual())));

        let _element = build_node_editor_modal(&state);
    }
}

#[test]
fn modal_without_visual_draft_keeps_non_visual_tabs_active() {
    for tab in [
        WorkflowNodeEditorTab::Description,
        WorkflowNodeEditorTab::Basic,
        WorkflowNodeEditorTab::AdvancedDsl,
    ] {
        let state = state_with_editor(editor_with_tab(tab, None));

        let _element = build_node_editor_modal(&state);
    }
}

#[test]
fn modal_builds_validation_summary_when_editor_has_errors() {
    let mut editor = editor_with_tab(WorkflowNodeEditorTab::AdvancedDsl, Some(answer_visual()));
    editor.validation.field_errors.push(state::WorkflowNodeValidationError {
        path: "answer.text".to_string(),
        message: "回复内容不能为空".to_string(),
    });
    let state = state_with_editor(editor);

    let _element = build_node_editor_modal(&state);
}

#[test]
fn node_editor_parts_build_individually() {
    let editor = editor_with_tab(WorkflowNodeEditorTab::Description, Some(answer_visual()));
    let content = text_editor::Content::with_text("正文");

    let _embedded = build_embedded_text_editor(
        &content,
        "占位",
        WorkflowMessage::NodeEditorDescriptionAction,
        88.0,
    );
    let _header = build_node_editor_header(&editor);
    let _tabs_with_visual = build_node_editor_tabs(true, WorkflowNodeEditorTab::Visual);
    let _tabs_without_visual = build_node_editor_tabs(false, WorkflowNodeEditorTab::Basic);
    let _active_tab = build_node_editor_tab_button(
        "配置",
        WorkflowNodeEditorTab::Visual,
        WorkflowNodeEditorTab::Visual,
    );
    let _inactive_tab = build_node_editor_tab_button(
        "描述",
        WorkflowNodeEditorTab::Description,
        WorkflowNodeEditorTab::Visual,
    );
    let _description = build_node_description_section(&editor);
    let _hint = build_node_connection_hint();
    let _advanced = build_node_advanced_dsl_section(&editor);
    let _placeholder = build_node_visual_placeholder();
}

#[test]
fn embedded_text_editor_preserves_zero_height_boundary() {
    let content = text_editor::Content::with_text("");

    let element = build_embedded_text_editor(
        &content,
        "空内容",
        WorkflowMessage::NodeEditorDescriptionAction,
        0.0,
    );

    assert_eq!(element.as_widget().size().height, Length::Fixed(0.0));
}

#[test]
fn start_variable_type_option_display_combines_label_and_value_type() {
    let option =
        StartVariableTypeOption { input_type: "text-input", label: "文本", value_type: "string" };

    assert_eq!(option.to_string(), "文本 · string");
}

#[test]
fn start_builtin_variable_item_fields_are_accessible() {
    let item = StartBuiltinVariableItem {
        name: "sys.query",
        value_type: "string",
        description: "用户输入",
        legacy: false,
    };

    assert_eq!(item.name, "sys.query");
    assert_eq!(item.value_type, "string");
    assert_eq!(item.description, "用户输入");
    assert!(!item.legacy);
}

#[test]
fn start_variable_type_option_supports_equality_and_copy() {
    let option =
        StartVariableTypeOption {
            input_type: "paragraph", label: "长文本", value_type: "string"
        };
    let copied = option;

    assert_eq!(copied, option);
    assert_eq!(copied.input_type, "paragraph");
}

#[test]
fn start_builtin_variable_item_supports_copy() {
    let item = StartBuiltinVariableItem {
        name: "sys.files",
        value_type: "array[file]",
        description: "用户上传文件",
        legacy: true,
    };
    let copied = item;

    assert_eq!(copied.name, "sys.files");
    assert_eq!(copied.value_type, "array[file]");
    assert_eq!(copied.description, "用户上传文件");
    assert!(copied.legacy);
}
