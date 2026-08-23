//! 开始节点元数据辅助视图的测试，覆盖内置变量、类型标签和错误摘要。
//! 活动模式与校验 fixture 用于固定聊天兼容规则及错误排序。
use super::*;
use crate::apps::workflow::model::{WorkflowAppMeta, WorkflowDocument};
use crate::apps::workflow::state::{
    WorkflowAppEntry, WorkflowHistorySnapshot, WorkflowNodeValidationError,
    WorkflowStartVariableDraft,
};
use iced::Vector;
use serde_yaml::Value;

fn active_state_with_mode(mode: &str) -> WorkflowState {
    let document = WorkflowDocument::default();
    let meta = WorkflowAppMeta { mode: mode.to_string(), ..WorkflowAppMeta::default() };
    let saved_snapshot = WorkflowHistorySnapshot {
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
            id: "app_1".to_string(),
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
            saved_snapshot,
        }],
        active_app_id: Some("app_1".to_string()),
        document,
        zoom: 1.0,
        ..WorkflowState::default()
    }
}

fn start_variable(input_type: &str, default_value: &str) -> WorkflowStartVariableDraft {
    WorkflowStartVariableDraft {
        raw_variable: Value::Null,
        label: "变量".to_string(),
        variable: "var".to_string(),
        input_type: input_type.to_string(),
        required: false,
        hidden: false,
        options: Vec::new(),
        allowed_file_types: Vec::new(),
        allowed_file_extensions: Vec::new(),
        allowed_file_extensions_input: String::new(),
        allowed_file_upload_methods: Vec::new(),
        default_value: default_value.to_string(),
        default_file_values: Vec::new(),
        placeholder: String::new(),
        hint: String::new(),
        max_length_input: String::new(),
    }
}

#[test]
fn start_builtin_variables_defaults_to_chat_inputs_without_active_app() {
    let items = start_builtin_variables(&WorkflowState::default());

    assert_eq!(items.len(), 2);
    assert_eq!(items[0].name, "userinput.query");
    assert_eq!(items[0].value_type, "String");
    assert_eq!(items[0].description, "用户当前这一轮输入的问题文本。");
    assert!(!items[0].legacy);
    assert_eq!(items[1].name, "userinput.files");
    assert_eq!(items[1].value_type, "Array[File]");
    assert_eq!(items[1].description, "用户上传的文件列表，可作为文件型输入或后续节点的附件来源。");
    assert!(!items[1].legacy);
}

#[test]
fn start_builtin_variables_marks_file_input_legacy_outside_chat_mode() {
    let state = active_state_with_mode("workflow");
    let items = start_builtin_variables(&state);

    assert_eq!(items.len(), 1);
    assert_eq!(items[0].name, "userinput.files");
    assert_eq!(items[0].value_type, "Array[File]");
    assert!(items[0].legacy);
}

#[test]
fn start_builtin_variables_treats_modes_containing_chat_as_chat() {
    let state = active_state_with_mode("advanced-chat");
    let names =
        start_builtin_variables(&state).into_iter().map(|item| item.name).collect::<Vec<_>>();

    assert_eq!(names, vec!["userinput.query", "userinput.files"]);
}

#[test]
fn start_variable_value_type_label_maps_supported_inputs() {
    assert_eq!(start_variable_value_type_label("number"), "number");
    assert_eq!(start_variable_value_type_label("checkbox"), "boolean");
    assert_eq!(start_variable_value_type_label("file"), "file");
    assert_eq!(start_variable_value_type_label("file-list"), "array[file]");
    assert_eq!(start_variable_value_type_label("text-input"), "string");
    assert_eq!(start_variable_value_type_label("unknown"), "string");
}

#[test]
fn start_variable_summary_error_collects_known_field_errors_in_order() {
    let validation = state::WorkflowNodeEditorValidation {
        field_errors: vec![
            WorkflowNodeValidationError {
                path: "start.variables[0].variable".to_string(),
                message: "变量名不能为空".to_string(),
            },
            WorkflowNodeValidationError {
                path: "start.variables[0].label".to_string(),
                message: "显示名称不能为空".to_string(),
            },
            WorkflowNodeValidationError {
                path: "start.variables[0].default".to_string(),
                message: "默认值无效".to_string(),
            },
            WorkflowNodeValidationError {
                path: "start.variables[1].label".to_string(),
                message: "其他变量错误".to_string(),
            },
            WorkflowNodeValidationError {
                path: "start.variables[0].ignored".to_string(),
                message: "忽略字段".to_string(),
            },
        ],
    };

    let error = start_variable_summary_error(0, &validation);

    assert_eq!(error.as_deref(), Some("显示名称不能为空 · 变量名不能为空 · 默认值无效"));
}

#[test]
fn start_variable_summary_error_returns_none_without_matching_errors() {
    let validation = state::WorkflowNodeEditorValidation {
        field_errors: vec![WorkflowNodeValidationError {
            path: "start.variables[2].label".to_string(),
            message: "其他变量错误".to_string(),
        }],
    };

    assert_eq!(start_variable_summary_error(0, &validation), None);
}

#[test]
fn start_variable_summary_error_includes_advanced_constraint_errors_in_order() {
    let paths_and_messages = [
        ("max_length", "最大长度无效"),
        ("options", "至少需要一个选项"),
        ("allowed_file_types", "文件类型无效"),
        ("allowed_file_extensions", "扩展名无效"),
        ("allowed_file_upload_methods", "上传方式无效"),
    ];
    let validation = state::WorkflowNodeEditorValidation {
        field_errors: paths_and_messages
            .iter()
            .map(|(field, message)| WorkflowNodeValidationError {
                path: format!("start.variables[3].{field}"),
                message: (*message).to_string(),
            })
            .collect(),
    };

    assert_eq!(
        start_variable_summary_error(3, &validation).as_deref(),
        Some("最大长度无效 · 至少需要一个选项 · 文件类型无效 · 扩展名无效 · 上传方式无效")
    );
}

#[test]
fn start_variable_default_error_only_rejects_invalid_number_defaults() {
    assert_eq!(
        start_variable_default_error(&start_variable("number", "abc")),
        Some("数字类型默认值必须是数字")
    );
    assert_eq!(start_variable_default_error(&start_variable("number", "12.5")), None);
    assert_eq!(start_variable_default_error(&start_variable("number", "")), None);
    assert_eq!(start_variable_default_error(&start_variable("text-input", "abc")), None);
}

#[test]
fn start_variable_advanced_hint_has_no_current_special_cases() {
    assert_eq!(start_variable_advanced_hint("number"), None);
    assert_eq!(start_variable_advanced_hint("file-list"), None);
}

#[test]
fn start_variable_type_options_are_stable_and_displayable() {
    let options = start_variable_type_options();

    assert_eq!(options.len(), 7);
    assert_eq!(options[0].input_type, "text-input");
    assert_eq!(options[0].label, "文本");
    assert_eq!(options[0].value_type, "string");
    assert_eq!(options[3].input_type, "number");
    assert_eq!(options[3].to_string(), "数字 · number");
    assert_eq!(options[6].input_type, "file-list");
    assert_eq!(options[6].value_type, "array[file]");
}

#[test]
fn selected_start_variable_type_option_matches_input_type() {
    let option = selected_start_variable_type_option("checkbox");

    assert_eq!(option.map(|item| item.label), Some("复选框"));
    assert_eq!(selected_start_variable_type_option("unsupported"), None);
}

#[test]
fn start_variable_file_type_options_are_stable() {
    assert_eq!(
        start_variable_file_type_options(),
        [
            ("document", "文档", "PDF, DOCX"),
            ("image", "图片", "PNG, JPG"),
            ("audio", "音频", "MP3, WAV"),
            ("video", "视频", "MP4, MOV"),
            ("custom", "自定义", "手动填写扩展名"),
        ]
    );
}

#[test]
fn start_variable_ui_builders_return_elements() {
    let badge = start_variable_badge("string");
    let known_selector = build_start_variable_type_selector("number");
    let fallback_selector = build_start_variable_type_selector("unknown");

    std::hint::black_box(badge);
    std::hint::black_box(known_selector);
    std::hint::black_box(fallback_selector);
}
