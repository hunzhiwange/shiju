//! # Workflow YAML 辅助
//!
//! 该模块提供 YAML 映射、列表、提示词、变量值和类型校验等通用辅助方法。

use super::*;

pub(super) fn parse_mapping_yaml(
    text: &str,
    label: &str,
) -> Result<Value, crate::app::i18n::workflow::Feedback> {
    if text.trim().is_empty() {
        return Ok(Value::Mapping(Mapping::new()));
    }

    let value = serde_yaml::from_str::<Value>(text).map_err(|error| {
        crate::app::i18n::workflow::Feedback::new(
            "%s YAML 解析失败: %s",
            vec![label.to_string(), error.to_string()],
        )
    })?;
    if value.is_mapping() {
        Ok(value)
    } else {
        Err(crate::app::i18n::workflow::Feedback::new(
            "%s 必须是对象映射（YAML map）",
            vec![label.to_string()],
        ))
    }
}

pub(super) fn string_list_input_from_value(value: Option<&Value>) -> String {
    value
        .and_then(Value::as_sequence)
        .map(|items| items.iter().filter_map(Value::as_str).collect::<Vec<_>>().join(", "))
        .unwrap_or_default()
}

pub(super) fn string_list_from_value(value: &Value) -> Vec<String> {
    value
        .as_sequence()
        .map(|items| {
            items.iter().filter_map(Value::as_str).map(|item| item.to_string()).collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

pub(super) fn string_list_value_from_input(input: &str) -> Value {
    Value::Sequence(
        input
            .split(',')
            .map(str::trim)
            .filter(|item| !item.is_empty())
            .map(|item| Value::String(item.to_string()))
            .collect(),
    )
}

pub(super) fn prompt_text_by_role(prompt_template: Option<&Vec<Value>>, role: &str) -> String {
    prompt_template
        .and_then(|items| {
            items.iter().find_map(|item| {
                let map = item.as_mapping()?;
                let item_role = mapping_value(map, "role")?.as_str()?;
                if item_role == role {
                    mapping_value(map, "text").and_then(Value::as_str).map(|text| text.to_string())
                } else {
                    None
                }
            })
        })
        .unwrap_or_default()
}

pub(super) fn merge_prompt_template_value(
    existing: Value,
    system_text: String,
    user_text: String,
) -> Value {
    let mut items = existing.as_sequence().cloned().unwrap_or_default();
    upsert_prompt_template_item(&mut items, "system", &system_text, true);
    upsert_prompt_template_item(&mut items, "user", &user_text, false);
    if items.is_empty() {
        upsert_prompt_template_item(&mut items, "system", "", true);
    }
    Value::Sequence(items)
}

fn upsert_prompt_template_item(
    items: &mut Vec<Value>,
    role: &str,
    text: &str,
    create_if_missing: bool,
) {
    if let Some(item) = items.iter_mut().find(|item| {
        item.as_mapping().and_then(|map| mapping_value(map, "role")).and_then(Value::as_str)
            == Some(role)
    }) {
        if let Some(map) = item.as_mapping_mut() {
            set_mapping_string(map, "role", role);
            set_mapping_string(map, "text", text);
            if !map.contains_key(&yaml_key("id")) {
                set_mapping_string(map, "id", &generate_prompt_item_id(role));
            }
        }
        return;
    }

    if !create_if_missing && text.trim().is_empty() {
        return;
    }

    let mut map = Mapping::new();
    set_mapping_string(&mut map, "id", &generate_prompt_item_id(role));
    set_mapping_string(&mut map, "role", role);
    set_mapping_string(&mut map, "text", text);
    items.push(Value::Mapping(map));
}

pub(super) fn ensure_mapping_entry<'a>(map: &'a mut Mapping, key: &str) -> &'a mut Mapping {
    let value = map.entry(yaml_key(key)).or_insert_with(|| Value::Mapping(Mapping::new()));
    if !value.is_mapping() {
        *value = Value::Mapping(Mapping::new());
    }
    value.as_mapping_mut().expect("mapping just initialized")
}

pub(super) fn mapping_value<'a>(map: &'a Mapping, key: &str) -> Option<&'a Value> {
    map.get(&yaml_key(key))
}

pub(super) fn set_mapping_string(map: &mut Mapping, key: &str, value: &str) {
    map.insert(yaml_key(key), Value::String(value.to_string()));
}

pub(super) fn set_mapping_bool(map: &mut Mapping, key: &str, value: bool) {
    map.insert(yaml_key(key), Value::Bool(value));
}

pub(super) fn yaml_key(key: &str) -> Value {
    Value::String(key.to_string())
}

pub(super) fn value_yaml_for_editor(value: &Value) -> String {
    let yaml = serde_yaml::to_string(value).unwrap_or_else(|_| String::new());
    yaml.strip_prefix("---\n").unwrap_or(&yaml).to_string()
}

pub(super) fn parse_yaml_editor_value(
    text: &str,
) -> Result<Value, crate::app::i18n::workflow::Feedback> {
    if text.trim().is_empty() {
        return Ok(Value::String(String::new()));
    }

    serde_yaml::from_str::<Value>(text).map_err(|error| {
        crate::app::i18n::workflow::Feedback::new(
            "变量值 YAML 解析失败: %s",
            vec![error.to_string()],
        )
    })
}

pub(super) fn normalize_environment_value_type(
    value_type: &str,
) -> Result<String, crate::app::i18n::workflow::Feedback> {
    let normalized = value_type.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "string" | "number" | "secret" => Ok(normalized),
        _ => Err(crate::app::i18n::workflow::Feedback::new(
            "环境变量类型仅支持 string / number / secret",
            vec![],
        )),
    }
}

pub(super) fn normalize_conversation_value_type(
    value_type: &str,
) -> Result<String, crate::app::i18n::workflow::Feedback> {
    let normalized = value_type.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        Err(crate::app::i18n::workflow::Feedback::new("会话变量类型不能为空", vec![]))
    } else {
        Ok(normalized)
    }
}

pub(super) fn validate_environment_value(
    value_type: &str,
    value: &Value,
) -> Result<(), crate::app::i18n::workflow::Feedback> {
    match (value_type, value) {
        ("string" | "secret", Value::String(_)) => Ok(()),
        ("number", Value::Number(_)) => Ok(()),
        ("string", _) => Err(crate::app::i18n::workflow::Feedback::new(
            "string 类型环境变量的值必须是字符串 YAML 标量",
            vec![],
        )),
        ("secret", _) => Err(crate::app::i18n::workflow::Feedback::new(
            "secret 类型环境变量的值必须是字符串 YAML 标量",
            vec![],
        )),
        ("number", _) => Err(crate::app::i18n::workflow::Feedback::new(
            "number 类型环境变量的值必须是数字 YAML 标量",
            vec![],
        )),
        _ => Err(crate::app::i18n::workflow::Feedback::new("不支持的环境变量类型", vec![])),
    }
}

pub(super) fn ensure_unique_variable_name<T>(
    variables: &[T],
    name: &str,
    current_id: Option<&str>,
    label: &str,
) -> Result<(), crate::app::i18n::workflow::Feedback>
where
    T: WorkflowVariableNameAccess,
{
    if variables.iter().any(|variable| variable.name() == name && Some(variable.id()) != current_id)
    {
        Err(crate::app::i18n::workflow::Feedback::new("%s名称不能重复", vec![label.to_string()]))
    } else {
        Ok(())
    }
}

pub(super) trait WorkflowVariableNameAccess {
    fn id(&self) -> &str;
    fn name(&self) -> &str;
}

impl WorkflowVariableNameAccess for WorkflowEnvironmentVariable {
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        &self.name
    }
}

impl WorkflowVariableNameAccess for WorkflowConversationVariable {
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        &self.name
    }
}

#[cfg(test)]
#[path = "yaml_helpers_tests.rs"]
mod yaml_helpers_tests;
