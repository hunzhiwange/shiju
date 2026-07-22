//! Workflow saved app run messages.

use super::*;
use std::collections::BTreeMap;

pub(super) fn handle(app: &mut App, message: WorkflowMessage) -> Option<Task<Message>> {
    Some(match message {
        WorkflowMessage::OpenSavedAppRunPrompt(uuid) => {
            app.workflow_state.open_saved_app_run_prompt(uuid.clone());
            load_saved_app_run_prompt_task(uuid)
        }
        WorkflowMessage::SavedAppRunPromptLoaded { uuid, result } => {
            app.workflow_state.finish_saved_app_run_prompt_load(&uuid, result);
            Task::none()
        }
        WorkflowMessage::SavedAppRunQueryChanged(query) => {
            app.workflow_state.set_saved_app_run_query(query);
            Task::none()
        }
        WorkflowMessage::SavedAppRunInputChanged(index, value) => {
            app.workflow_state.set_saved_app_run_input_value(index, value);
            Task::none()
        }
        WorkflowMessage::SavedAppRunInputToggled(index, value) => {
            app.workflow_state.toggle_saved_app_run_input_value(index, value);
            Task::none()
        }
        WorkflowMessage::CancelSavedAppRunPrompt => {
            app.workflow_state.close_saved_app_run_prompt();
            Task::none()
        }
        WorkflowMessage::SubmitSavedAppRunPrompt => {
            let Some(prompt) = app.workflow_state.saved_app_run_prompt.clone() else {
                return Some(Task::none());
            };
            if prompt.loading {
                app.workflow_state.set_saved_app_run_prompt_error("运行参数仍在加载".to_string());
                return Some(Task::none());
            }

            let (query, inputs) = match build_run_request_inputs(&prompt) {
                Ok(request) => request,
                Err(error) => {
                    app.workflow_state.set_saved_app_run_prompt_error(error);
                    return Some(Task::none());
                }
            };

            app.workflow_state.remember_saved_app_run_draft(prompt.clone());
            app.workflow_state.begin_saved_app_run(prompt.uuid, query, inputs);
            Task::none()
        }
        WorkflowMessage::SavedAppRunEvent { uuid, event } => {
            if app.workflow_state.running_saved_app_uuid.as_deref() == Some(uuid.as_str()) {
                app.workflow_state.apply_saved_app_run_event(event);
            }
            Task::none()
        }
        WorkflowMessage::SavedAppRunFinished { uuid, result } => {
            if app.workflow_state.running_saved_app_uuid.as_deref() != Some(uuid.as_str()) {
                return Some(Task::none());
            }

            match result {
                Ok(()) => app.workflow_state.finish_saved_app_run(),
                Err(error) => app.workflow_state.fail_saved_app_run(
                    crate::apps::workflow::state::WorkflowRunErrorPreview {
                        status: 0,
                        code: "transport_error".to_string(),
                        message: error,
                    },
                ),
            }
            Task::none()
        }
        WorkflowMessage::CancelSavedAppRun => {
            app.workflow_state.cancel_saved_app_run();
            Task::none()
        }
        WorkflowMessage::RetrySavedAppRunPreview => {
            app.workflow_state.retry_saved_app_run_preview();
            Task::none()
        }
        WorkflowMessage::CloseSavedAppRunPreview => {
            app.workflow_state.close_saved_app_run_preview();
            Task::none()
        }
        WorkflowMessage::OpenSavedAppRunNodeDetail(index) => {
            app.workflow_state.open_saved_app_run_node_detail(index);
            Task::none()
        }
        WorkflowMessage::OpenSavedAppRunOutputDetail => {
            app.workflow_state.open_saved_app_run_output_detail();
            Task::none()
        }
        WorkflowMessage::SavedAppRunDetailAction(action) => {
            app.workflow_state.saved_app_run_detail_action(action);
            Task::none()
        }
        WorkflowMessage::CloseSavedAppRunDetail => {
            app.workflow_state.close_saved_app_run_detail();
            Task::none()
        }
        WorkflowMessage::CopySavedAppRunError => {
            let Some(report) = app.workflow_state.copy_saved_app_run_error() else {
                return Some(Task::none());
            };
            Task::batch(vec![
                iced::clipboard::write(report),
                crate::app::message::after(
                    std::time::Duration::from_secs(2),
                    Message::WorkflowTool(WorkflowMessage::ClearSavedAppRunErrorCopied),
                ),
            ])
        }
        WorkflowMessage::ClearSavedAppRunErrorCopied => {
            app.workflow_state.clear_saved_app_run_error_copied();
            Task::none()
        }
        _ => return None,
    })
}

fn load_saved_app_run_prompt_task(uuid: String) -> Task<Message> {
    #[cfg(target_arch = "wasm32")]
    {
        Task::perform(
            async { Err("Web 平台暂不支持读取本地 Workflow 应用".to_string()) },
            move |result| {
                Message::WorkflowTool(WorkflowMessage::SavedAppRunPromptLoaded {
                    uuid: uuid.clone(),
                    result,
                })
            },
        )
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        Task::perform(
            {
                let uuid = uuid.clone();
                async move {
                    let client = crate::app::config::gateway_client()?;
                    let record = client.workflow_application_get(&uuid).await?;
                    let loaded = load_document_from_text(None, record.workflow_yaml)?;
                    Ok(crate::apps::workflow::state::workflow_run_inputs_from_document(
                        &loaded.document,
                    ))
                }
            },
            move |result| {
                Message::WorkflowTool(WorkflowMessage::SavedAppRunPromptLoaded {
                    uuid: uuid.clone(),
                    result,
                })
            },
        )
    }
}

fn build_run_request_inputs(
    prompt: &crate::apps::workflow::state::WorkflowRunPromptDraft,
) -> Result<(String, BTreeMap<String, serde_json::Value>), String> {
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
                return Err(format!("{} 必填", run_input_label(input)));
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
        return Err("请输入 query 后再运行".to_string());
    }

    Ok((query, parsed))
}

fn parse_run_input_value(
    input: &crate::apps::workflow::state::WorkflowRunInputDraft,
) -> Result<serde_json::Value, String> {
    let value = input.value.trim();
    match input.input_type.as_str() {
        "number" => parse_number_input(value, input),
        "checkbox" => parse_bool_input(value, input),
        "file" => parse_file_input(value, input),
        "file-list" => parse_file_list_input(value, input),
        "select" => {
            if !input.options.is_empty() && !input.options.iter().any(|option| option == value) {
                return Err(format!("{} 必须从选项中选择", run_input_label(input)));
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
    input: &crate::apps::workflow::state::WorkflowRunInputDraft,
) -> Result<serde_json::Value, String> {
    if let Ok(number) = value.parse::<i64>() {
        return Ok(serde_json::Value::Number(number.into()));
    }

    let number =
        value.parse::<f64>().map_err(|_| format!("{} 必须是数字", run_input_label(input)))?;
    let number = serde_json::Number::from_f64(number)
        .ok_or_else(|| format!("{} 必须是有效数字", run_input_label(input)))?;
    Ok(serde_json::Value::Number(number))
}

fn parse_bool_input(
    value: &str,
    input: &crate::apps::workflow::state::WorkflowRunInputDraft,
) -> Result<serde_json::Value, String> {
    match value.to_ascii_lowercase().as_str() {
        "" | "false" => Ok(serde_json::Value::Bool(false)),
        "true" => Ok(serde_json::Value::Bool(true)),
        _ => Err(format!("{} 必须是 boolean", run_input_label(input))),
    }
}

fn parse_file_input(
    value: &str,
    input: &crate::apps::workflow::state::WorkflowRunInputDraft,
) -> Result<serde_json::Value, String> {
    if split_file_values(value).len() > 1 {
        return Err(format!("{} 只能填写一个文件", run_input_label(input)));
    }
    file_value_from_text(value)
}

fn parse_file_list_input(
    value: &str,
    _input: &crate::apps::workflow::state::WorkflowRunInputDraft,
) -> Result<serde_json::Value, String> {
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

fn file_value_from_text(value: &str) -> Result<serde_json::Value, String> {
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(value) {
        return match json {
            serde_json::Value::Object(_) => Ok(json),
            serde_json::Value::String(text) => Ok(file_object_from_path_or_url(&text)),
            _ => Err("文件值必须是 URL、路径或文件对象".to_string()),
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
    input: &crate::apps::workflow::state::WorkflowRunInputDraft,
) -> Result<(), String> {
    let max_length = input.max_length_input.trim();
    if max_length.is_empty() {
        return Ok(());
    }
    let Ok(max_length) = max_length.parse::<usize>() else {
        return Ok(());
    };
    if input.value.chars().count() > max_length {
        return Err(format!("{} 超过最大长度 {}", run_input_label(input), max_length));
    }
    Ok(())
}

fn run_input_value_is_empty(input: &crate::apps::workflow::state::WorkflowRunInputDraft) -> bool {
    match input.input_type.as_str() {
        "file-list" => split_file_values(&input.value).is_empty(),
        _ => input.value.trim().is_empty(),
    }
}

fn run_input_label(input: &crate::apps::workflow::state::WorkflowRunInputDraft) -> &str {
    if input.label.trim().is_empty() { input.name.as_str() } else { input.label.as_str() }
}

#[cfg(test)]
#[path = "run_tests.rs"]
mod run_tests;
