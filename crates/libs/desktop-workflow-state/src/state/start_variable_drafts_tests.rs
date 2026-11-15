//! 开始变量草稿与 YAML 互转的测试，覆盖选择、文件和文件列表类型。
//! fixture 验证默认值清理、选项合并及最大数量钳制等兼容边界。
use super::*;

fn yaml_value(input: &str) -> Value {
    serde_yaml::from_str(input).expect("test YAML should parse")
}

fn mapping_field<'a>(value: &'a Value, key: &str) -> &'a Value {
    value
        .as_mapping()
        .and_then(|map| mapping_value(map, key))
        .unwrap_or_else(|| panic!("expected mapping field {key}"))
}

fn optional_mapping_field<'a>(value: &'a Value, key: &str) -> Option<&'a Value> {
    value.as_mapping().and_then(|map| mapping_value(map, key))
}

fn string_sequence(value: &Value) -> Vec<String> {
    value
        .as_sequence()
        .expect("expected YAML sequence")
        .iter()
        .map(|item| item.as_str().expect("expected sequence string").to_string())
        .collect()
}

fn base_start_variable(input_type: &str) -> WorkflowStartVariableDraft {
    WorkflowStartVariableDraft {
        raw_variable: yaml_map_for_state(vec![
            ("legacy", Value::String("kept".to_string())),
            ("options", Value::Sequence(vec![Value::String("stale".to_string())])),
        ]),
        label: "Label".to_string(),
        variable: "name".to_string(),
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
        placeholder: "Placeholder".to_string(),
        hint: "Hint".to_string(),
        max_length_input: String::new(),
    }
}

#[test]
fn build_start_variable_draft_reads_mapping_and_normalizes_file_fields() {
    let value = yaml_value(
        r#"
label: Upload
variable: avatar
type: file
required: true
hide: true
options:
  - ignored
allowed_file_types:
  - custom
  - IMAGE
allowed_file_extensions:
  - png
  - .JPG
  - png
allowed_file_upload_methods:
  - remote_url
  - unknown
  - local_file
default: https://example.test/avatar.png
placeholder: Pick one
hint: Public profile image
max_length: 99
"#,
    );

    let draft = build_start_variable_draft(&value);

    assert_eq!(draft.label, "Upload");
    assert_eq!(draft.variable, "avatar");
    assert_eq!(draft.input_type, "file");
    assert!(!draft.required);
    assert!(draft.hidden);
    assert!(draft.options.is_empty());
    assert_eq!(draft.allowed_file_types, vec!["custom", "image"]);
    assert_eq!(draft.allowed_file_extensions, vec![".jpg", ".png"]);
    assert_eq!(draft.allowed_file_extensions_input, ".jpg, .png");
    assert_eq!(draft.allowed_file_upload_methods, vec!["local_file", "remote_url"]);
    assert_eq!(draft.default_value, "https://example.test/avatar.png");
    assert_eq!(draft.default_file_values, vec!["https://example.test/avatar.png"]);
    assert_eq!(draft.placeholder, "Pick one");
    assert_eq!(draft.hint, "Public profile image");
    assert!(draft.max_length_input.is_empty());
    assert_eq!(mapping_field(&draft.raw_variable, "label").as_str(), Some("Upload"));
}

#[test]
fn build_start_variable_draft_uses_defaults_for_non_mapping_input() {
    let draft = build_start_variable_draft(&Value::String("not a map".to_string()));

    assert_eq!(draft.label, "");
    assert_eq!(draft.variable, "");
    assert_eq!(draft.input_type, "text-input");
    assert!(!draft.required);
    assert!(!draft.hidden);
    assert!(draft.raw_variable.is_mapping());
    assert_eq!(draft.max_length_input, "48");
}

#[test]
fn build_start_variable_draft_keeps_select_default_when_option_exists() {
    let value = yaml_value(
        r#"
label: Choice
variable: choice
type: select
options:
  - alpha
  - beta
default: beta
"#,
    );

    let draft = build_start_variable_draft(&value);

    assert_eq!(draft.options, vec!["alpha", "beta"]);
    assert_eq!(draft.default_value, "beta");
    assert!(draft.max_length_input.is_empty());
}

#[test]
fn build_start_variable_draft_clears_select_default_when_missing_from_options() {
    let value = yaml_value(
        r#"
label: Choice
variable: choice
type: select
options:
  - alpha
default: beta
"#,
    );

    let draft = build_start_variable_draft(&value);

    assert_eq!(draft.options, vec!["alpha"]);
    assert_eq!(draft.default_value, "");
}

#[test]
fn build_start_variable_draft_parses_file_list_defaults_and_clamps_max_length() {
    let value = yaml_value(
        r#"
label: Files
variable: files
type: file-list
allowed_file_types:
  - image
default:
  - https://example.test/a.png
  - https://example.test/a.png
  - https://example.test/b.png
max_length: 1
"#,
    );

    let draft = build_start_variable_draft(&value);

    assert_eq!(draft.default_file_values, vec!["https://example.test/a.png"]);
    assert_eq!(draft.max_length_input, "1");
    assert!(draft.default_value.contains("https://example.test/a.png"));
}

#[test]
fn build_start_variable_draft_accepts_scalar_file_list_default_as_single_file() {
    let value = yaml_value(
        r#"
label: Files
variable: files
type: file-list
default: " https://example.test/one.png "
"#,
    );

    let draft = build_start_variable_draft(&value);

    assert_eq!(draft.default_file_values, vec!["https://example.test/one.png"]);
    assert_eq!(draft.default_value, "- https://example.test/one.png\n");
}

#[test]
fn merge_start_variable_value_writes_select_options_and_removes_file_fields() {
    let mut variable = base_start_variable("select");
    variable.options = vec![" alpha ".to_string(), String::new(), "beta".to_string()];
    variable.allowed_file_types = vec!["custom".to_string()];
    variable.allowed_file_extensions_input = ".png".to_string();
    variable.allowed_file_upload_methods = vec!["remote_url".to_string()];
    variable.default_value = "beta".to_string();
    variable.max_length_input = "20".to_string();

    let value = merge_start_variable_value(&variable).expect("select variable should merge");

    assert_eq!(mapping_field(&value, "label").as_str(), Some("Label"));
    assert_eq!(mapping_field(&value, "variable").as_str(), Some("name"));
    assert_eq!(mapping_field(&value, "type").as_str(), Some("select"));
    assert_eq!(mapping_field(&value, "required").as_bool(), Some(true));
    assert_eq!(mapping_field(&value, "hide").as_bool(), Some(false));
    assert_eq!(mapping_field(&value, "placeholder").as_str(), Some("Placeholder"));
    assert_eq!(mapping_field(&value, "hint").as_str(), Some("Hint"));
    assert_eq!(string_sequence(mapping_field(&value, "options")), vec!["alpha", "beta"]);
    assert_eq!(mapping_field(&value, "default").as_str(), Some("beta"));
    assert_eq!(mapping_field(&value, "max_length").as_u64(), Some(20));
    assert_eq!(mapping_field(&value, "legacy").as_str(), Some("kept"));
    assert_eq!(optional_mapping_field(&value, "allowed_file_types"), None);
}

#[test]
fn merge_start_variable_value_clears_non_select_options_to_empty_sequence() {
    let mut variable = base_start_variable("paragraph");
    variable.default_value = "text".to_string();

    let value = merge_start_variable_value(&variable).expect("paragraph variable should merge");

    assert_eq!(mapping_field(&value, "default").as_str(), Some("text"));
    assert_eq!(string_sequence(mapping_field(&value, "options")), Vec::<String>::new());
}

#[test]
fn merge_start_variable_value_removes_empty_default_and_max_length() {
    let mut variable = base_start_variable("text-input");
    variable.raw_variable = yaml_map_for_state(vec![
        ("default", Value::String("old".to_string())),
        ("max_length", serde_yaml::to_value(48_u64).expect("u64 should serialize")),
    ]);

    let value = merge_start_variable_value(&variable).expect("empty values should merge");

    assert_eq!(optional_mapping_field(&value, "default"), None);
    assert_eq!(optional_mapping_field(&value, "max_length"), None);
    assert_eq!(string_sequence(mapping_field(&value, "options")), Vec::<String>::new());
}

#[test]
fn merge_start_variable_value_preserves_text_default_whitespace_and_zero_max_length() {
    let mut variable = base_start_variable("text-input");
    variable.default_value = "  padded text  ".to_string();
    variable.max_length_input = " 0 ".to_string();

    let value = merge_start_variable_value(&variable).expect("text variable should merge");

    assert_eq!(mapping_field(&value, "default").as_str(), Some("  padded text  "));
    assert_eq!(mapping_field(&value, "max_length").as_u64(), Some(0));
}

#[test]
fn merge_start_variable_value_serializes_number_defaults() {
    let mut integer_variable = base_start_variable("number");
    integer_variable.default_value = "42".to_string();
    let integer_value =
        merge_start_variable_value(&integer_variable).expect("integer default should merge");
    assert_eq!(mapping_field(&integer_value, "default").as_i64(), Some(42));

    let mut float_variable = base_start_variable("number");
    float_variable.default_value = "3.5".to_string();
    let float_value =
        merge_start_variable_value(&float_variable).expect("float default should merge");
    assert_eq!(mapping_field(&float_value, "default").as_f64(), Some(3.5));
}

#[test]
fn merge_start_variable_value_rejects_invalid_number_default() {
    let mut variable = base_start_variable("number");
    variable.default_value = "abc".to_string();

    let error = merge_start_variable_value(&variable).expect_err("invalid number should fail");

    assert_eq!(error, "数字类型默认值必须是数字");
}

#[test]
fn merge_start_variable_value_serializes_checkbox_defaults() {
    let mut true_variable = base_start_variable("checkbox");
    true_variable.default_value = "TRUE".to_string();
    let true_value =
        merge_start_variable_value(&true_variable).expect("true checkbox should merge");
    assert_eq!(mapping_field(&true_value, "default").as_bool(), Some(true));

    let mut false_variable = base_start_variable("checkbox");
    false_variable.default_value = "false".to_string();
    let false_value =
        merge_start_variable_value(&false_variable).expect("false checkbox should merge");
    assert_eq!(mapping_field(&false_value, "default").as_bool(), Some(false));
}

#[test]
fn merge_start_variable_value_rejects_invalid_checkbox_default() {
    let mut variable = base_start_variable("checkbox");
    variable.default_value = "yes".to_string();

    let error = merge_start_variable_value(&variable).expect_err("invalid checkbox should fail");

    assert_eq!(error, "复选框默认值只能是 true 或 false");
}

#[test]
fn merge_start_variable_value_serializes_file_constraints_and_default() {
    let mut variable = base_start_variable("file");
    variable.allowed_file_types = vec!["custom".to_string(), "image".to_string()];
    variable.allowed_file_extensions_input = ".PNG jpg\n.gif".to_string();
    variable.allowed_file_upload_methods = vec!["remote_url".to_string()];
    variable.default_value = "https://example.test/file.png".to_string();

    let value = merge_start_variable_value(&variable).expect("file variable should merge");

    assert_eq!(
        string_sequence(mapping_field(&value, "allowed_file_types")),
        vec!["custom", "image"]
    );
    assert_eq!(
        string_sequence(mapping_field(&value, "allowed_file_extensions")),
        vec![".gif", ".jpg", ".png"]
    );
    assert_eq!(
        string_sequence(mapping_field(&value, "allowed_file_upload_methods")),
        vec!["remote_url"]
    );
    assert_eq!(mapping_field(&value, "default").as_str(), Some("https://example.test/file.png"));
}

#[test]
fn merge_start_variable_value_serializes_file_list_defaults() {
    let mut variable = base_start_variable("file-list");
    variable.allowed_file_types = vec!["image".to_string()];
    variable.allowed_file_upload_methods = vec!["local_file".to_string()];
    variable.default_value = "ignored scalar default".to_string();
    variable.default_file_values =
        vec!["https://example.test/a.png".to_string(), "https://example.test/b.png".to_string()];

    let value = merge_start_variable_value(&variable).expect("file-list should merge");

    assert_eq!(
        string_sequence(mapping_field(&value, "default")),
        vec!["https://example.test/a.png", "https://example.test/b.png"]
    );
}

#[test]
fn merge_start_variable_value_removes_empty_file_list_default() {
    let mut variable = base_start_variable("file-list");
    variable.raw_variable = yaml_map_for_state(vec![(
        "default",
        Value::Sequence(vec![Value::String("old".to_string())]),
    )]);

    let value = merge_start_variable_value(&variable).expect("empty file-list should merge");

    assert_eq!(optional_mapping_field(&value, "default"), None);
}

#[test]
fn merge_start_variable_value_rejects_invalid_max_length() {
    let mut variable = base_start_variable("text-input");
    variable.max_length_input = "abc".to_string();

    let error = merge_start_variable_value(&variable).expect_err("invalid max_length should fail");

    assert_eq!(error, "开始节点变量 max_length 必须是非负整数");
}

#[test]
fn build_if_else_case_draft_reads_case_id_fallback_and_conditions() {
    let value = yaml_value(
        r#"
id: fallback-case
logical_operator: or
conditions:
  - variable_selector:
      - sys
      - user_id
    comparison_operator: is
    value: ada
    varType: string
"#,
    );

    let draft = build_if_else_case_draft(&value);

    assert_eq!(draft.case_id, "fallback-case");
    assert_eq!(draft.logical_operator, "or");
    assert_eq!(draft.conditions.len(), 1);
    assert_eq!(draft.conditions[0].variable_selector_input, "sys.user_id");
    assert_eq!(draft.conditions[0].comparison_operator, "is");
    assert_eq!(draft.conditions[0].compare_value, "ada");
    assert_eq!(draft.conditions[0].var_type, "string");
}

#[test]
fn build_if_else_case_draft_prefers_case_id_and_keeps_empty_conditions() {
    let value = yaml_value(
        r#"
case_id: primary-case
id: fallback-case
conditions: []
"#,
    );

    let draft = build_if_else_case_draft(&value);

    assert_eq!(draft.case_id, "primary-case");
    assert_eq!(draft.logical_operator, "and");
    assert!(draft.conditions.is_empty());
}

#[test]
fn build_if_else_case_draft_supplies_default_condition_for_non_mapping_input() {
    let draft = build_if_else_case_draft(&Value::Bool(true));

    assert_eq!(draft.case_id, "");
    assert_eq!(draft.logical_operator, "and");
    assert_eq!(draft.conditions.len(), 1);
    assert_eq!(draft.conditions[0].comparison_operator, "contains");
}

#[test]
fn merge_if_else_case_value_writes_ids_operator_and_conditions() {
    let condition = WorkflowIfElseConditionDraft {
        raw_condition: yaml_map_for_state(vec![]),
        variable_selector_input: "sys.query".to_string(),
        comparison_operator: "contains".to_string(),
        compare_value: "hello".to_string(),
        var_type: "string".to_string(),
    };
    let case = WorkflowIfElseCaseDraft {
        raw_case: yaml_map_for_state(vec![("legacy", Value::String("kept".to_string()))]),
        case_id: "case-a".to_string(),
        logical_operator: "or".to_string(),
        conditions: vec![condition],
    };

    let value = merge_if_else_case_value(&case).expect("case should merge");
    let conditions = mapping_field(&value, "conditions").as_sequence().expect("conditions");
    let condition_map = conditions[0].as_mapping().expect("condition map");

    assert_eq!(mapping_field(&value, "case_id").as_str(), Some("case-a"));
    assert_eq!(mapping_field(&value, "id").as_str(), Some("case-a"));
    assert_eq!(mapping_field(&value, "logical_operator").as_str(), Some("or"));
    assert_eq!(mapping_field(&value, "legacy").as_str(), Some("kept"));
    assert!(mapping_value(condition_map, "id").and_then(Value::as_str).is_some());
    assert_eq!(
        mapping_value(condition_map, "variable_selector"),
        Some(&yaml_value("[sys, query]"))
    );
}

#[test]
fn merge_if_else_case_value_inserts_default_condition_when_empty() {
    let case = WorkflowIfElseCaseDraft {
        raw_case: yaml_map_for_state(vec![]),
        case_id: "case-empty".to_string(),
        logical_operator: "and".to_string(),
        conditions: Vec::new(),
    };

    let value = merge_if_else_case_value(&case).expect("empty case should merge");
    let conditions = mapping_field(&value, "conditions").as_sequence().expect("conditions");

    assert_eq!(conditions.len(), 1);
    assert_eq!(
        conditions[0]
            .as_mapping()
            .and_then(|map| mapping_value(map, "comparison_operator"))
            .and_then(Value::as_str),
        Some("contains")
    );
}

#[test]
fn build_if_else_condition_draft_uses_defaults_for_missing_fields() {
    let draft = build_if_else_condition_draft(&Value::Null);

    assert_eq!(draft.variable_selector_input, "");
    assert_eq!(draft.comparison_operator, "contains");
    assert_eq!(draft.compare_value, "");
    assert_eq!(draft.var_type, "string");
    assert!(draft.raw_condition.is_mapping());
}

#[test]
fn build_if_else_condition_draft_stringifies_scalar_compare_value() {
    let value = yaml_value(
        r#"
variable_selector:
  - node
  - count
comparison_operator: ">="
value: 7
varType: number
"#,
    );

    let draft = build_if_else_condition_draft(&value);

    assert_eq!(draft.variable_selector_input, "node.count");
    assert_eq!(draft.comparison_operator, ">=");
    assert_eq!(draft.compare_value, "7");
    assert_eq!(draft.var_type, "number");
}

#[test]
fn merge_if_else_condition_value_generates_missing_id() {
    let condition = WorkflowIfElseConditionDraft {
        raw_condition: yaml_map_for_state(vec![]),
        variable_selector_input: "start.name".to_string(),
        comparison_operator: "is".to_string(),
        compare_value: "Ada".to_string(),
        var_type: "string".to_string(),
    };

    let value = merge_if_else_condition_value(&condition).expect("condition should merge");

    assert!(mapping_field(&value, "id").as_str().is_some_and(|id| id.starts_with("condition-")));
    assert_eq!(mapping_field(&value, "comparison_operator").as_str(), Some("is"));
    assert_eq!(mapping_field(&value, "variable_selector"), &yaml_value("[start, name]"));
}

#[test]
fn merge_if_else_condition_value_preserves_existing_id() {
    let condition = WorkflowIfElseConditionDraft {
        raw_condition: yaml_map_for_state(vec![("id", Value::String("condition-a".to_string()))]),
        variable_selector_input: "start.name".to_string(),
        comparison_operator: "contains".to_string(),
        compare_value: "Ada".to_string(),
        var_type: "string".to_string(),
    };

    let value = merge_if_else_condition_value(&condition).expect("condition should merge");

    assert_eq!(mapping_field(&value, "id").as_str(), Some("condition-a"));
    assert_eq!(mapping_field(&value, "comparison_operator").as_str(), Some("contains"));
    assert_eq!(mapping_field(&value, "value").as_str(), Some("Ada"));
    assert_eq!(mapping_field(&value, "varType").as_str(), Some("string"));
    assert_eq!(mapping_field(&value, "variable_selector"), &yaml_value("[start, name]"));
}

#[test]
fn scalar_value_to_string_handles_scalar_and_structured_values() {
    assert_eq!(scalar_value_to_string(&Value::String("text".to_string())), "text");
    assert_eq!(scalar_value_to_string(&Value::Bool(true)), "true");
    assert_eq!(
        scalar_value_to_string(&serde_yaml::to_value(12_u64).expect("u64 should serialize")),
        "12"
    );
    assert_eq!(scalar_value_to_string(&Value::Null), "");
    assert!(scalar_value_to_string(&yaml_value("- a\n- b\n")).contains("- a"));
}

#[test]
fn default_start_variable_draft_contains_stable_defaults() {
    let draft = default_start_variable_draft();

    assert_eq!(draft.label, "新变量");
    assert!(draft.variable.starts_with("input_"));
    assert_eq!(draft.input_type, "text-input");
    assert!(draft.required);
    assert!(!draft.hidden);
    assert_eq!(draft.max_length_input, "48");
    assert_eq!(mapping_field(&draft.raw_variable, "label").as_str(), Some("新变量"));
    assert_eq!(mapping_field(&draft.raw_variable, "required").as_bool(), Some(true));
    assert_eq!(mapping_field(&draft.raw_variable, "type").as_str(), Some("text-input"));
}
