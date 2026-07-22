use super::*;

fn draft(input_type: &str) -> WorkflowStartVariableDraft {
    WorkflowStartVariableDraft {
        raw_variable: Value::Mapping(Default::default()),
        label: "Label".to_string(),
        variable: "variable_name".to_string(),
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
        max_length_input: String::new(),
    }
}

fn valid_file_draft(input_type: &str) -> WorkflowStartVariableDraft {
    let mut variable = draft(input_type);
    variable.allowed_file_types = vec!["image".to_string()];
    variable.allowed_file_upload_methods = vec!["local_file".to_string()];
    if input_type == "file-list" {
        variable.max_length_input = "2".to_string();
    }
    variable
}

fn validate_error(variable: &WorkflowStartVariableDraft) -> String {
    validate_start_variable_editor_draft(variable, &[], WorkflowStartVariableEditorMode::Create)
        .expect_err("draft should be invalid")
}

#[test]
fn number_default_validation_accepts_empty_integer_and_float_values() {
    assert!(is_valid_start_variable_number_default_value(""));
    assert!(is_valid_start_variable_number_default_value(" 42 "));
    assert!(is_valid_start_variable_number_default_value("-3.5"));
}

#[test]
fn number_default_validation_rejects_non_numeric_text() {
    assert!(!is_valid_start_variable_number_default_value("12 apples"));
}

#[test]
fn file_default_parser_handles_file_scalar_and_ignores_blank_values() {
    let value = Value::String(" https://example.test/a.png ".to_string());

    assert_eq!(
        parse_start_variable_file_default_values("file", Some(&value)),
        vec!["https://example.test/a.png".to_string()]
    );
    assert!(parse_start_variable_file_default_values("file", Some(&Value::Null)).is_empty());
    assert!(parse_start_variable_file_default_values("text-input", Some(&value)).is_empty());
}

#[test]
fn file_default_parser_deduplicates_file_list_sequence_values() {
    let value = Value::Sequence(vec![
        Value::String(" a.png ".to_string()),
        Value::String(String::new()),
        Value::String("a.png".to_string()),
        Value::Number(serde_yaml::Number::from(7)),
    ]);

    assert_eq!(
        parse_start_variable_file_default_values("file-list", Some(&value)),
        vec!["a.png".to_string(), "7".to_string()]
    );
}

#[test]
fn file_default_parser_handles_file_list_scalar_and_missing_values() {
    let value = Value::String(" remote://one ".to_string());

    assert_eq!(
        parse_start_variable_file_default_values("file-list", Some(&value)),
        vec!["remote://one".to_string()]
    );
    assert_eq!(
        parse_start_variable_file_default_values("file-list", Some(&Value::Bool(true))),
        vec!["true".to_string()]
    );
    assert!(
        parse_start_variable_file_default_values(
            "file-list",
            Some(&Value::Mapping(Default::default()))
        )
        .is_empty()
    );
    assert!(parse_start_variable_file_default_values("file-list", None).is_empty());
}

#[test]
fn file_list_max_length_validation_accepts_only_supported_range() {
    assert!(is_valid_start_variable_file_list_max_length("1"));
    assert!(is_valid_start_variable_file_list_max_length(" 10 "));
    assert!(!is_valid_start_variable_file_list_max_length("0"));
    assert!(!is_valid_start_variable_file_list_max_length("11"));
    assert!(!is_valid_start_variable_file_list_max_length("many"));
}

#[test]
fn file_list_max_length_normalization_clamps_or_defaults_value() {
    assert_eq!(normalized_start_variable_file_list_max_length("0"), 1);
    assert_eq!(normalized_start_variable_file_list_max_length("8"), 8);
    assert_eq!(normalized_start_variable_file_list_max_length("11"), 10);
    assert_eq!(normalized_start_variable_file_list_max_length("bad"), 5);
}

#[test]
fn normalizing_text_variable_clears_file_and_option_fields_and_defaults_max_length() {
    let mut variable = draft("text-input");
    variable.options = vec![" stale ".to_string()];
    variable.allowed_file_types = vec!["image".to_string()];
    variable.allowed_file_extensions = vec![".png".to_string()];
    variable.allowed_file_extensions_input = "png".to_string();
    variable.allowed_file_upload_methods = vec!["local_file".to_string()];

    normalize_start_variable_draft(&mut variable);

    assert_eq!(variable.max_length_input, "48");
    assert!(variable.options.is_empty());
    assert!(variable.allowed_file_types.is_empty());
    assert!(variable.allowed_file_extensions.is_empty());
    assert!(variable.allowed_file_extensions_input.is_empty());
    assert!(variable.allowed_file_upload_methods.is_empty());
}

#[test]
fn normalizing_hidden_variable_disables_required_flag() {
    let mut variable = draft("paragraph");
    variable.hidden = true;

    normalize_start_variable_draft(&mut variable);

    assert!(!variable.required);
}

#[test]
fn normalizing_file_variable_applies_defaults_and_keeps_only_first_default_file() {
    let mut variable = draft("file");
    variable.allowed_file_types = vec![" CUSTOM ".to_string(), "image".to_string()];
    variable.allowed_file_extensions_input = ".PNG png pdf".to_string();
    variable.allowed_file_upload_methods =
        vec!["remote_url".to_string(), "bad".to_string(), "LOCAL_FILE".to_string()];
    variable.default_file_values =
        vec![" first.png ".to_string(), "second.png".to_string(), "first.png".to_string()];
    variable.max_length_input = "9".to_string();

    normalize_start_variable_draft(&mut variable);

    assert_eq!(variable.allowed_file_types, vec!["custom".to_string(), "image".to_string()]);
    assert_eq!(variable.allowed_file_extensions, vec![".pdf".to_string(), ".png".to_string()]);
    assert_eq!(variable.allowed_file_extensions_input, ".pdf, .png");
    assert_eq!(
        variable.allowed_file_upload_methods,
        vec!["local_file".to_string(), "remote_url".to_string()]
    );
    assert_eq!(variable.default_file_values, vec!["first.png".to_string()]);
    assert_eq!(variable.default_value, "first.png");
    assert!(variable.max_length_input.is_empty());
}

#[test]
fn normalizing_file_variable_without_custom_clears_extensions() {
    let mut variable = draft("file");
    variable.allowed_file_types = vec!["document".to_string()];
    variable.allowed_file_extensions_input = "pdf".to_string();

    normalize_start_variable_draft(&mut variable);

    assert!(variable.allowed_file_extensions.is_empty());
    assert!(variable.allowed_file_extensions_input.is_empty());
    assert_eq!(
        variable.allowed_file_upload_methods,
        vec!["local_file".to_string(), "remote_url".to_string()]
    );
}

#[test]
fn normalizing_file_variable_with_blank_default_clears_default_files() {
    let mut variable = draft("file");
    variable.default_file_values = vec![" ".to_string()];

    normalize_start_variable_draft(&mut variable);

    assert!(variable.default_value.is_empty());
    assert!(variable.default_file_values.is_empty());
}

#[test]
fn normalizing_file_list_clamps_count_and_serializes_truncated_defaults() {
    let mut variable = draft("file-list");
    variable.allowed_file_types = vec!["custom".to_string()];
    variable.allowed_file_extensions = vec!["TXT".to_string()];
    variable.allowed_file_upload_methods = vec!["remote_url".to_string()];
    variable.max_length_input = "2".to_string();
    variable.default_file_values =
        vec![" a.txt ".to_string(), "b.txt".to_string(), "c.txt".to_string()];

    normalize_start_variable_draft(&mut variable);

    assert_eq!(variable.max_length_input, "2");
    assert_eq!(variable.default_file_values, vec!["a.txt".to_string(), "b.txt".to_string()]);
    assert!(variable.default_value.contains("a.txt"));
    assert!(variable.default_value.contains("b.txt"));
    assert!(!variable.default_value.contains("c.txt"));
}

#[test]
fn normalizing_file_list_empty_defaults_clears_default_value() {
    let mut variable = draft("file-list");
    variable.default_file_values = vec![" ".to_string()];

    normalize_start_variable_draft(&mut variable);

    assert!(variable.default_value.is_empty());
    assert!(variable.default_file_values.is_empty());
}

#[test]
fn normalizing_file_list_applies_default_file_settings_and_clears_non_custom_extensions() {
    let mut variable = draft("file-list");
    variable.allowed_file_extensions = vec!["png".to_string()];
    variable.allowed_file_extensions_input = ".PNG".to_string();
    variable.max_length_input = String::new();

    normalize_start_variable_draft(&mut variable);

    assert_eq!(variable.allowed_file_types, vec!["image".to_string()]);
    assert_eq!(
        variable.allowed_file_upload_methods,
        vec!["local_file".to_string(), "remote_url".to_string()]
    );
    assert!(variable.allowed_file_extensions.is_empty());
    assert!(variable.allowed_file_extensions_input.is_empty());
    assert_eq!(variable.max_length_input, "5");
}

#[test]
fn normalizing_select_removes_invalid_default_and_non_select_fields() {
    let mut variable = draft("select");
    variable.options = vec![" One ".to_string(), "Two".to_string(), " ".to_string()];
    variable.default_value = "Missing".to_string();
    variable.max_length_input = "48".to_string();
    variable.allowed_file_types = vec!["image".to_string()];
    variable.default_file_values = vec!["file.png".to_string()];

    normalize_start_variable_draft(&mut variable);

    assert_eq!(variable.options, vec!["One".to_string(), "Two".to_string(), String::new()]);
    assert!(variable.default_value.is_empty());
    assert!(variable.max_length_input.is_empty());
    assert!(variable.allowed_file_types.is_empty());
    assert!(variable.default_file_values.is_empty());
}

#[test]
fn normalizing_select_keeps_trimmed_matching_default() {
    let mut variable = draft("select");
    variable.options = vec![" One ".to_string(), "Two".to_string()];
    variable.default_value = " One ".to_string();

    normalize_start_variable_draft(&mut variable);

    assert_eq!(variable.options, vec!["One".to_string(), "Two".to_string()]);
    assert_eq!(variable.default_value, " One ");
}

#[test]
fn normalizing_number_clears_irrelevant_fields() {
    let mut variable = draft("number");
    variable.options = vec!["unused".to_string()];
    variable.max_length_input = "10".to_string();
    variable.allowed_file_types = vec!["image".to_string()];
    variable.default_file_values = vec!["file.png".to_string()];

    normalize_start_variable_draft(&mut variable);

    assert!(variable.options.is_empty());
    assert!(variable.max_length_input.is_empty());
    assert!(variable.allowed_file_types.is_empty());
    assert!(variable.default_file_values.is_empty());
}

#[test]
fn normalizing_checkbox_canonicalizes_known_defaults_and_keeps_unknown() {
    let mut true_variable = draft("checkbox");
    true_variable.default_value = " TRUE ".to_string();
    normalize_start_variable_draft(&mut true_variable);
    assert_eq!(true_variable.default_value, "true");

    let mut empty_variable = draft("checkbox");
    normalize_start_variable_draft(&mut empty_variable);
    assert_eq!(empty_variable.default_value, "false");

    let mut invalid_variable = draft("checkbox");
    invalid_variable.default_value = "maybe".to_string();
    normalize_start_variable_draft(&mut invalid_variable);
    assert_eq!(invalid_variable.default_value, "maybe");
}

#[test]
fn normalizing_checkbox_canonicalizes_explicit_false_default() {
    let mut variable = draft("checkbox");
    variable.default_value = " FALSE ".to_string();

    normalize_start_variable_draft(&mut variable);

    assert_eq!(variable.default_value, "false");
}

#[test]
fn normalizing_unknown_type_clears_capability_specific_fields() {
    let mut variable = draft("unsupported");
    variable.options = vec!["unused".to_string()];
    variable.allowed_file_types = vec!["image".to_string()];
    variable.allowed_file_upload_methods = vec!["local_file".to_string()];
    variable.default_file_values = vec!["file.png".to_string()];

    normalize_start_variable_draft(&mut variable);

    assert!(variable.options.is_empty());
    assert!(variable.allowed_file_types.is_empty());
    assert!(variable.allowed_file_upload_methods.is_empty());
    assert!(variable.default_file_values.is_empty());
}

#[test]
fn validation_accepts_basic_supported_drafts() {
    for input_type in ["text-input", "paragraph", "number", "checkbox"] {
        let mut variable = draft(input_type);
        if input_type == "number" {
            variable.default_value = "3.14".to_string();
        }

        validate_start_variable_editor_draft(
            &variable,
            &[],
            WorkflowStartVariableEditorMode::Create,
        )
        .expect("supported draft should be valid");
    }
}

#[test]
fn validation_accepts_select_and_file_drafts() {
    let mut select = draft("select");
    select.options = vec!["red".to_string(), "blue".to_string()];
    select.default_value = "blue".to_string();
    validate_start_variable_editor_draft(&select, &[], WorkflowStartVariableEditorMode::Create)
        .expect("select should be valid");

    for input_type in ["file", "file-list"] {
        let variable = valid_file_draft(input_type);
        validate_start_variable_editor_draft(
            &variable,
            &[],
            WorkflowStartVariableEditorMode::Create,
        )
        .expect("file draft should be valid");
    }
}

#[test]
fn validation_accepts_non_file_max_length_select_empty_default_and_custom_file() {
    let mut text = draft("text-input");
    text.max_length_input = "0".to_string();
    validate_start_variable_editor_draft(&text, &[], WorkflowStartVariableEditorMode::Create)
        .expect("zero max length should be a non-negative integer");

    let mut select = draft("select");
    select.options = vec![" red ".to_string()];
    validate_start_variable_editor_draft(&select, &[], WorkflowStartVariableEditorMode::Create)
        .expect("empty select default should be valid");

    let mut file = valid_file_draft("file");
    file.allowed_file_types = vec!["custom".to_string()];
    file.allowed_file_extensions = vec![".png".to_string()];
    validate_start_variable_editor_draft(&file, &[], WorkflowStartVariableEditorMode::Create)
        .expect("custom file type with extensions should be valid");
}

#[test]
fn validation_rejects_empty_label_empty_name_invalid_name_and_unsupported_type() {
    let mut variable = draft("text-input");
    variable.label.clear();
    assert_eq!(validate_error(&variable), "显示名称不能为空");

    let mut variable = draft("text-input");
    variable.variable.clear();
    assert_eq!(validate_error(&variable), "变量名不能为空");

    let mut variable = draft("text-input");
    variable.variable = "1bad".to_string();
    assert_eq!(validate_error(&variable), "变量名只能包含字母、数字和下划线，且不能以数字开头");

    let mut variable = draft("unsupported");
    variable.input_type = "unsupported".to_string();
    assert_eq!(validate_error(&variable), "不支持的字段类型");
}

#[test]
fn validation_rejects_duplicate_names_except_current_edit_index() {
    let existing = draft("text-input");
    let mut variable = draft("paragraph");
    variable.variable = "variable_name".to_string();

    let create_error = validate_start_variable_editor_draft(
        &variable,
        std::slice::from_ref(&existing),
        WorkflowStartVariableEditorMode::Create,
    )
    .expect_err("create mode should reject duplicate");
    assert_eq!(create_error, "变量名不能重复");

    validate_start_variable_editor_draft(
        &variable,
        &[existing],
        WorkflowStartVariableEditorMode::Edit(0),
    )
    .expect("editing the same index should allow the name");

    let existing = draft("text-input");
    let other_existing = draft("number");
    let edit_error = validate_start_variable_editor_draft(
        &variable,
        &[existing, other_existing],
        WorkflowStartVariableEditorMode::Edit(1),
    )
    .expect_err("editing a different index should still reject duplicate");
    assert_eq!(edit_error, "变量名不能重复");
}

#[test]
fn validation_rejects_invalid_max_length_values() {
    let mut variable = draft("file-list");
    variable.max_length_input = "11".to_string();
    assert_eq!(validate_error(&variable), "最大上传数必须在 1 到 10 之间");

    let mut variable = draft("text-input");
    variable.max_length_input = "-1".to_string();
    assert_eq!(validate_error(&variable), "最大长度必须是非负整数");
}

#[test]
fn validation_rejects_select_without_options_duplicate_options_or_invalid_default() {
    let mut variable = draft("select");
    assert_eq!(validate_error(&variable), "下拉选项至少需要一个有效选项");

    variable.options = vec!["one".to_string(), "one".to_string()];
    assert_eq!(validate_error(&variable), "下拉选项不能重复");

    variable.options = vec![" one ".to_string(), "one".to_string()];
    assert_eq!(validate_error(&variable), "下拉选项不能重复");

    variable.options = vec!["one".to_string(), "two".to_string()];
    variable.default_value = "three".to_string();
    assert_eq!(validate_error(&variable), "默认值必须在下拉选项中");
}

#[test]
fn validation_rejects_invalid_number_default() {
    let mut variable = draft("number");
    variable.default_value = "not-number".to_string();

    assert_eq!(validate_error(&variable), "数字类型默认值必须是数字");
}

#[test]
fn validation_rejects_file_draft_without_required_file_settings() {
    let variable = draft("file");
    assert_eq!(validate_error(&variable), "至少选择一种支持的文件类型");

    let mut variable = valid_file_draft("file");
    variable.allowed_file_types = vec!["custom".to_string()];
    variable.allowed_file_extensions.clear();
    assert_eq!(validate_error(&variable), "选择自定义文件类型时，必须填写扩展名");

    let mut variable = valid_file_draft("file");
    variable.allowed_file_upload_methods.clear();
    assert_eq!(validate_error(&variable), "至少选择一种上传方式");
}

#[test]
fn validation_rejects_file_list_default_count_above_max() {
    let mut variable = valid_file_draft("file-list");
    variable.max_length_input = "1".to_string();
    variable.default_file_values = vec!["one.png".to_string(), "two.png".to_string()];

    assert_eq!(validate_error(&variable), "默认文件数量不能超过最大上传数");
}

#[test]
fn normalization_helpers_return_sorted_deduplicated_values() {
    assert_eq!(default_start_variable_allowed_file_types(), vec!["image".to_string()]);
    assert_eq!(
        default_start_variable_allowed_upload_methods(),
        vec!["local_file".to_string(), "remote_url".to_string()]
    );
    assert_eq!(
        normalize_file_upload_methods(vec![
            "remote_url".to_string(),
            "LOCAL_FILE".to_string(),
            "bad".to_string(),
            "local_file".to_string(),
        ]),
        vec!["local_file".to_string(), "remote_url".to_string()]
    );
    assert_eq!(
        normalize_file_extensions(vec![
            ".PNG".to_string(),
            " pdf ".to_string(),
            String::new(),
            "png".to_string(),
        ]),
        vec![".pdf".to_string(), ".png".to_string()]
    );
}

#[test]
fn supported_type_and_variable_name_helpers_enforce_contracts() {
    assert!(is_supported_start_variable_input_type("file-list"));
    assert!(!is_supported_start_variable_input_type("unknown"));
    assert!(is_valid_start_variable_name("_name1"));
    assert!(is_valid_start_variable_name("name_1"));
    assert!(!is_valid_start_variable_name(""));
    assert!(!is_valid_start_variable_name("1name"));
    assert!(!is_valid_start_variable_name("bad-name"));
}
