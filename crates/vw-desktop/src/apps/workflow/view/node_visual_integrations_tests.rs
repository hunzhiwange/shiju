//! 工作流集成节点视图测试模块，覆盖代码节点变量引用候选项的筛选与回退。

use super::*;
use iced::{Point, Size, Vector, widget::text_editor};
use serde_yaml::Value;

use crate::apps::workflow::model::{
    WorkflowAppMeta, WorkflowConversationVariable, WorkflowDocument, WorkflowEnvironmentVariable,
    WorkflowHandleSide, WorkflowViewport,
};
use crate::apps::workflow::state::{
    WorkflowAppEntry, WorkflowHistorySnapshot, WorkflowNodeEditorDraft,
    WorkflowNodeEditorValidation, WorkflowNodeValidationError,
};

fn workflow_node(id: &str, block_type: &str, title: &str, raw_yaml: &str) -> WorkflowNode {
    WorkflowNode {
        id: id.to_string(),
        block_type: block_type.to_string(),
        title: title.to_string(),
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
        raw_node: serde_yaml::from_str::<Value>(raw_yaml).expect("node yaml should parse"),
    }
}

fn minimal_code_editor(node_id: &str) -> WorkflowNodeEditorDraft {
    WorkflowNodeEditorDraft {
        mode: WorkflowNodeEditorMode::Edit(node_id.to_string()),
        active_tab: WorkflowNodeEditorTab::Visual,
        block_type: "code".to_string(),
        title: "代码节点".to_string(),
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

fn workflow_state_with_active_app(
    document: WorkflowDocument,
    environment_variables: Vec<WorkflowEnvironmentVariable>,
    conversation_variables: Vec<WorkflowConversationVariable>,
) -> WorkflowState {
    let meta = WorkflowAppMeta::default();
    let saved_snapshot = WorkflowHistorySnapshot {
        meta: meta.clone(),
        document: document.clone(),
        environment_variables: environment_variables.clone(),
        conversation_variables: conversation_variables.clone(),
        pan: Vector::new(0.0, 0.0),
        zoom: 1.0,
        selected_node_id: None,
        selected_edge_id: None,
    };

    WorkflowState {
        apps: vec![WorkflowAppEntry {
            id: "app_1".to_string(),
            local_uuid: None,
            meta,
            source_path: None,
            raw_root: Value::Null,
            document: document.clone(),
            environment_variables: environment_variables.clone(),
            conversation_variables: conversation_variables.clone(),
            pan: Vector::new(0.0, 0.0),
            zoom: 1.0,
            selected_node_id: None,
            selected_edge_id: None,
            connection_draft: None,
            is_dirty: false,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            saved_snapshot: saved_snapshot.clone(),
        }],
        active_app_id: Some("app_1".to_string()),
        saved_apps: Vec::new(),
        saved_apps_loading: false,
        saved_apps_loaded: false,
        saved_apps_error: None,
        opening_saved_app_uuid: None,
        deleting_saved_app_uuid: None,
        confirm_delete_saved_app_uuid: None,
        saved_app_actions_menu_uuid: None,
        saved_app_search_query: String::new(),
        copied_saved_app_uuid: None,
        saved_app_run_prompt: None,
        running_saved_app_uuid: None,
        running_saved_app_query: String::new(),
        running_saved_app_inputs: Default::default(),
        saved_app_run_preview: None,
        last_run_drafts: Default::default(),
        saved_app_run_detail: None,
        run_error_copied: false,
        active_is_dirty: false,
        app_editor: None,
        node_editor: None,
        node_editor_dsl_context_menu_open: false,
        node_editor_dsl_context_menu_pos: None,
        node_editor_dsl_undo_stack: Vec::new(),
        node_editor_dsl_redo_stack: Vec::new(),
        variable_panel: None,
        variable_editor: None,
        context_menu: None,
        quick_insert_panel_open: false,
        action_menu_open: false,
        zoom_menu_open: false,
        source_name: "测试工作流".to_string(),
        local_uuid: None,
        source_path: None,
        document,
        environment_variables,
        conversation_variables,
        pan: Vector::new(0.0, 0.0),
        zoom: 1.0,
        selected_node_id: None,
        selected_edge_id: None,
        connection_draft: None,
        undo_stack: Vec::new(),
        redo_stack: Vec::new(),
        saved_snapshot: Some(saved_snapshot),
        dragging_node_id: None,
        drag_pending_snapshot: None,
        status_message: None,
        error_message: None,
    }
}

fn validation_with_errors(paths: &[&str]) -> WorkflowNodeEditorValidation {
    WorkflowNodeEditorValidation {
        field_errors: paths
            .iter()
            .map(|path| WorkflowNodeValidationError {
                path: (*path).to_string(),
                message: format!("{path} is invalid"),
            })
            .collect(),
    }
}

fn empty_document() -> WorkflowDocument {
    WorkflowDocument {
        name: "空工作流".to_string(),
        nodes: Vec::new(),
        edges: Vec::new(),
        viewport: WorkflowViewport::default(),
    }
}

#[test]
fn selected_code_variable_reference_option_falls_back_to_selector_when_missing() {
    let input = WorkflowCodeVariableDraft {
        variable: String::new(),
        value_type: "number".to_string(),
        selector: vec!["start".to_string(), "limit".to_string()],
    };

    let selected = selected_code_variable_reference_option(&input, &[])
        .expect("missing selector should still build fallback option");

    assert_eq!(selected.selector_key, "start.limit");
    assert_eq!(selected.label, "start.limit");
    assert_eq!(selected.value_type, "number");
}

#[test]
fn selected_code_variable_reference_option_uses_matching_option_and_named_fallback() {
    let options = vec![WorkflowCodeVariableReferenceOption {
        selector_key: "start.limit".to_string(),
        label: "limit".to_string(),
        value_type: "number".to_string(),
    }];
    let input = WorkflowCodeVariableDraft {
        variable: "max_limit".to_string(),
        value_type: "string".to_string(),
        selector: vec!["start".to_string(), "limit".to_string()],
    };

    let selected = selected_code_variable_reference_option(&input, &options)
        .expect("matching selector should be selected");

    assert_eq!(selected, options[0]);

    let missing = WorkflowCodeVariableDraft {
        variable: "payload".to_string(),
        value_type: "object".to_string(),
        selector: vec!["tool".to_string(), "payload".to_string()],
    };
    let selected = selected_code_variable_reference_option(&missing, &options)
        .expect("missing selector should fall back to current input");

    assert_eq!(selected.selector_key, "tool.payload");
    assert_eq!(selected.label, "payload");
    assert_eq!(selected.value_type, "object");
}

#[test]
fn selected_code_variable_reference_option_returns_none_for_empty_selector() {
    let input = WorkflowCodeVariableDraft {
        variable: "query".to_string(),
        value_type: "string".to_string(),
        selector: Vec::new(),
    };

    assert!(selected_code_variable_reference_option(&input, &[]).is_none());
}

#[test]
fn code_picker_options_match_supported_values() {
    let languages = code_language_options();
    let outputs = code_output_type_options();
    let error_strategies = code_error_strategy_options();

    assert_eq!(code_picker_option("python3", &languages).expect("python3 option").label, "PYTHON3");
    assert!(code_picker_option("ruby", &languages).is_none());
    assert_eq!(
        outputs.iter().map(|option| option.value).collect::<Vec<_>>(),
        vec![
            "string",
            "number",
            "boolean",
            "array[number]",
            "array[string]",
            "array[boolean]",
            "array[object]",
            "object",
        ],
    );
    assert_eq!(
        error_strategies.iter().map(ToString::to_string).collect::<Vec<_>>(),
        vec!["无", "默认值", "异常分支"],
    );
}

#[test]
fn code_value_type_label_maps_known_types_and_defaults_to_string() {
    assert_eq!(code_value_type_label("number"), "Number");
    assert_eq!(code_value_type_label("boolean"), "Boolean");
    assert_eq!(code_value_type_label("object"), "Object");
    assert_eq!(code_value_type_label("array[number]"), "Array[Number]");
    assert_eq!(code_value_type_label("array[string]"), "Array[String]");
    assert_eq!(code_value_type_label("array[boolean]"), "Array[Boolean]");
    assert_eq!(code_value_type_label("array[object]"), "Array[Object]");
    assert_eq!(code_value_type_label("file"), "File");
    assert_eq!(code_value_type_label("array[file]"), "Array[File]");
    assert_eq!(code_value_type_label("unknown"), "String");
}

#[test]
fn code_variable_reference_options_include_upstream_and_context_sources_only() {
    let start_node = workflow_node(
        "start",
        "start",
        "开始",
        r#"
data:
  variables:
    - variable: query
      type: paragraph
    - variable: uploads
      type: file-list
"#,
    );
    let llm_node = workflow_node(
        "llm_1",
        "llm",
        "LLM",
        r#"
data:
  outputs:
    text:
      type: string
      children: null
    score:
      type: number
      children: null
"#,
    );
    let tool_node = workflow_node(
        "tool_1",
        "tool",
        "工具",
        r#"
data:
  outputs:
    payload:
      type: object
      children: null
"#,
    );
    let code_node = workflow_node(
        "code_1",
        "code",
        "代码",
        r#"
data:
  code_language: python3
  code: |
    def main():
      return {}
"#,
    );

    let document = WorkflowDocument {
        name: "测试工作流".to_string(),
        nodes: vec![start_node, llm_node, tool_node, code_node],
        edges: vec![
            WorkflowEdge {
                id: "edge_start_llm".to_string(),
                source: "start".to_string(),
                target: "llm_1".to_string(),
                source_handle: None,
                target_handle: None,
                source_type: "start".to_string(),
                target_type: "llm".to_string(),
                selected: false,
                z_index: 0.0,
                raw_edge: Value::Null,
            },
            WorkflowEdge {
                id: "edge_llm_code".to_string(),
                source: "llm_1".to_string(),
                target: "code_1".to_string(),
                source_handle: None,
                target_handle: None,
                source_type: "llm".to_string(),
                target_type: "code".to_string(),
                selected: false,
                z_index: 0.0,
                raw_edge: Value::Null,
            },
        ],
        viewport: WorkflowViewport::default(),
    };
    let state = workflow_state_with_active_app(
        document,
        vec![WorkflowEnvironmentVariable {
            id: "env_api_key".to_string(),
            name: "api_key".to_string(),
            value_type: "string".to_string(),
            value: Value::String("secret".to_string()),
            description: String::new(),
            raw_variable: Value::Null,
        }],
        vec![WorkflowConversationVariable {
            id: "conv_thread_id".to_string(),
            name: "thread_id".to_string(),
            value_type: "string".to_string(),
            value: Value::String("thread-1".to_string()),
            description: String::new(),
            raw_variable: Value::Null,
        }],
    );
    let editor = minimal_code_editor("code_1");

    let options = build_code_variable_reference_options(&state, &editor);
    let selector_keys =
        options.iter().map(|option| option.selector_key.as_str()).collect::<Vec<_>>();

    assert!(selector_keys.contains(&"start.query"));
    assert!(selector_keys.contains(&"start.uploads"));
    assert!(selector_keys.contains(&"llm_1.text"));
    assert!(selector_keys.contains(&"llm_1.score"));
    assert!(!selector_keys.contains(&"tool_1.payload"));
    assert!(selector_keys.contains(&"env.api_key"));
    assert!(selector_keys.contains(&"conversation.thread_id"));
    assert!(selector_keys.contains(&"sys.user_id"));
    assert!(selector_keys.contains(&"sys.conversation_id"));

    let uploads = options
        .iter()
        .find(|option| option.selector_key == "start.uploads")
        .expect("start file-list variable should be available");
    assert_eq!(uploads.value_type, "array[file]");

    let llm_text = options
        .iter()
        .find(|option| option.selector_key == "llm_1.text")
        .expect("llm output should be available");
    assert_eq!(llm_text.label, "LLM · text");
    assert_eq!(llm_text.value_type, "string");
}

#[test]
fn code_variable_reference_options_for_create_mode_include_all_nodes_and_deduplicate() {
    let start_node = workflow_node(
        "start",
        "start",
        "开始",
        r#"
data:
  variables:
    - variable: query
      type: text-input
"#,
    );
    let first_node = workflow_node(
        "first",
        "llm",
        "第一个节点",
        r#"
data:
  outputs:
    answer:
      type: string
"#,
    );
    let second_node = workflow_node(
        "second",
        "tool",
        "第二个节点",
        r#"
data:
  outputs:
    answer:
      type: number
"#,
    );
    let document = WorkflowDocument {
        name: "测试工作流".to_string(),
        nodes: vec![start_node, first_node, second_node],
        edges: Vec::new(),
        viewport: WorkflowViewport::default(),
    };
    let state = workflow_state_with_active_app(
        document,
        vec![WorkflowEnvironmentVariable {
            id: "env_query".to_string(),
            name: "query".to_string(),
            value_type: "string".to_string(),
            value: Value::Null,
            description: String::new(),
            raw_variable: Value::Null,
        }],
        Vec::new(),
    );
    let mut editor = minimal_code_editor("new_code");
    editor.mode = WorkflowNodeEditorMode::Create;

    let options = build_code_variable_reference_options(&state, &editor);
    let selector_keys =
        options.iter().map(|option| option.selector_key.as_str()).collect::<Vec<_>>();

    assert!(selector_keys.contains(&"start.query"));
    assert!(selector_keys.contains(&"first.answer"));
    assert!(selector_keys.contains(&"second.answer"));
    assert!(selector_keys.contains(&"env.query"));
    assert_eq!(selector_keys.iter().filter(|key| **key == "start.query").count(), 1);
}

#[test]
fn code_variable_reference_options_skip_current_node_and_non_upstream_chain() {
    let start_node = workflow_node(
        "start",
        "start",
        "开始",
        r#"
data:
  variables:
    - variable: query
      type: paragraph
"#,
    );
    let middle_node = workflow_node(
        "middle",
        "llm",
        "中间",
        r#"
data:
  outputs:
    text:
      type: string
"#,
    );
    let code_node = workflow_node(
        "code_1",
        "code",
        "代码",
        r#"
data:
  outputs:
    own:
      type: string
"#,
    );
    let unrelated_node = workflow_node(
        "unrelated",
        "tool",
        "无关",
        r#"
data:
  outputs:
    payload:
      type: object
"#,
    );
    let document = WorkflowDocument {
        name: "测试工作流".to_string(),
        nodes: vec![start_node, middle_node, code_node, unrelated_node],
        edges: vec![
            WorkflowEdge {
                id: "edge_start_middle".to_string(),
                source: "start".to_string(),
                target: "middle".to_string(),
                source_handle: None,
                target_handle: None,
                source_type: "start".to_string(),
                target_type: "llm".to_string(),
                selected: false,
                z_index: 0.0,
                raw_edge: Value::Null,
            },
            WorkflowEdge {
                id: "edge_middle_code".to_string(),
                source: "middle".to_string(),
                target: "code_1".to_string(),
                source_handle: None,
                target_handle: None,
                source_type: "llm".to_string(),
                target_type: "code".to_string(),
                selected: false,
                z_index: 0.0,
                raw_edge: Value::Null,
            },
        ],
        viewport: WorkflowViewport::default(),
    };
    let state = workflow_state_with_active_app(document, Vec::new(), Vec::new());
    let editor = minimal_code_editor("code_1");

    let options = build_code_variable_reference_options(&state, &editor);
    let selector_keys =
        options.iter().map(|option| option.selector_key.as_str()).collect::<Vec<_>>();

    assert!(selector_keys.contains(&"start.query"));
    assert!(selector_keys.contains(&"middle.text"));
    assert!(!selector_keys.contains(&"code_1.own"));
    assert!(!selector_keys.contains(&"unrelated.payload"));
}

#[test]
fn start_node_variable_entries_ignore_invalid_names_and_map_input_types() {
    let node = workflow_node(
        "start",
        "start",
        "开始",
        r#"
data:
  variables:
    - variable: count
      type: number
    - variable: accepted
      type: checkbox
    - variable: receipt
      type: file
    - variable: attachments
      type: file-list
    - variable: title
      type: text-input
    - variable: " "
      type: number
"#,
    );

    let entries = start_node_variable_entries(&node);

    assert_eq!(
        entries,
        vec![
            ("count".to_string(), "number".to_string()),
            ("accepted".to_string(), "boolean".to_string()),
            ("receipt".to_string(), "file".to_string()),
            ("attachments".to_string(), "array[file]".to_string()),
            ("title".to_string(), "string".to_string()),
        ],
    );
}

#[test]
fn start_node_variable_entries_and_node_outputs_default_when_yaml_is_missing_or_invalid() {
    let missing_data = workflow_node("node", "start", "开始", "{}");
    let invalid_variables = workflow_node(
        "node",
        "start",
        "开始",
        r#"
data:
  variables:
    query:
      type: text-input
"#,
    );
    let invalid_outputs = workflow_node(
        "node",
        "llm",
        "LLM",
        r#"
data:
  outputs:
    - text
"#,
    );

    assert!(node_data_map(&missing_data).is_none());
    assert!(start_node_variable_entries(&missing_data).is_empty());
    assert!(start_node_variable_entries(&invalid_variables).is_empty());
    assert!(node_output_entries(&invalid_outputs).is_empty());
}

#[test]
fn node_output_entries_ignore_blank_keys_and_default_missing_types() {
    let node = workflow_node(
        "llm",
        "llm",
        "LLM",
        r#"
data:
  outputs:
    text:
      type: string
    score:
      type: number
    payload: {}
    " ":
      type: object
"#,
    );

    assert_eq!(
        node_output_entries(&node),
        vec![
            ("text".to_string(), "string".to_string()),
            ("score".to_string(), "number".to_string()),
            ("payload".to_string(), "string".to_string()),
        ],
    );
}

#[test]
fn workflow_code_variable_reference_option_display_includes_type_label() {
    let option = WorkflowCodeVariableReferenceOption {
        selector_key: "llm.score".to_string(),
        label: "LLM · score".to_string(),
        value_type: "number".to_string(),
    };

    assert_eq!(option.to_string(), "LLM · score · Number");
}

#[test]
fn code_row_builders_cover_empty_options_fallbacks_and_validation_errors() {
    let validation = validation_with_errors(&[
        "code.inputs[0].variable",
        "code.inputs[0].selector",
        "code.inputs[0].value_type",
        "code.outputs[0].key",
        "code.outputs[0].type",
    ]);
    let input = WorkflowCodeVariableDraft {
        variable: "payload".to_string(),
        value_type: "object".to_string(),
        selector: vec!["tool".to_string(), "payload".to_string()],
    };
    let output =
        WorkflowCodeOutputDraft { key: "result".to_string(), value_type: "unknown".to_string() };

    let _input_row = build_code_input_variable_row(0, &input, &[], &validation);
    let _output_row =
        build_code_output_variable_row(0, &output, &code_output_type_options(), &validation);
}

#[test]
fn integration_visual_sections_build_with_validation_errors() {
    let validation = validation_with_errors(&[
        "knowledge.dataset_ids",
        "knowledge.retrieval_mode",
        "knowledge.query_selector",
        "knowledge.multiple.top_k",
        "knowledge.multiple.score_threshold",
        "knowledge.single.provider",
        "knowledge.single.model_name",
        "knowledge.single.model_mode",
        "tool.provider_id",
        "tool.provider_type",
        "tool.provider_name",
        "tool.tool_name",
        "tool.tool_parameters",
        "tool.tool_configurations",
        "agent.strategy_provider",
        "agent.strategy_name",
        "agent.strategy_label",
        "agent.output_schema",
        "agent.parameters",
        "agent.memory.window_size",
    ]);
    let yaml_editor = text_editor::Content::with_text("key: value");

    let _knowledge = build_knowledge_visual_section(
        &validation,
        "sys.query",
        "start.files",
        "kb_orders",
        "multiple",
        "5",
        true,
        "0.5",
        true,
        &yaml_editor,
        "automatic",
        &yaml_editor,
        "langgenius/openai/openai",
        "gpt-4o-mini",
        "chat",
    );
    let _tool = build_tool_visual_section(
        &validation,
        "google",
        "builtin",
        "google/google",
        "search",
        "Search",
        "Search tool",
        "cred-1",
        "plugin-1",
        &yaml_editor,
        &yaml_editor,
    );
    let _agent = build_agent_visual_section(
        &validation,
        "langgenius/openai/openai",
        "function_call",
        "Function Call",
        "plugin-1",
        &yaml_editor,
        &yaml_editor,
        true,
        "3",
        &yaml_editor,
    );
}

#[test]
fn integration_visual_sections_build_without_validation_errors_and_disabled_toggles() {
    let validation = WorkflowNodeEditorValidation::default();
    let yaml_editor = text_editor::Content::with_text("{}");

    let _knowledge = build_knowledge_visual_section(
        &validation,
        "",
        "",
        "",
        "single",
        "",
        false,
        "",
        false,
        &yaml_editor,
        "disabled",
        &yaml_editor,
        "",
        "",
        "",
    );
    let _tool = build_tool_visual_section(
        &validation,
        "",
        "",
        "",
        "",
        "",
        "",
        "",
        "",
        &yaml_editor,
        &yaml_editor,
    );
    let _agent = build_agent_visual_section(
        &validation,
        "",
        "",
        "",
        "",
        &yaml_editor,
        &yaml_editor,
        false,
        "",
        &yaml_editor,
    );
}

#[test]
fn code_visual_section_builds_empty_disabled_retry_and_default_value_branches() {
    let validation = validation_with_errors(&[
        "code.language",
        "code.body",
        "code.outputs",
        "code.retry.max_retries",
        "code.error_strategy",
        "code.default_value",
    ]);
    let state = workflow_state_with_active_app(empty_document(), Vec::new(), Vec::new());
    let editor = minimal_code_editor("code_1");
    let code_editor = text_editor::Content::with_text("def main():\n    return {}");
    let default_value_editor = text_editor::Content::with_text("result: ok");

    let _element = build_code_visual_section(
        &state,
        &editor,
        &validation,
        "ruby",
        &[],
        &code_editor,
        &[],
        WorkflowNodeRetryDraft { enabled: false, max_retries: 1, retry_interval: 100 },
        "default-value",
        &default_value_editor,
    );
}

#[test]
fn code_visual_section_builds_populated_enabled_retry_and_fail_branch() {
    let validation = validation_with_errors(&["code.retry.retry_interval"]);
    let upstream = workflow_node(
        "start",
        "start",
        "开始",
        r#"
data:
  variables:
    - variable: query
      type: paragraph
"#,
    );
    let code_node = workflow_node("code_1", "code", "代码", "data: {}");
    let document = WorkflowDocument {
        name: "测试工作流".to_string(),
        nodes: vec![upstream, code_node],
        edges: vec![WorkflowEdge {
            id: "edge_start_code".to_string(),
            source: "start".to_string(),
            target: "code_1".to_string(),
            source_handle: None,
            target_handle: None,
            source_type: "start".to_string(),
            target_type: "code".to_string(),
            selected: false,
            z_index: 0.0,
            raw_edge: Value::Null,
        }],
        viewport: WorkflowViewport::default(),
    };
    let state = workflow_state_with_active_app(document, Vec::new(), Vec::new());
    let editor = minimal_code_editor("code_1");
    let code_editor =
        text_editor::Content::with_text("def main(query):\n    return {\"result\": query}");
    let default_value_editor = text_editor::Content::with_text("{}");
    let inputs = vec![WorkflowCodeVariableDraft {
        variable: "query".to_string(),
        value_type: "string".to_string(),
        selector: vec!["start".to_string(), "query".to_string()],
    }];
    let outputs = vec![WorkflowCodeOutputDraft {
        key: "result".to_string(),
        value_type: "string".to_string(),
    }];

    let _element = build_code_visual_section(
        &state,
        &editor,
        &validation,
        "python3",
        &inputs,
        &code_editor,
        &outputs,
        WorkflowNodeRetryDraft { enabled: true, max_retries: 3, retry_interval: 1200 },
        "fail-branch",
        &default_value_editor,
    );
}

#[test]
fn workflow_code_picker_option_display_returns_label() {
    let option = WorkflowCodePickerOption { value: "python3", label: "PYTHON3" };

    assert_eq!(option.to_string(), "PYTHON3");
}

#[test]
fn start_input_value_type_covers_all_supported_mappings() {
    assert_eq!(start_input_value_type("number"), "number");
    assert_eq!(start_input_value_type("checkbox"), "boolean");
    assert_eq!(start_input_value_type("file"), "file");
    assert_eq!(start_input_value_type("file-list"), "array[file]");
    assert_eq!(start_input_value_type("paragraph"), "string");
}

#[test]
fn upstream_node_ids_collect_transitive_sources_without_duplicates() {
    let document = WorkflowDocument {
        name: "测试工作流".to_string(),
        nodes: Vec::new(),
        edges: vec![
            WorkflowEdge {
                id: "edge_start_llm".to_string(),
                source: "start".to_string(),
                target: "llm".to_string(),
                source_handle: None,
                target_handle: None,
                source_type: "start".to_string(),
                target_type: "llm".to_string(),
                selected: false,
                z_index: 0.0,
                raw_edge: Value::Null,
            },
            WorkflowEdge {
                id: "edge_llm_tool".to_string(),
                source: "llm".to_string(),
                target: "tool".to_string(),
                source_handle: None,
                target_handle: None,
                source_type: "llm".to_string(),
                target_type: "tool".to_string(),
                selected: false,
                z_index: 0.0,
                raw_edge: Value::Null,
            },
            WorkflowEdge {
                id: "edge_start_tool".to_string(),
                source: "start".to_string(),
                target: "tool".to_string(),
                source_handle: None,
                target_handle: None,
                source_type: "start".to_string(),
                target_type: "tool".to_string(),
                selected: false,
                z_index: 0.0,
                raw_edge: Value::Null,
            },
            WorkflowEdge {
                id: "edge_tool_code".to_string(),
                source: "tool".to_string(),
                target: "code".to_string(),
                source_handle: None,
                target_handle: None,
                source_type: "tool".to_string(),
                target_type: "code".to_string(),
                selected: false,
                z_index: 0.0,
                raw_edge: Value::Null,
            },
        ],
        viewport: WorkflowViewport::default(),
    };
    let state = workflow_state_with_active_app(document, Vec::new(), Vec::new());

    let upstream = upstream_node_ids(&state, "code");

    assert_eq!(upstream.len(), 3);
    assert!(upstream.contains("tool"));
    assert!(upstream.contains("llm"));
    assert!(upstream.contains("start"));
}

#[test]
fn upstream_node_ids_terminates_and_deduplicates_cycles() {
    let document = WorkflowDocument {
        name: "循环工作流".to_string(),
        nodes: Vec::new(),
        edges: vec![
            WorkflowEdge {
                id: "edge_a_code".to_string(),
                source: "a".to_string(),
                target: "code".to_string(),
                source_handle: None,
                target_handle: None,
                source_type: "answer".to_string(),
                target_type: "code".to_string(),
                selected: false,
                z_index: 0.0,
                raw_edge: Value::Null,
            },
            WorkflowEdge {
                id: "edge_code_a".to_string(),
                source: "code".to_string(),
                target: "a".to_string(),
                source_handle: None,
                target_handle: None,
                source_type: "code".to_string(),
                target_type: "answer".to_string(),
                selected: false,
                z_index: 0.0,
                raw_edge: Value::Null,
            },
        ],
        viewport: WorkflowViewport::default(),
    };
    let state = workflow_state_with_active_app(document, Vec::new(), Vec::new());

    let upstream = upstream_node_ids(&state, "code");

    assert_eq!(upstream.len(), 2);
    assert!(upstream.contains("a"));
    assert!(upstream.contains("code"));
}

#[test]
fn node_data_map_returns_mapping_when_present() {
    let node = workflow_node(
        "start",
        "start",
        "开始",
        r#"
data:
  variables:
    - variable: query
      type: text-input
"#,
    );

    let data = node_data_map(&node).expect("data mapping should exist");

    assert!(data.contains_key(Value::String("variables".to_string())));
}

#[test]
fn code_variable_reference_options_without_active_meta_skip_system_variables() {
    let start_node = workflow_node(
        "start",
        "start",
        "开始",
        r#"
data:
  variables:
    - variable: query
      type: text-input
"#,
    );
    let code_node = workflow_node("code_1", "code", "代码", "data: {}");
    let document = WorkflowDocument {
        name: "测试工作流".to_string(),
        nodes: vec![start_node, code_node],
        edges: vec![WorkflowEdge {
            id: "edge_start_code".to_string(),
            source: "start".to_string(),
            target: "code_1".to_string(),
            source_handle: None,
            target_handle: None,
            source_type: "start".to_string(),
            target_type: "code".to_string(),
            selected: false,
            z_index: 0.0,
            raw_edge: Value::Null,
        }],
        viewport: WorkflowViewport::default(),
    };
    let mut state = workflow_state_with_active_app(
        document,
        vec![WorkflowEnvironmentVariable {
            id: "env_api_key".to_string(),
            name: "api_key".to_string(),
            value_type: "string".to_string(),
            value: Value::Null,
            description: String::new(),
            raw_variable: Value::Null,
        }],
        vec![WorkflowConversationVariable {
            id: "conv_thread_id".to_string(),
            name: "thread_id".to_string(),
            value_type: "string".to_string(),
            value: Value::Null,
            description: String::new(),
            raw_variable: Value::Null,
        }],
    );
    state.active_app_id = None;
    let editor = minimal_code_editor("code_1");

    let options = build_code_variable_reference_options(&state, &editor);
    let selector_keys =
        options.iter().map(|option| option.selector_key.as_str()).collect::<Vec<_>>();

    assert!(selector_keys.contains(&"start.query"));
    assert!(selector_keys.contains(&"env.api_key"));
    assert!(selector_keys.contains(&"conversation.thread_id"));
    assert!(!selector_keys.iter().any(|key| key.starts_with("sys.")));
}

#[test]
fn code_row_builders_support_selected_option_backfill_and_known_output_type() {
    let input = WorkflowCodeVariableDraft {
        variable: "payload".to_string(),
        value_type: "object".to_string(),
        selector: vec!["tool".to_string(), "payload".to_string()],
    };
    let options = vec![WorkflowCodeVariableReferenceOption {
        selector_key: "start.query".to_string(),
        label: "query".to_string(),
        value_type: "string".to_string(),
    }];
    let output =
        WorkflowCodeOutputDraft { key: "result".to_string(), value_type: "number".to_string() };

    let validation = WorkflowNodeEditorValidation::default();
    let output_type_options = code_output_type_options();
    let _input_row = build_code_input_variable_row(0, &input, &options, &validation);
    let _output_row = build_code_output_variable_row(0, &output, &output_type_options, &validation);
}

#[test]
fn code_visual_section_builds_none_error_strategy_without_default_value() {
    let state = workflow_state_with_active_app(empty_document(), Vec::new(), Vec::new());
    let editor = minimal_code_editor("code_1");
    let validation = WorkflowNodeEditorValidation::default();
    let code_editor = text_editor::Content::with_text("def main():\n    return {\"result\": 1}");
    let default_value_editor = text_editor::Content::with_text("result: 1");
    let outputs = vec![WorkflowCodeOutputDraft {
        key: "result".to_string(),
        value_type: "number".to_string(),
    }];

    let _element = build_code_visual_section(
        &state,
        &editor,
        &validation,
        "javascript",
        &[],
        &code_editor,
        &outputs,
        WorkflowNodeRetryDraft { enabled: false, max_retries: 1, retry_interval: 100 },
        "none",
        &default_value_editor,
    );
}
