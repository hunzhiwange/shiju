//! # Workflow 开始变量草稿
//!
//! 该模块负责开始节点输入变量的草稿解析、回填 YAML 以及默认草稿生成。

use super::*;

pub(super) fn build_start_variable_draft(value: &Value) -> WorkflowStartVariableDraft {
    let raw_variable = ensure_root_mapping(value.clone());
    let variable_map = raw_variable.as_mapping();
    let input_type = variable_map
        .and_then(|map| mapping_value(map, "type"))
        .and_then(Value::as_str)
        .unwrap_or("text-input")
        .to_string();

    let mut draft = WorkflowStartVariableDraft {
        label: variable_map
            .and_then(|map| mapping_value(map, "label"))
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        variable: variable_map
            .and_then(|map| mapping_value(map, "variable"))
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        input_type: input_type.clone(),
        required: variable_map
            .and_then(|map| mapping_value(map, "required"))
            .and_then(Value::as_bool)
            .unwrap_or(false),
        hidden: variable_map
            .and_then(|map| mapping_value(map, "hide"))
            .and_then(Value::as_bool)
            .unwrap_or(false),
        options: variable_map
            .and_then(|map| mapping_value(map, "options"))
            .map(string_list_from_value)
            .unwrap_or_default(),
        allowed_file_types: variable_map
            .and_then(|map| mapping_value(map, "allowed_file_types"))
            .map(string_list_from_value)
            .unwrap_or_default(),
        allowed_file_extensions: variable_map
            .and_then(|map| mapping_value(map, "allowed_file_extensions"))
            .map(string_list_from_value)
            .unwrap_or_default(),
        allowed_file_extensions_input: variable_map
            .and_then(|map| mapping_value(map, "allowed_file_extensions"))
            .map(|value| string_list_from_value(value).join(", "))
            .unwrap_or_default(),
        allowed_file_upload_methods: variable_map
            .and_then(|map| mapping_value(map, "allowed_file_upload_methods"))
            .map(string_list_from_value)
            .unwrap_or_default(),
        default_value: variable_map
            .and_then(|map| mapping_value(map, "default"))
            .map(scalar_value_to_string)
            .unwrap_or_default(),
        default_file_values: parse_start_variable_file_default_values(
            &input_type,
            variable_map.and_then(|map| mapping_value(map, "default")),
        ),
        placeholder: variable_map
            .and_then(|map| mapping_value(map, "placeholder"))
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        hint: variable_map
            .and_then(|map| mapping_value(map, "hint"))
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        max_length_input: variable_map
            .and_then(|map| mapping_value(map, "max_length"))
            .map(scalar_value_to_string)
            .unwrap_or_default(),
        raw_variable,
    };

    normalize_start_variable_draft(&mut draft);
    draft
}

pub(super) fn merge_start_variable_value(
    variable: &WorkflowStartVariableDraft,
) -> Result<Value, crate::app::i18n::workflow::Feedback> {
    let mut raw = ensure_root_mapping(variable.raw_variable.clone());
    let variable_map = raw.as_mapping_mut().ok_or_else(|| {
        crate::app::i18n::workflow::Feedback::new("开始节点变量必须是对象映射（YAML map）", vec![])
    })?;

    set_mapping_string(variable_map, "label", &variable.label);
    set_mapping_string(variable_map, "variable", &variable.variable);
    set_mapping_string(variable_map, "type", &variable.input_type);
    set_mapping_bool(variable_map, "required", variable.required);
    set_mapping_bool(variable_map, "hide", variable.hidden);
    set_mapping_string(variable_map, "placeholder", &variable.placeholder);
    set_mapping_string(variable_map, "hint", &variable.hint);

    match variable.input_type.as_str() {
        "select" => {
            variable_map.insert(
                yaml_key("options"),
                Value::Sequence(
                    variable
                        .options
                        .iter()
                        .map(|option| option.trim())
                        .filter(|option| !option.is_empty())
                        .map(|option| Value::String(option.to_string()))
                        .collect(),
                ),
            );
        }
        _ => {
            variable_map.remove(&yaml_key("options"));
        }
    }

    if matches!(variable.input_type.as_str(), "file" | "file-list") {
        let normalized_extensions = normalize_file_extensions(
            variable
                .allowed_file_extensions_input
                .split(|ch: char| ch == ',' || ch.is_whitespace())
                .map(str::trim)
                .filter(|item| !item.is_empty())
                .map(|item| item.to_string())
                .collect(),
        );

        variable_map.insert(
            yaml_key("allowed_file_types"),
            Value::Sequence(
                variable
                    .allowed_file_types
                    .iter()
                    .map(|item: &String| Value::String(item.clone()))
                    .collect(),
            ),
        );
        variable_map.insert(
            yaml_key("allowed_file_extensions"),
            Value::Sequence(
                normalized_extensions.iter().map(|item| Value::String(item.clone())).collect(),
            ),
        );
        variable_map.insert(
            yaml_key("allowed_file_upload_methods"),
            Value::Sequence(
                variable
                    .allowed_file_upload_methods
                    .iter()
                    .map(|item| Value::String(item.clone()))
                    .collect(),
            ),
        );
    } else {
        variable_map.remove(&yaml_key("allowed_file_types"));
        variable_map.remove(&yaml_key("allowed_file_extensions"));
        variable_map.remove(&yaml_key("allowed_file_upload_methods"));
    }

    let default_value = variable.default_value.trim();
    if variable.input_type == "file-list" {
        if variable.default_file_values.is_empty() {
            variable_map.remove(&yaml_key("default"));
        } else {
            variable_map.insert(
                yaml_key("default"),
                Value::Sequence(
                    variable
                        .default_file_values
                        .iter()
                        .map(|item| Value::String(item.clone()))
                        .collect(),
                ),
            );
        }
    } else if default_value.is_empty() {
        variable_map.remove(&yaml_key("default"));
    } else {
        let default_yaml = match variable.input_type.as_str() {
            "number" => {
                if let Ok(number) = default_value.parse::<i64>() {
                    serde_yaml::to_value(number).map_err(|error| {
                        crate::app::i18n::workflow::Feedback::new(
                            "开始节点变量默认数字序列化失败: %s",
                            vec![error.to_string()],
                        )
                    })?
                } else {
                    let number = default_value.parse::<f64>().map_err(|_| {
                        crate::app::i18n::workflow::Feedback::new(
                            "数字类型默认值必须是数字",
                            vec![],
                        )
                    })?;
                    serde_yaml::to_value(number).map_err(|error| {
                        crate::app::i18n::workflow::Feedback::new(
                            "开始节点变量默认数字序列化失败: %s",
                            vec![error.to_string()],
                        )
                    })?
                }
            }
            "checkbox" => match default_value.to_ascii_lowercase().as_str() {
                "true" => Value::Bool(true),
                "false" => Value::Bool(false),
                _ => {
                    return Err(crate::app::i18n::workflow::Feedback::new(
                        "复选框默认值只能是 true 或 false",
                        vec![],
                    ));
                }
            },
            _ => Value::String(variable.default_value.clone()),
        };
        variable_map.insert(yaml_key("default"), default_yaml);
    }

    if variable.max_length_input.trim().is_empty() {
        variable_map.remove(&yaml_key("max_length"));
    } else {
        let max_length = variable.max_length_input.trim().parse::<u64>().map_err(|_| {
            crate::app::i18n::workflow::Feedback::new(
                "开始节点变量 max_length 必须是非负整数",
                vec![],
            )
        })?;
        variable_map.insert(
            yaml_key("max_length"),
            serde_yaml::to_value(max_length).map_err(|error| {
                crate::app::i18n::workflow::Feedback::new(
                    "开始节点变量 max_length 序列化失败: %s",
                    vec![error.to_string()],
                )
            })?,
        );
    }

    if !variable_map.contains_key(&yaml_key("options")) {
        variable_map.insert(yaml_key("options"), Value::Sequence(Vec::new()));
    }

    Ok(raw)
}

pub(super) fn build_if_else_case_draft(value: &Value) -> WorkflowIfElseCaseDraft {
    let raw_case = ensure_root_mapping(value.clone());
    let case_map = raw_case.as_mapping();

    WorkflowIfElseCaseDraft {
        case_id: case_map
            .and_then(|map| mapping_value(map, "case_id"))
            .or_else(|| case_map.and_then(|map| mapping_value(map, "id")))
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        logical_operator: case_map
            .and_then(|map| mapping_value(map, "logical_operator"))
            .and_then(Value::as_str)
            .unwrap_or("and")
            .to_string(),
        conditions: case_map
            .and_then(|map| mapping_value(map, "conditions"))
            .and_then(Value::as_sequence)
            .map(|conditions: &Vec<Value>| {
                conditions.iter().map(build_if_else_condition_draft).collect::<Vec<_>>()
            })
            .unwrap_or_else(|| vec![default_if_else_condition_draft()]),
        raw_case,
    }
}

pub(super) fn merge_if_else_case_value(
    case: &WorkflowIfElseCaseDraft,
) -> Result<Value, crate::app::i18n::workflow::Feedback> {
    let mut raw_case = ensure_root_mapping(case.raw_case.clone());
    let case_map = raw_case.as_mapping_mut().ok_or_else(|| {
        crate::app::i18n::workflow::Feedback::new(
            "条件分支 case 必须是对象映射（YAML map）",
            vec![],
        )
    })?;

    set_mapping_string(case_map, "case_id", &case.case_id);
    set_mapping_string(case_map, "id", &case.case_id);
    set_mapping_string(case_map, "logical_operator", &case.logical_operator);

    let conditions = if case.conditions.is_empty() {
        vec![merge_if_else_condition_value(&default_if_else_condition_draft())?]
    } else {
        case.conditions.iter().map(merge_if_else_condition_value).collect::<Result<Vec<_>, _>>()?
    };

    case_map.insert(yaml_key("conditions"), Value::Sequence(conditions));
    Ok(raw_case)
}

pub(super) fn build_if_else_condition_draft(value: &Value) -> WorkflowIfElseConditionDraft {
    let raw_condition = ensure_root_mapping(value.clone());
    let condition_map = raw_condition.as_mapping();

    WorkflowIfElseConditionDraft {
        variable_selector_input: condition_map
            .and_then(|map| mapping_value(map, "variable_selector"))
            .map(|value| selector_path_input_from_value(Some(value)))
            .unwrap_or_default(),
        comparison_operator: condition_map
            .and_then(|map| mapping_value(map, "comparison_operator"))
            .and_then(Value::as_str)
            .unwrap_or("contains")
            .to_string(),
        compare_value: condition_map
            .and_then(|map| mapping_value(map, "value"))
            .map(scalar_value_to_string)
            .unwrap_or_default(),
        var_type: condition_map
            .and_then(|map| mapping_value(map, "varType"))
            .and_then(Value::as_str)
            .unwrap_or("string")
            .to_string(),
        raw_condition,
    }
}

pub(super) fn merge_if_else_condition_value(
    condition: &WorkflowIfElseConditionDraft,
) -> Result<Value, crate::app::i18n::workflow::Feedback> {
    let mut raw_condition = ensure_root_mapping(condition.raw_condition.clone());
    let condition_map = raw_condition.as_mapping_mut().ok_or_else(|| {
        crate::app::i18n::workflow::Feedback::new(
            "条件分支 condition 必须是对象映射（YAML map）",
            vec![],
        )
    })?;
    if !condition_map.contains_key(&yaml_key("id")) {
        set_mapping_string(condition_map, "id", &generate_condition_id());
    }
    set_mapping_string(condition_map, "comparison_operator", &condition.comparison_operator);
    set_mapping_string(condition_map, "value", &condition.compare_value);
    set_mapping_string(condition_map, "varType", &condition.var_type);
    condition_map.insert(
        yaml_key("variable_selector"),
        selector_path_value_from_input(&condition.variable_selector_input),
    );
    Ok(raw_condition)
}

pub(super) fn scalar_value_to_string(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        Value::Null => String::new(),
        _ => value_yaml_for_editor(value),
    }
}

pub(super) fn default_start_variable_draft() -> WorkflowStartVariableDraft {
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

#[cfg(test)]
#[path = "start_variable_drafts_tests.rs"]
mod start_variable_drafts_tests;
