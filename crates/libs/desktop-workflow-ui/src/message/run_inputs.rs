use std::collections::BTreeMap;
pub fn build_run_request_inputs(
    prompt: &desktop_workflow_state::state::WorkflowRunPromptDraft,
) -> Result<(String, BTreeMap<String, serde_json::Value>), desktop_i18n::workflow::Feedback> {
    // 提交前统一按输入定义解析和校验，避免把界面草稿字符串直接交给运行时。
    let mut parsed = BTreeMap::new();
    let mut query = prompt.query.trim().to_string();

    for input in &prompt.inputs {
        let name = input.name.trim();
        if name.is_empty() {
            continue;
        }

        if input.input_type == "checkbox" {
            parsed.insert(name.to_string(), parse_run_input_value(input)?);
            if name == "query" && query.is_empty() {
                query = input.value.trim().to_string();
            }
            continue;
        }

        if run_input_value_is_empty(input) {
            if input.required {
                return Err(desktop_i18n::workflow::Feedback::new(
                    "%s 必填",
                    vec![run_input_label(input).to_string()],
                ));
            }
            continue;
        }

        let value = parse_run_input_value(input)?;
        if name == "query" && query.is_empty() {
            if let Some(text) = value.as_str() {
                query = text.to_string();
            }
        }
        parsed.insert(name.to_string(), value);
    }

    if query.trim().is_empty() {
        return Err(desktop_i18n::workflow::Feedback::new("请输入 query 后再运行", vec![]));
    }

    Ok((query, parsed))
}

fn parse_run_input_value(
    input: &desktop_workflow_state::state::WorkflowRunInputDraft,
) -> Result<serde_json::Value, desktop_i18n::workflow::Feedback> {
    let value = input.value.trim();
    match input.input_type.as_str() {
        "number" => parse_number_input(value, input),
        "checkbox" => parse_bool_input(value, input),
        "file" => parse_file_input(value, input),
        "file-list" => parse_file_list_input(value, input),
        "select" => {
            if !input.options.is_empty() && !input.options.iter().any(|option| option == value) {
                return Err(desktop_i18n::workflow::Feedback::new(
                    "%s 必须从选项中选择",
                    vec![run_input_label(input).to_string()],
                ));
            }
            Ok(serde_json::Value::String(input.value.clone()))
        }
        _ => {
            validate_text_length(input)?;
            Ok(serde_json::Value::String(input.value.clone()))
        }
    }
}

fn parse_number_input(
    value: &str,
    input: &desktop_workflow_state::state::WorkflowRunInputDraft,
) -> Result<serde_json::Value, desktop_i18n::workflow::Feedback> {
    if let Ok(number) = value.parse::<i64>() {
        return Ok(serde_json::Value::Number(number.into()));
    }

    let number = value.parse::<f64>().map_err(|_| {
        desktop_i18n::workflow::Feedback::new(
            "%s 必须是数字",
            vec![run_input_label(input).to_string()],
        )
    })?;
    let number = serde_json::Number::from_f64(number).ok_or_else(|| {
        desktop_i18n::workflow::Feedback::new(
            "%s 必须是有效数字",
            vec![run_input_label(input).to_string()],
        )
    })?;
    Ok(serde_json::Value::Number(number))
}

fn parse_bool_input(
    value: &str,
    input: &desktop_workflow_state::state::WorkflowRunInputDraft,
) -> Result<serde_json::Value, desktop_i18n::workflow::Feedback> {
    match value.to_ascii_lowercase().as_str() {
        "" | "false" => Ok(serde_json::Value::Bool(false)),
        "true" => Ok(serde_json::Value::Bool(true)),
        _ => Err(desktop_i18n::workflow::Feedback::new(
            "%s 必须是 boolean",
            vec![run_input_label(input).to_string()],
        )),
    }
}

fn parse_file_input(
    value: &str,
    input: &desktop_workflow_state::state::WorkflowRunInputDraft,
) -> Result<serde_json::Value, desktop_i18n::workflow::Feedback> {
    if split_file_values(value).len() > 1 {
        return Err(desktop_i18n::workflow::Feedback::new(
            "%s 只能填写一个文件",
            vec![run_input_label(input).to_string()],
        ));
    }
    file_value_from_text(value)
}

fn parse_file_list_input(
    value: &str,
    _input: &desktop_workflow_state::state::WorkflowRunInputDraft,
) -> Result<serde_json::Value, desktop_i18n::workflow::Feedback> {
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(value) {
        return match json {
            serde_json::Value::Array(_) => Ok(json),
            serde_json::Value::Object(_) => Ok(serde_json::Value::Array(vec![json])),
            _ => Ok(serde_json::Value::Array(vec![file_value_from_text(value)?])),
        };
    }

    split_file_values(value)
        .into_iter()
        .map(file_value_from_text)
        .collect::<Result<Vec<_>, _>>()
        .map(serde_json::Value::Array)
}

fn file_value_from_text(
    value: &str,
) -> Result<serde_json::Value, desktop_i18n::workflow::Feedback> {
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(value) {
        return match json {
            serde_json::Value::Object(_) => Ok(json),
            serde_json::Value::String(text) => Ok(file_object_from_path_or_url(&text)),
            _ => Err(desktop_i18n::workflow::Feedback::new(
                "文件值必须是 URL、路径或文件对象",
                vec![],
            )),
        };
    }

    Ok(file_object_from_path_or_url(value))
}

fn file_object_from_path_or_url(value: &str) -> serde_json::Value {
    let mut object = serde_json::Map::new();
    let trimmed = value.trim();
    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        object.insert("url".to_string(), serde_json::Value::String(trimmed.to_string()));
    } else {
        object.insert("path".to_string(), serde_json::Value::String(trimmed.to_string()));
    }
    let name =
        trimmed.rsplit(['/', '\\']).next().filter(|name| !name.is_empty()).unwrap_or(trimmed);
    object.insert("name".to_string(), serde_json::Value::String(name.to_string()));
    serde_json::Value::Object(object)
}

fn split_file_values(value: &str) -> Vec<&str> {
    value.split([',', '\n']).map(str::trim).filter(|item| !item.is_empty()).collect()
}

fn validate_text_length(
    input: &desktop_workflow_state::state::WorkflowRunInputDraft,
) -> Result<(), desktop_i18n::workflow::Feedback> {
    let max_length = input.max_length_input.trim();
    if max_length.is_empty() {
        return Ok(());
    }
    let Ok(max_length) = max_length.parse::<usize>() else {
        return Ok(());
    };
    if input.value.chars().count() > max_length {
        return Err(desktop_i18n::workflow::Feedback::new(
            "%s 超过最大长度 %s",
            vec![run_input_label(input).to_string(), max_length.to_string()],
        ));
    }
    Ok(())
}

fn run_input_value_is_empty(input: &desktop_workflow_state::state::WorkflowRunInputDraft) -> bool {
    match input.input_type.as_str() {
        "file-list" => split_file_values(&input.value).is_empty(),
        _ => input.value.trim().is_empty(),
    }
}

fn run_input_label(input: &desktop_workflow_state::state::WorkflowRunInputDraft) -> &str {
    if input.label.trim().is_empty() { input.name.as_str() } else { input.label.as_str() }
}

#[cfg(test)]
#[path = "run_inputs_tests.rs"]
mod run_tests;
