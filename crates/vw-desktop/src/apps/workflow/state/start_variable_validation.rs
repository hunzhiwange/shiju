//! # Workflow 开始变量校验
//!
//! 该模块规范化并校验开始节点输入变量草稿，覆盖类型、选项和文件限制等规则。

use super::*;

const START_VARIABLE_FILE_LIST_MIN_UPLOAD_COUNT: u8 = 1;
const START_VARIABLE_FILE_LIST_MAX_UPLOAD_COUNT: u8 = 10;
const START_VARIABLE_FILE_LIST_DEFAULT_UPLOAD_COUNT: u8 = 5;

pub(in crate::apps::workflow) fn is_valid_start_variable_number_default_value(
    default_value: &str,
) -> bool {
    let trimmed = default_value.trim();
    trimmed.is_empty() || trimmed.parse::<i64>().is_ok() || trimmed.parse::<f64>().is_ok()
}

pub(super) fn parse_start_variable_file_default_values(
    input_type: &str,
    default_value: Option<&Value>,
) -> Vec<String> {
    let mut values = match (input_type, default_value) {
        ("file-list", Some(Value::Sequence(items))) => items
            .iter()
            .map(scalar_value_to_string)
            .map(|item| item.trim().to_string())
            .filter(|item| !item.is_empty())
            .collect::<Vec<_>>(),
        ("file" | "file-list", Some(Value::Mapping(_))) => Vec::new(),
        ("file" | "file-list", Some(value)) => {
            let text = scalar_value_to_string(value).trim().to_string();
            if text.is_empty() { Vec::new() } else { vec![text] }
        }
        _ => Vec::new(),
    };

    values.dedup();
    values
}

pub(super) fn is_valid_start_variable_file_list_max_length(input: &str) -> bool {
    input
        .trim()
        .parse::<u8>()
        .map(|value| {
            (START_VARIABLE_FILE_LIST_MIN_UPLOAD_COUNT..=START_VARIABLE_FILE_LIST_MAX_UPLOAD_COUNT)
                .contains(&value)
        })
        .unwrap_or(false)
}

pub(in crate::apps::workflow) fn normalized_start_variable_file_list_max_length(input: &str) -> u8 {
    input
        .trim()
        .parse::<u8>()
        .ok()
        .map(|value| {
            value.clamp(
                START_VARIABLE_FILE_LIST_MIN_UPLOAD_COUNT,
                START_VARIABLE_FILE_LIST_MAX_UPLOAD_COUNT,
            )
        })
        .unwrap_or(START_VARIABLE_FILE_LIST_DEFAULT_UPLOAD_COUNT)
}

fn sync_start_variable_file_defaults(variable: &mut WorkflowStartVariableDraft) {
    let mut values = variable
        .default_file_values
        .iter()
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
        .collect::<Vec<_>>();
    values.dedup();

    match variable.input_type.as_str() {
        "file" => {
            let first = values.into_iter().next().unwrap_or_default();
            variable.default_file_values =
                if first.is_empty() { Vec::new() } else { vec![first.clone()] };
            variable.default_value = first;
        }
        "file-list" => {
            let max_count = usize::from(normalized_start_variable_file_list_max_length(
                &variable.max_length_input,
            ));
            values.truncate(max_count);
            variable.default_value = if values.is_empty() {
                String::new()
            } else {
                value_yaml_for_editor(&Value::Sequence(
                    values.iter().cloned().map(Value::String).collect(),
                ))
            };
            variable.default_file_values = values;
        }
        _ => {
            variable.default_file_values.clear();
        }
    }
}

pub(super) fn normalize_start_variable_draft(variable: &mut WorkflowStartVariableDraft) {
    variable.options = variable.options.iter().map(|item| item.trim().to_string()).collect();
    variable.allowed_file_types = variable
        .allowed_file_types
        .iter()
        .map(|item| item.trim().to_ascii_lowercase())
        .filter(|item| !item.is_empty())
        .collect();
    variable.allowed_file_types.sort();
    variable.allowed_file_types.dedup();
    variable.allowed_file_upload_methods =
        normalize_file_upload_methods(variable.allowed_file_upload_methods.clone());
    variable.allowed_file_extensions =
        normalize_file_extensions(if variable.allowed_file_extensions_input.trim().is_empty() {
            variable.allowed_file_extensions.clone()
        } else {
            variable
                .allowed_file_extensions_input
                .split(|ch: char| ch == ',' || ch.is_whitespace())
                .map(str::trim)
                .filter(|item| !item.is_empty())
                .map(|item| item.to_string())
                .collect()
        });
    variable.allowed_file_extensions_input = variable.allowed_file_extensions.join(", ");

    if variable.hidden {
        variable.required = false;
    }

    match variable.input_type.as_str() {
        "text-input" | "paragraph" => {
            variable.options.clear();
            variable.allowed_file_types.clear();
            variable.allowed_file_extensions.clear();
            variable.allowed_file_extensions_input.clear();
            variable.allowed_file_upload_methods.clear();
            if variable.max_length_input.trim().is_empty() {
                variable.max_length_input = "48".to_string();
            }
        }
        "file-list" => {
            variable.options.clear();
            if variable.allowed_file_types.is_empty() {
                variable.allowed_file_types = default_start_variable_allowed_file_types();
            }
            if variable.allowed_file_upload_methods.is_empty() {
                variable.allowed_file_upload_methods =
                    default_start_variable_allowed_upload_methods();
            }
            if !variable.allowed_file_types.iter().any(|item| item == "custom") {
                variable.allowed_file_extensions.clear();
                variable.allowed_file_extensions_input.clear();
            }
            variable.max_length_input =
                normalized_start_variable_file_list_max_length(&variable.max_length_input)
                    .to_string();
            sync_start_variable_file_defaults(variable);
        }
        "file" => {
            variable.options.clear();
            if variable.allowed_file_types.is_empty() {
                variable.allowed_file_types = default_start_variable_allowed_file_types();
            }
            if variable.allowed_file_upload_methods.is_empty() {
                variable.allowed_file_upload_methods =
                    default_start_variable_allowed_upload_methods();
            }
            if !variable.allowed_file_types.iter().any(|item| item == "custom") {
                variable.allowed_file_extensions.clear();
                variable.allowed_file_extensions_input.clear();
            }
            variable.max_length_input.clear();
            sync_start_variable_file_defaults(variable);
        }
        "select" => {
            let options = variable
                .options
                .iter()
                .map(|item| item.trim())
                .filter(|item| !item.is_empty())
                .collect::<Vec<_>>();
            variable.max_length_input.clear();
            variable.allowed_file_types.clear();
            variable.allowed_file_extensions.clear();
            variable.allowed_file_extensions_input.clear();
            variable.allowed_file_upload_methods.clear();
            variable.default_file_values.clear();
            if !options.iter().any(|item| *item == variable.default_value.trim()) {
                variable.default_value.clear();
            }
        }
        "number" => {
            variable.options.clear();
            variable.max_length_input.clear();
            variable.allowed_file_types.clear();
            variable.allowed_file_extensions.clear();
            variable.allowed_file_extensions_input.clear();
            variable.allowed_file_upload_methods.clear();
            variable.default_file_values.clear();
        }
        "checkbox" => {
            variable.options.clear();
            variable.max_length_input.clear();
            variable.allowed_file_types.clear();
            variable.allowed_file_extensions.clear();
            variable.allowed_file_extensions_input.clear();
            variable.allowed_file_upload_methods.clear();
            variable.default_file_values.clear();
            let normalized = variable.default_value.trim().to_ascii_lowercase();
            variable.default_value = if normalized == "true" {
                "true".to_string()
            } else if normalized == "false" || normalized.is_empty() {
                "false".to_string()
            } else {
                variable.default_value.clone()
            };
        }
        _ => {
            variable.options.clear();
            variable.allowed_file_types.clear();
            variable.allowed_file_extensions.clear();
            variable.allowed_file_extensions_input.clear();
            variable.allowed_file_upload_methods.clear();
            variable.default_file_values.clear();
        }
    }
}

pub(super) fn validate_start_variable_editor_draft(
    variable: &WorkflowStartVariableDraft,
    variables: &[WorkflowStartVariableDraft],
    mode: WorkflowStartVariableEditorMode,
) -> Result<(), String> {
    let label = variable.label.trim();
    if label.is_empty() {
        return Err("显示名称不能为空".to_string());
    }

    let name = variable.variable.trim();
    if name.is_empty() {
        return Err("变量名不能为空".to_string());
    }
    if !is_valid_start_variable_name(name) {
        return Err("变量名只能包含字母、数字和下划线，且不能以数字开头".to_string());
    }
    if !is_supported_start_variable_input_type(&variable.input_type) {
        return Err("不支持的字段类型".to_string());
    }

    let duplicated = variables.iter().enumerate().any(|(index, item)| {
        item.variable.trim() == name
            && !matches!(mode, WorkflowStartVariableEditorMode::Edit(edit_index) if edit_index == index)
    });
    if duplicated {
        return Err("变量名不能重复".to_string());
    }

    if variable.input_type == "file-list" {
        if !is_valid_start_variable_file_list_max_length(&variable.max_length_input) {
            return Err("最大上传数必须在 1 到 10 之间".to_string());
        }
    } else if !variable.max_length_input.trim().is_empty() {
        variable
            .max_length_input
            .trim()
            .parse::<u64>()
            .map_err(|_| "最大长度必须是非负整数".to_string())?;
    }

    if variable.input_type == "select" {
        let options = variable
            .options
            .iter()
            .map(|option| option.trim())
            .filter(|option| !option.is_empty())
            .collect::<Vec<_>>();
        if options.is_empty() {
            return Err("下拉选项至少需要一个有效选项".to_string());
        }
        let mut seen = std::collections::HashSet::new();
        for option in &options {
            if !seen.insert((*option).to_string()) {
                return Err("下拉选项不能重复".to_string());
            }
        }
        if !variable.default_value.trim().is_empty()
            && !options.iter().any(|option| *option == variable.default_value.trim())
        {
            return Err("默认值必须在下拉选项中".to_string());
        }
    }

    if variable.input_type == "number"
        && !is_valid_start_variable_number_default_value(&variable.default_value)
    {
        return Err("数字类型默认值必须是数字".to_string());
    }

    if matches!(variable.input_type.as_str(), "file" | "file-list") {
        if variable.allowed_file_types.is_empty() {
            return Err("至少选择一种支持的文件类型".to_string());
        }
        if variable.allowed_file_types.iter().any(|item| item == "custom")
            && variable.allowed_file_extensions.is_empty()
        {
            return Err("选择自定义文件类型时，必须填写扩展名".to_string());
        }
        if variable.allowed_file_upload_methods.is_empty() {
            return Err("至少选择一种上传方式".to_string());
        }
        if variable.input_type == "file-list"
            && variable.default_file_values.len()
                > usize::from(normalized_start_variable_file_list_max_length(
                    &variable.max_length_input,
                ))
        {
            return Err("默认文件数量不能超过最大上传数".to_string());
        }
    }

    Ok(())
}

pub(super) fn default_start_variable_allowed_file_types() -> Vec<String> {
    vec!["image".to_string()]
}

pub(super) fn default_start_variable_allowed_upload_methods() -> Vec<String> {
    vec!["local_file".to_string(), "remote_url".to_string()]
}

pub(super) fn normalize_file_upload_methods(methods: Vec<String>) -> Vec<String> {
    let mut normalized = methods
        .into_iter()
        .map(|item| item.trim().to_ascii_lowercase())
        .filter(|item| matches!(item.as_str(), "local_file" | "remote_url"))
        .collect::<Vec<_>>();
    normalized.sort();
    normalized.dedup();
    normalized
}

pub(super) fn normalize_file_extensions(extensions: Vec<String>) -> Vec<String> {
    let mut normalized = extensions
        .into_iter()
        .map(|item| item.trim().trim_start_matches('.').to_ascii_lowercase())
        .filter(|item| !item.is_empty())
        .map(|item| format!(".{item}"))
        .collect::<Vec<_>>();
    normalized.sort();
    normalized.dedup();
    normalized
}

pub(super) fn is_supported_start_variable_input_type(input_type: &str) -> bool {
    matches!(
        input_type,
        "text-input" | "paragraph" | "select" | "number" | "checkbox" | "file" | "file-list"
    )
}

pub(super) fn is_valid_start_variable_name(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };

    if !first.is_ascii_alphabetic() && first != '_' {
        return false;
    }

    chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

#[cfg(test)]
#[path = "start_variable_validation_tests.rs"]
mod start_variable_validation_tests;
