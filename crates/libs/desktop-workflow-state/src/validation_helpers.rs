use super::*;

pub fn parse_mapping_yaml(
    text: &str,
    label: &str,
) -> Result<Value, desktop_i18n::workflow::Feedback> {
    if text.trim().is_empty() {
        return Ok(Value::Mapping(Mapping::new()));
    }

    let value = serde_yaml::from_str::<Value>(text).map_err(|error| {
        desktop_i18n::workflow::Feedback::new(
            "%s YAML 解析失败: %s",
            vec![label.to_string(), error.to_string()],
        )
    })?;
    if value.is_mapping() {
        Ok(value)
    } else {
        Err(desktop_i18n::workflow::Feedback::new(
            "%s 必须是对象映射（YAML map）",
            vec![label.to_string()],
        ))
    }
}

pub fn value_yaml_for_editor(value: &Value) -> String {
    let yaml = serde_yaml::to_string(value).unwrap_or_else(|_| String::new());
    yaml.strip_prefix("---\n").unwrap_or(&yaml).to_string()
}

pub fn parse_http_value(
    input: &str,
    label: &str,
) -> Result<Value, desktop_i18n::workflow::Feedback> {
    if input.trim().is_empty() {
        return Ok(Value::String(String::new()));
    }
    serde_yaml::from_str(input).map_err(|_| {
        desktop_i18n::workflow::Feedback::new("%s必须是合法 YAML", vec![label.to_string()])
    })
}

pub fn parse_node_data_yaml_value(
    text: &str,
) -> Result<Value, desktop_i18n::workflow::Feedback> {
    if text.trim().is_empty() {
        return Ok(Value::Mapping(Mapping::new()));
    }

    let value = serde_yaml::from_str::<Value>(text).map_err(|error| {
        desktop_i18n::workflow::Feedback::new(
            "节点 data YAML 解析失败: %s",
            vec![error.to_string()],
        )
    })?;
    if value.is_mapping() {
        Ok(value)
    } else {
        Err(desktop_i18n::workflow::Feedback::new(
            "节点 data 必须是对象映射（YAML map）",
            vec![],
        ))
    }
}

pub fn parse_code_default_value_yaml(
    text: &str,
) -> Result<Value, desktop_i18n::workflow::Feedback> {
    if text.trim().is_empty() {
        return Ok(Value::Sequence(Vec::new()));
    }

    let value = serde_yaml::from_str::<Value>(text).map_err(|error| {
        desktop_i18n::workflow::Feedback::new(
            "代码节点 default_value YAML 解析失败: %s",
            vec![error.to_string()],
        )
    })?;
    if value.is_sequence() {
        Ok(value)
    } else {
        Err(desktop_i18n::workflow::Feedback::new(
            "代码节点 default_value 必须是数组（YAML sequence）",
            vec![],
        ))
    }
}

pub fn scalar_value_to_string(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        Value::Null => String::new(),
        _ => value_yaml_for_editor(value),
    }
}

pub fn default_start_variable_draft() -> WorkflowStartVariableDraft {
    let variable_name = generate_start_variable_name();
    WorkflowStartVariableDraft {
        raw_variable: yaml_map_for_state(vec![
            ("default", Value::String(String::new())),
            ("hide", Value::Bool(false)),
            ("hint", Value::String(String::new())),
            ("label", Value::String("新变量".to_string())),
            ("max_length", serde_yaml::to_value(48_u64).unwrap_or(Value::String("48".to_string()))),
            ("options", Value::Sequence(Vec::new())),
            ("placeholder", Value::String(String::new())),
            ("required", Value::Bool(true)),
            ("type", Value::String("text-input".to_string())),
            ("variable", Value::String(variable_name.clone())),
        ]),
        label: "新变量".to_string(),
        variable: variable_name,
        input_type: "text-input".to_string(),
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

pub fn default_if_else_case_draft() -> WorkflowIfElseCaseDraft {
    let case_id = generate_case_id();
    WorkflowIfElseCaseDraft {
        raw_case: yaml_map_for_state(vec![
            ("case_id", Value::String(case_id.clone())),
            ("conditions", Value::Sequence(vec![default_if_else_condition_value()])),
            ("id", Value::String(case_id.clone())),
            ("logical_operator", Value::String("and".to_string())),
        ]),
        case_id,
        logical_operator: "and".to_string(),
        conditions: vec![default_if_else_condition_draft()],
    }
}

pub fn default_if_else_condition_draft() -> WorkflowIfElseConditionDraft {
    WorkflowIfElseConditionDraft {
        raw_condition: default_if_else_condition_value(),
        variable_selector_input: String::new(),
        comparison_operator: "contains".to_string(),
        compare_value: String::new(),
        var_type: "string".to_string(),
    }
}

pub fn default_if_else_condition_value() -> Value {
    yaml_map_for_state(vec![
        ("comparison_operator", Value::String("contains".to_string())),
        ("id", Value::String(generate_condition_id())),
        ("value", Value::String(String::new())),
        ("varType", Value::String("string".to_string())),
        ("variable_selector", Value::Sequence(Vec::new())),
    ])
}

pub fn yaml_map_for_state(entries: Vec<(&str, Value)>) -> Value {
    let mut map = Mapping::new();
    for (key, value) in entries {
        map.insert(Value::String(key.to_string()), value);
    }
    Value::Mapping(map)
}
