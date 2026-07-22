use iced::widget::text_editor;
use serde_yaml::Value;

use super::*;

fn consume_element<'a>(element: Element<'a, Message>) {
    std::hint::black_box(element);
}

fn variable_with_type(input_type: &str) -> super::super::state::WorkflowStartVariableDraft {
    super::super::state::WorkflowStartVariableDraft {
        raw_variable: Value::Mapping(Default::default()),
        label: "新变量".to_string(),
        variable: "input".to_string(),
        input_type: input_type.to_string(),
        required: true,
        hidden: false,
        options: Vec::new(),
        allowed_file_types: Vec::new(),
        allowed_file_extensions: Vec::new(),
        allowed_file_extensions_input: String::new(),
        allowed_file_upload_methods: Vec::new(),
        default_value: String::new(),
        default_file_values: Vec::new(),
        placeholder: String::new(),
        hint: String::new(),
        max_length_input: "48".to_string(),
    }
}

fn editor_with_variable(
    variable: super::super::state::WorkflowStartVariableDraft,
) -> super::super::state::WorkflowStartVariableEditorDraft {
    let default_value_editor = text_editor::Content::with_text(&variable.default_value);
    let default_file_url_input = variable.default_value.clone();

    super::super::state::WorkflowStartVariableEditorDraft {
        mode: super::super::state::WorkflowStartVariableEditorMode::Create,
        variable,
        default_value_editor,
        default_file_url_input,
        show_default_file_url_input: false,
    }
}

#[test]
fn start_variable_file_type_options_returns_stable_options() {
    assert_eq!(
        start_variable_file_type_options(),
        [
            ("document", "文档", "pdf, doc, docx, txt, md"),
            ("image", "图片", "png, jpg, jpeg, webp, gif"),
            ("audio", "音频", "mp3, wav, m4a"),
            ("video", "视频", "mp4, mov, avi"),
            ("custom", "自定义", "手动填写扩展名"),
        ]
    );
}

#[test]
fn build_start_variable_option_editor_handles_empty_and_existing_options() {
    let empty_variable = variable_with_type("select");
    consume_element(build_start_variable_option_editor(&empty_variable));

    let mut variable = variable_with_type("select");
    variable.options = vec!["alpha".to_string(), " beta ".to_string()];

    consume_element(build_start_variable_option_editor(&variable));
}

#[test]
fn build_start_variable_select_default_field_handles_empty_matching_and_missing_defaults() {
    let empty_variable = variable_with_type("select");
    consume_element(build_start_variable_select_default_field(&empty_variable));

    let mut matching_variable = variable_with_type("select");
    matching_variable.options = vec!["alpha".to_string(), " ".to_string(), "beta".to_string()];
    matching_variable.default_value = "beta".to_string();
    consume_element(build_start_variable_select_default_field(&matching_variable));

    let mut missing_variable = matching_variable.clone();
    missing_variable.default_value = "gamma".to_string();
    consume_element(build_start_variable_select_default_field(&missing_variable));
}

#[test]
fn build_start_variable_buttons_handle_selected_and_unselected_states() {
    consume_element(build_start_variable_file_type_button("document", "文档", "pdf, doc", true));
    consume_element(build_start_variable_file_type_button("image", "图片", "png, jpg", false));
    consume_element(build_start_variable_upload_method_button("local_file", "本地上传", true));
    consume_element(build_start_variable_upload_method_button("remote_url", "URL", false));
    consume_element(build_start_variable_default_file_button(
        Icon::Link,
        "粘贴文件链接",
        WorkflowMessage::NodeEditorStartVariableEditorOpenDefaultFileUrlInput,
    ));
}

#[test]
fn build_start_variable_file_settings_handles_local_remote_custom_and_default_files() {
    let mut variable = variable_with_type("file");
    variable.allowed_file_types = vec!["document".to_string(), "custom".to_string()];
    variable.allowed_file_upload_methods = vec!["local_file".to_string(), "remote_url".to_string()];
    variable.allowed_file_extensions_input = ".csv, .json".to_string();
    variable.default_file_values = vec!["https://example.test/a.pdf".to_string()];
    let mut editor = editor_with_variable(variable);
    editor.show_default_file_url_input = true;
    editor.default_file_url_input = "https://example.test/b.pdf".to_string();

    consume_element(build_start_variable_file_settings(&editor));
}

#[test]
fn build_start_variable_file_settings_handles_file_list_and_upload_method_edges() {
    let mut file_list_variable = variable_with_type("file-list");
    file_list_variable.allowed_file_types = vec!["image".to_string()];
    file_list_variable.allowed_file_upload_methods = vec!["local_file".to_string()];
    file_list_variable.max_length_input = "3".to_string();
    file_list_variable.default_file_values = vec!["one.png".to_string(), "two.png".to_string()];
    let file_list_editor = editor_with_variable(file_list_variable);
    consume_element(build_start_variable_file_settings(&file_list_editor));

    let mut remote_only_variable = variable_with_type("file");
    remote_only_variable.allowed_file_upload_methods = vec!["remote_url".to_string()];
    let remote_only_editor = editor_with_variable(remote_only_variable);
    consume_element(build_start_variable_file_settings(&remote_only_editor));

    let no_upload_editor = editor_with_variable(variable_with_type("file"));
    consume_element(build_start_variable_file_settings(&no_upload_editor));
}

#[test]
fn build_start_variable_file_default_value_handles_hidden_overlay_and_unsupported_type() {
    let mut file_variable = variable_with_type("file");
    file_variable.allowed_file_upload_methods = vec!["local_file".to_string()];
    let file_editor = editor_with_variable(file_variable);
    consume_element(build_start_variable_file_default_value(&file_editor));

    let unsupported_editor = editor_with_variable(variable_with_type("text-input"));
    consume_element(build_start_variable_file_settings(&unsupported_editor));
}

#[test]
fn build_start_variable_file_default_value_handles_remote_overlay_and_file_list_fallback_count() {
    let mut remote_only_variable = variable_with_type("file");
    remote_only_variable.allowed_file_upload_methods = vec!["remote_url".to_string()];
    let mut remote_only_editor = editor_with_variable(remote_only_variable);
    remote_only_editor.show_default_file_url_input = true;
    remote_only_editor.default_file_url_input = "https://example.test/remote.mov".to_string();
    consume_element(build_start_variable_file_default_value(&remote_only_editor));

    let mut file_list_variable = variable_with_type("file-list");
    file_list_variable.max_length_input = "bad".to_string();
    file_list_variable.default_file_values = vec![
        "https://example.test/one.png".to_string(),
        "https://example.test/two.png".to_string(),
        "https://example.test/three.png".to_string(),
    ];
    let file_list_editor = editor_with_variable(file_list_variable);
    consume_element(build_start_variable_file_default_value(&file_list_editor));
}

#[test]
fn build_start_variable_file_settings_handles_file_list_without_custom_type() {
    let mut variable = variable_with_type("file-list");
    variable.allowed_file_types = vec!["document".to_string()];
    variable.allowed_file_upload_methods = vec!["local_file".to_string(), "remote_url".to_string()];
    variable.max_length_input = "11".to_string();
    variable.default_file_values = vec!["draft.pdf".to_string()];

    let editor = editor_with_variable(variable);
    consume_element(build_start_variable_file_settings(&editor));
}

#[test]
fn build_start_variable_file_default_value_handles_both_upload_methods_at_max_count() {
    let mut variable = variable_with_type("file-list");
    variable.allowed_file_upload_methods = vec!["local_file".to_string(), "remote_url".to_string()];
    variable.max_length_input = "2".to_string();
    variable.default_file_values = vec!["one.pdf".to_string(), "two.pdf".to_string()];

    let editor = editor_with_variable(variable);

    consume_element(build_start_variable_file_default_value(&editor));
}

#[test]
fn build_start_variable_file_settings_handles_file_list_without_upload_methods() {
    let mut variable = variable_with_type("file-list");
    variable.allowed_file_types = vec!["custom".to_string()];
    variable.allowed_file_extensions_input = ".zip".to_string();
    variable.max_length_input = "0".to_string();

    let editor = editor_with_variable(variable);

    consume_element(build_start_variable_file_settings(&editor));
}

#[test]
fn build_start_variable_card_handles_select_checkbox_text_paragraph_number_and_files() {
    let mut select_variable = variable_with_type("select");
    select_variable.options = vec!["alpha".to_string(), "beta".to_string()];
    select_variable.default_value = "alpha".to_string();
    let select_editor = editor_with_variable(select_variable);
    consume_element(build_start_variable_card(&select_editor));

    let mut checked_variable = variable_with_type("checkbox");
    checked_variable.default_value = "true".to_string();
    let checked_editor = editor_with_variable(checked_variable);
    consume_element(build_start_variable_card(&checked_editor));

    let mut unchecked_variable = variable_with_type("checkbox");
    unchecked_variable.default_value = "false".to_string();
    let unchecked_editor = editor_with_variable(unchecked_variable);
    consume_element(build_start_variable_card(&unchecked_editor));

    let mut unknown_checkbox_variable = variable_with_type("checkbox");
    unknown_checkbox_variable.default_value = "maybe".to_string();
    let unknown_checkbox_editor = editor_with_variable(unknown_checkbox_variable);
    consume_element(build_start_variable_card(&unknown_checkbox_editor));

    let mut paragraph_variable = variable_with_type("paragraph");
    paragraph_variable.default_value = "line one\nline two".to_string();
    let paragraph_editor = editor_with_variable(paragraph_variable);
    consume_element(build_start_variable_card(&paragraph_editor));

    let mut number_variable = variable_with_type("number");
    number_variable.default_value = "not-a-number".to_string();
    let number_editor = editor_with_variable(number_variable);
    consume_element(build_start_variable_card(&number_editor));

    let text_input_editor = editor_with_variable(variable_with_type("text-input"));
    consume_element(build_start_variable_card(&text_input_editor));

    let file_editor = editor_with_variable(variable_with_type("file"));
    consume_element(build_start_variable_card(&file_editor));
}

#[test]
fn build_start_variable_card_handles_empty_checkbox_default_and_file_list_type() {
    let checkbox_editor = editor_with_variable(variable_with_type("checkbox"));
    consume_element(build_start_variable_card(&checkbox_editor));

    let mut file_list_variable = variable_with_type("file-list");
    file_list_variable.allowed_file_types = vec!["image".to_string()];
    file_list_variable.allowed_file_upload_methods = vec!["local_file".to_string()];
    file_list_variable.default_file_values = vec!["cover.png".to_string()];
    let file_list_editor = editor_with_variable(file_list_variable);
    consume_element(build_start_variable_card(&file_list_editor));
}

#[test]
fn build_start_variable_card_normalizes_whitespace_and_case_in_checkbox_default() {
    let mut variable = variable_with_type("checkbox");
    variable.default_value = "  TrUe  ".to_string();

    let editor = editor_with_variable(variable);

    consume_element(build_start_variable_card(&editor));
}

#[test]
fn build_start_variable_card_handles_valid_number_text_length_and_unknown_type() {
    let mut number_variable = variable_with_type("number");
    number_variable.default_value = "42".to_string();
    let number_editor = editor_with_variable(number_variable);
    consume_element(build_start_variable_card(&number_editor));

    let mut text_variable = variable_with_type("text-input");
    text_variable.max_length_input = "12".to_string();
    text_variable.default_value = "short text".to_string();
    let text_editor = editor_with_variable(text_variable);
    consume_element(build_start_variable_card(&text_editor));

    let mut unknown_variable = variable_with_type("legacy");
    unknown_variable.default_value = "legacy default".to_string();
    let unknown_editor = editor_with_variable(unknown_variable);
    consume_element(build_start_variable_card(&unknown_editor));
}
