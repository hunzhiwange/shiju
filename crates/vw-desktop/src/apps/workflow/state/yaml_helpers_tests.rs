use super::*;

fn mapping_value_str<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.as_mapping().and_then(|map| mapping_value(map, key)).and_then(Value::as_str)
}

fn sequence_items(value: &Value) -> &[Value] {
    value.as_sequence().expect("value should be a sequence")
}

fn environment_variable(id: &str, name: &str) -> WorkflowEnvironmentVariable {
    WorkflowEnvironmentVariable {
        id: id.to_string(),
        name: name.to_string(),
        value_type: "string".to_string(),
        value: Value::String("value".to_string()),
        description: String::new(),
        raw_variable: Value::Null,
    }
}

fn conversation_variable(id: &str, name: &str) -> WorkflowConversationVariable {
    WorkflowConversationVariable {
        id: id.to_string(),
        name: name.to_string(),
        value_type: "string".to_string(),
        value: Value::String("value".to_string()),
        description: String::new(),
        raw_variable: Value::Null,
    }
}

#[test]
fn parse_mapping_yaml_accepts_empty_and_mapping_input() {
    let empty = parse_mapping_yaml(" \n\t", "工具参数").expect("empty input should become map");
    assert_eq!(empty, Value::Mapping(Mapping::new()));

    let value = parse_mapping_yaml("enabled: true\ncount: 2\n", "工具参数")
        .expect("mapping yaml should parse");
    let map = value.as_mapping().expect("parsed value should be a map");
    assert_eq!(mapping_value(map, "enabled"), Some(&Value::Bool(true)));
    assert_eq!(mapping_value(map, "count").and_then(Value::as_i64), Some(2));
}

#[test]
fn parse_mapping_yaml_rejects_invalid_yaml_and_non_mapping_values() {
    let parse_error = parse_mapping_yaml("[", "工具参数").expect_err("invalid yaml should fail");
    assert!(parse_error.contains("工具参数 YAML 解析失败"));

    let type_error = parse_mapping_yaml("- item\n", "工具参数")
        .expect_err("sequence yaml should not be accepted as a map");
    assert_eq!(type_error, "工具参数 必须是对象映射（YAML map）");
}

#[test]
fn string_list_helpers_ignore_non_string_items() {
    let value = serde_yaml::from_str::<Value>("- alpha\n- 7\n- beta\n- false\n")
        .expect("list yaml should parse");

    assert_eq!(string_list_input_from_value(Some(&value)), "alpha, beta");
    assert_eq!(string_list_from_value(&value), vec!["alpha".to_string(), "beta".to_string()]);
    assert_eq!(string_list_input_from_value(None), "");
    assert_eq!(string_list_from_value(&Value::String("alpha".to_string())), Vec::<String>::new());
}

#[test]
fn string_list_value_from_input_trims_and_drops_empty_parts() {
    let value = string_list_value_from_input(" alpha, , beta ,, gamma ");
    assert_eq!(
        value,
        Value::Sequence(vec![
            Value::String("alpha".to_string()),
            Value::String("beta".to_string()),
            Value::String("gamma".to_string()),
        ])
    );
}

#[test]
fn prompt_text_by_role_reads_matching_mapping_text_only() {
    let prompt_template = vec![
        Value::String("ignored".to_string()),
        serde_yaml::from_str::<Value>("role: system\ntext: system prompt\n")
            .expect("prompt item should parse"),
        serde_yaml::from_str::<Value>("role: user\ntext: user prompt\n")
            .expect("prompt item should parse"),
        serde_yaml::from_str::<Value>("role: assistant\ntext:\n  - ignored\n")
            .expect("prompt item should parse"),
    ];

    assert_eq!(prompt_text_by_role(Some(&prompt_template), "system"), "system prompt");
    assert_eq!(prompt_text_by_role(Some(&prompt_template), "assistant"), "");
    assert_eq!(prompt_text_by_role(None, "user"), "");
}

#[test]
fn prompt_text_by_role_skips_items_without_string_role_or_text() {
    let prompt_template = vec![
        serde_yaml::from_str::<Value>("text: missing role\n").expect("prompt item should parse"),
        serde_yaml::from_str::<Value>("role: system\ntext:\n  - not text\n")
            .expect("prompt item should parse"),
        serde_yaml::from_str::<Value>("role: user\ntext: user prompt\n")
            .expect("prompt item should parse"),
    ];

    assert_eq!(prompt_text_by_role(Some(&prompt_template), "system"), "");
    assert_eq!(prompt_text_by_role(Some(&prompt_template), "user"), "user prompt");
}

#[test]
fn prompt_text_by_role_continues_after_matching_item_without_string_text() {
    let prompt_template = vec![
        serde_yaml::from_str::<Value>("role: system\ntext:\n  - invalid\n")
            .expect("prompt item should parse"),
        serde_yaml::from_str::<Value>("role: system\ntext: valid system prompt\n")
            .expect("prompt item should parse"),
    ];

    assert_eq!(prompt_text_by_role(Some(&prompt_template), "system"), "valid system prompt");
}

#[test]
fn string_list_input_from_value_returns_empty_for_non_sequence() {
    assert_eq!(string_list_input_from_value(Some(&Value::String("not a list".to_string()))), "");
}

#[test]
fn merge_prompt_template_value_updates_existing_items_and_creates_missing_user() {
    let existing = serde_yaml::from_str::<Value>(
        r#"
- id: existing-system
  role: system
  text: old system
  keep: yes
- role: user
  text: old user
- role: assistant
  text: untouched
"#,
    )
    .expect("existing prompt template should parse");

    let merged =
        merge_prompt_template_value(existing, "new system".to_string(), "new user".to_string());
    let items = sequence_items(&merged);
    assert_eq!(items.len(), 3);
    assert_eq!(mapping_value_str(&items[0], "id"), Some("existing-system"));
    assert_eq!(mapping_value_str(&items[0], "text"), Some("new system"));
    assert_eq!(mapping_value_str(&items[0], "keep"), Some("yes"));
    assert!(mapping_value_str(&items[1], "id").is_some());
    assert_eq!(mapping_value_str(&items[1], "role"), Some("user"));
    assert_eq!(mapping_value_str(&items[1], "text"), Some("new user"));
    assert_eq!(mapping_value_str(&items[2], "text"), Some("untouched"));
}

#[test]
fn merge_prompt_template_value_handles_scalar_existing_and_blank_user() {
    let merged = merge_prompt_template_value(
        Value::String("not a sequence".to_string()),
        String::new(),
        String::new(),
    );
    let items = sequence_items(&merged);
    assert_eq!(items.len(), 1);
    assert!(mapping_value_str(&items[0], "id").is_some());
    assert_eq!(mapping_value_str(&items[0], "role"), Some("system"));
    assert_eq!(mapping_value_str(&items[0], "text"), Some(""));

    let user_created = merge_prompt_template_value(
        Value::Sequence(Vec::new()),
        "system".to_string(),
        "user".to_string(),
    );
    assert_eq!(sequence_items(&user_created).len(), 2);
}

#[test]
fn merge_prompt_template_value_keeps_blank_user_absent_when_system_item_exists() {
    let existing = serde_yaml::from_str::<Value>(
        r#"
- id: system-id
  role: system
  text: old
"#,
    )
    .expect("existing prompt template should parse");

    let merged = merge_prompt_template_value(existing, "updated".to_string(), "   ".to_string());
    let items = sequence_items(&merged);

    assert_eq!(items.len(), 1);
    assert_eq!(mapping_value_str(&items[0], "id"), Some("system-id"));
    assert_eq!(mapping_value_str(&items[0], "role"), Some("system"));
    assert_eq!(mapping_value_str(&items[0], "text"), Some("updated"));
}

#[test]
fn mapping_helpers_set_read_and_replace_nested_maps() {
    let mut map = Mapping::new();
    set_mapping_string(&mut map, "title", "Workflow");
    set_mapping_bool(&mut map, "enabled", true);

    assert_eq!(mapping_value(&map, "title").and_then(Value::as_str), Some("Workflow"));
    assert_eq!(mapping_value(&map, "enabled").and_then(Value::as_bool), Some(true));
    assert_eq!(mapping_value(&map, "missing"), None);
    assert_eq!(yaml_key("title"), Value::String("title".to_string()));

    map.insert(yaml_key("nested"), Value::String("replace me".to_string()));
    let nested = ensure_mapping_entry(&mut map, "nested");
    set_mapping_string(nested, "child", "value");
    assert_eq!(
        map.get(yaml_key("nested"))
            .and_then(Value::as_mapping)
            .and_then(|nested_map| mapping_value(nested_map, "child"))
            .and_then(Value::as_str),
        Some("value")
    );

    let created = ensure_mapping_entry(&mut map, "created");
    assert!(created.is_empty());
}

#[test]
fn yaml_editor_helpers_serialize_without_document_marker_and_parse_values() {
    let value = serde_yaml::from_str::<Value>("name: Ada\nitems:\n  - one\n")
        .expect("mapping yaml should parse");
    let yaml = value_yaml_for_editor(&value);
    assert!(!yaml.starts_with("---"));
    assert!(yaml.contains("name: Ada"));

    assert_eq!(
        parse_yaml_editor_value(" \n").expect("empty editor should parse"),
        Value::String(String::new())
    );
    assert_eq!(parse_yaml_editor_value("42").expect("number should parse").as_i64(), Some(42));
    let error = parse_yaml_editor_value("[").expect_err("invalid yaml should fail");
    assert!(error.contains("变量值 YAML 解析失败"));
}

#[test]
fn value_yaml_for_editor_preserves_scalar_yaml_without_document_marker() {
    assert_eq!(value_yaml_for_editor(&Value::String("plain".to_string())), "plain\n");
    assert_eq!(value_yaml_for_editor(&Value::Bool(true)), "true\n");
}

#[test]
fn environment_value_type_normalization_accepts_supported_types_only() {
    assert_eq!(
        normalize_environment_value_type(" String ").expect("string should normalize"),
        "string"
    );
    assert_eq!(
        normalize_environment_value_type("NUMBER").expect("number should normalize"),
        "number"
    );
    assert_eq!(
        normalize_environment_value_type("secret").expect("secret should normalize"),
        "secret"
    );
    assert_eq!(
        normalize_environment_value_type("boolean").expect_err("unsupported type should fail"),
        "环境变量类型仅支持 string / number / secret"
    );
}

#[test]
fn conversation_value_type_normalization_rejects_blank_type() {
    assert_eq!(
        normalize_conversation_value_type(" Object ").expect("non-empty type should normalize"),
        "object"
    );
    assert_eq!(
        normalize_conversation_value_type(" \n").expect_err("blank type should fail"),
        "会话变量类型不能为空"
    );
}

#[test]
fn validate_environment_value_checks_type_specific_yaml_values() {
    assert!(validate_environment_value("string", &Value::String("value".to_string())).is_ok());
    assert!(validate_environment_value("secret", &Value::String("value".to_string())).is_ok());
    assert!(validate_environment_value("number", &serde_yaml::to_value(3).unwrap()).is_ok());

    assert_eq!(
        validate_environment_value("string", &Value::Bool(true)).expect_err("string mismatch"),
        "string 类型环境变量的值必须是字符串 YAML 标量"
    );
    assert_eq!(
        validate_environment_value("secret", &Value::Bool(true)).expect_err("secret mismatch"),
        "secret 类型环境变量的值必须是字符串 YAML 标量"
    );
    assert_eq!(
        validate_environment_value("number", &Value::String("3".to_string()))
            .expect_err("number mismatch"),
        "number 类型环境变量的值必须是数字 YAML 标量"
    );
    assert_eq!(
        validate_environment_value("boolean", &Value::Bool(true)).expect_err("unsupported type"),
        "不支持的环境变量类型"
    );
}

#[test]
fn ensure_unique_variable_name_allows_current_id_and_rejects_other_duplicates() {
    let environment_variables =
        vec![environment_variable("env-1", "api_key"), environment_variable("env-2", "region")];
    assert!(
        ensure_unique_variable_name(&environment_variables, "api_key", Some("env-1"), "环境变量")
            .is_ok()
    );
    assert!(
        ensure_unique_variable_name(&environment_variables, "new_name", None, "环境变量").is_ok()
    );
    assert_eq!(
        ensure_unique_variable_name(&environment_variables, "api_key", Some("env-2"), "环境变量")
            .expect_err("duplicate name should fail"),
        "环境变量名称不能重复"
    );

    let conversation_variables = vec![conversation_variable("conv-1", "topic")];
    assert_eq!(
        ensure_unique_variable_name(&conversation_variables, "topic", None, "会话变量")
            .expect_err("duplicate conversation name should fail"),
        "会话变量名称不能重复"
    );
}

#[test]
fn variable_name_access_returns_environment_and_conversation_fields() {
    let environment = environment_variable("env-1", "api_key");
    let conversation = conversation_variable("conv-1", "topic");

    assert_eq!(WorkflowVariableNameAccess::id(&environment), "env-1");
    assert_eq!(WorkflowVariableNameAccess::name(&environment), "api_key");
    assert_eq!(WorkflowVariableNameAccess::id(&conversation), "conv-1");
    assert_eq!(WorkflowVariableNameAccess::name(&conversation), "topic");
}
