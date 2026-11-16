//! Internal code variables helpers.
use super::*;
use iced::widget::column;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct WorkflowCodePickerOption {
    pub(super) value: &'static str,
    pub(super) label: &'static str,
}

impl std::fmt::Display for WorkflowCodePickerOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct WorkflowCodeVariableReferenceOption {
    pub(super) selector_key: String,
    pub(super) label: String,
    pub(super) value_type: String,
}

impl std::fmt::Display for WorkflowCodeVariableReferenceOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} · {}", self.label, code_value_type_label(&self.value_type))
    }
}

pub(super) fn build_code_input_variable_row<'a>(
    i18n: desktop_i18n::I18n,
    index: usize,
    input: &'a WorkflowCodeVariableDraft,
    options: &[WorkflowCodeVariableReferenceOption],
    validation: &'a super::state::WorkflowNodeEditorValidation,
) -> Element<'a, Message> {
    let variable_path = format!("code.inputs[{index}].variable");
    let selector_path = format!("code.inputs[{index}].selector");
    let value_type_path = format!("code.inputs[{index}].value_type");
    let selected = selected_code_variable_reference_option(input, options);
    let mut row_options = options.to_vec();
    if let Some(selected) = selected.clone()
        && !row_options.iter().any(|option| option == &selected)
    {
        row_options.push(selected);
    }

    let selector_control: Element<'a, Message> = if row_options.is_empty() {
        text(i18n.__("当前没有可选变量")).size(12).style(settings_muted_text_style).into()
    } else {
        settings_select(row_options, selected, move |option| {
            WorkflowMessage::NodeEditorCodeInputVariableSelectorChanged(
                index,
                option.selector_key.clone(),
                option.value_type.clone(),
            )
        })
        .width(Length::FillPortion(3))
        .into()
    };

    let mut content = column![
        row![
            selector_control,
            workflow_text_input(i18n.__("参数名"), &input.variable, move |value| {
                WorkflowMessage::NodeEditorCodeInputVariableNameChanged(
                    index, value,
                )
            }),
            text(code_value_type_label(&input.value_type))
                .size(11)
                .style(settings_muted_text_style),
            button(text(i18n.__("删除")).size(12))
                .style(danger_action_btn_style)
                .padding([8, 10])
                .on_press(
                    WorkflowMessage::NodeEditorCodeRemoveInputVariable(index)),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    ]
    .spacing(8);

    if let Some(error) = validation.localized_error_for(i18n, &variable_path) {
        content = content.push(build_inline_error(error));
    }
    if let Some(error) = validation.localized_error_for(i18n, &selector_path) {
        content = content.push(build_inline_error(error));
    }
    if let Some(error) = validation.localized_error_for(i18n, &value_type_path) {
        content = content.push(build_inline_error(error));
    }

    content.into()
}

pub(super) fn build_code_output_variable_row<'a>(
    i18n: desktop_i18n::I18n,
    index: usize,
    output: &'a WorkflowCodeOutputDraft,
    type_options: &[WorkflowCodePickerOption],
    validation: &'a super::state::WorkflowNodeEditorValidation,
) -> Element<'a, Message> {
    let key_path = format!("code.outputs[{index}].key");
    let type_path = format!("code.outputs[{index}].type");
    let selected = code_picker_option(&output.value_type, type_options)
        .or_else(|| Some(WorkflowCodePickerOption { value: "string", label: "String" }));

    let mut content = column![
        row![
            workflow_text_input(i18n.__("输出变量名"), &output.key, move |value| {
                WorkflowMessage::NodeEditorCodeOutputNameChanged(
                    index, value,
                )
            }),
            settings_select(type_options.to_vec(), selected, move |option| {
                WorkflowMessage::NodeEditorCodeOutputTypeChanged(
                    index,
                    option.value.to_string(),
                )
            })
            .width(Length::Fill),
            button(text(i18n.__("删除")).size(12))
                .style(danger_action_btn_style)
                .padding([8, 10])
                .on_press(
                    WorkflowMessage::NodeEditorCodeRemoveOutputVariable(index)),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    ]
    .spacing(8);

    if let Some(error) = validation.localized_error_for(i18n, &key_path) {
        content = content.push(build_inline_error(error));
    }
    if let Some(error) = validation.localized_error_for(i18n, &type_path) {
        content = content.push(build_inline_error(error));
    }

    content.into()
}

pub(super) fn code_language_options() -> [WorkflowCodePickerOption; 2] {
    [
        WorkflowCodePickerOption { value: "python3", label: "PYTHON3" },
        WorkflowCodePickerOption { value: "javascript", label: "JAVASCRIPT" },
    ]
}

pub(super) fn code_output_type_options() -> [WorkflowCodePickerOption; 8] {
    [
        WorkflowCodePickerOption { value: "string", label: "String" },
        WorkflowCodePickerOption { value: "number", label: "Number" },
        WorkflowCodePickerOption { value: "boolean", label: "Boolean" },
        WorkflowCodePickerOption { value: "array[number]", label: "Array[Number]" },
        WorkflowCodePickerOption { value: "array[string]", label: "Array[String]" },
        WorkflowCodePickerOption { value: "array[boolean]", label: "Array[Boolean]" },
        WorkflowCodePickerOption { value: "array[object]", label: "Array[Object]" },
        WorkflowCodePickerOption { value: "object", label: "Object" },
    ]
}

pub(super) fn code_error_strategy_options(i18n: desktop_i18n::I18n) -> [WorkflowCodePickerOption; 3] {
    [
        WorkflowCodePickerOption { value: "none", label: i18n.__("无") },
        WorkflowCodePickerOption { value: "default-value", label: i18n.__("默认值") },
        WorkflowCodePickerOption { value: "fail-branch", label: i18n.__("异常分支") },
    ]
}

pub(super) fn code_picker_option(
    value: &str,
    options: &[WorkflowCodePickerOption],
) -> Option<WorkflowCodePickerOption> {
    options.iter().copied().find(|option| option.value == value)
}

pub(super) fn code_value_type_label(value_type: &str) -> &'static str {
    match value_type {
        "number" => "Number",
        "boolean" => "Boolean",
        "object" => "Object",
        "array[number]" => "Array[Number]",
        "array[string]" => "Array[String]",
        "array[boolean]" => "Array[Boolean]",
        "array[object]" => "Array[Object]",
        "file" => "File",
        "array[file]" => "Array[File]",
        _ => "String",
    }
}

pub(super) fn selected_code_variable_reference_option(
    input: &WorkflowCodeVariableDraft,
    options: &[WorkflowCodeVariableReferenceOption],
) -> Option<WorkflowCodeVariableReferenceOption> {
    let selector_key = input.selector.join(".");
    if selector_key.is_empty() {
        return None;
    }

    options.iter().find(|option| option.selector_key == selector_key).cloned().or_else(|| {
        Some(WorkflowCodeVariableReferenceOption {
            selector_key,
            label: if input.variable.trim().is_empty() {
                input.selector.join(".")
            } else {
                input.variable.clone()
            },
            value_type: input.value_type.clone(),
        })
    })
}

pub(super) fn build_code_variable_reference_options(
    state: &WorkflowState,
    editor: &super::state::WorkflowNodeEditorDraft,
) -> Vec<WorkflowCodeVariableReferenceOption> {
    let current_node_id = match &editor.mode {
        super::state::WorkflowNodeEditorMode::Edit(node_id) => Some(node_id.as_str()),
        super::state::WorkflowNodeEditorMode::Create => None,
    };
    let restrict_to_upstream = current_node_id.is_some();
    let upstream_ids =
        current_node_id.map(|node_id| upstream_node_ids(state, node_id)).unwrap_or_default();
    let mut options = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for node in &state.document.nodes {
        if current_node_id == Some(node.id.as_str()) {
            continue;
        }
        if restrict_to_upstream && !upstream_ids.contains(node.id.as_str()) {
            continue;
        }

        if node.block_type == "start" {
            for (name, value_type) in start_node_variable_entries(node) {
                let selector_key = format!("{}.{}", node.id, name);
                if seen.insert(selector_key.clone()) {
                    options.push(WorkflowCodeVariableReferenceOption {
                        selector_key,
                        label: name,
                        value_type,
                    });
                }
            }
        }

        for (key, value_type) in node_output_entries(node) {
            let selector_key = format!("{}.{}", node.id, key);
            if seen.insert(selector_key.clone()) {
                options.push(WorkflowCodeVariableReferenceOption {
                    selector_key,
                    label: format!("{} · {}", node.title, key),
                    value_type,
                });
            }
        }
    }

    for variable in &state.environment_variables {
        let selector_key = format!("env.{}", variable.name);
        if seen.insert(selector_key.clone()) {
            options.push(WorkflowCodeVariableReferenceOption {
                selector_key,
                label: format!("env.{}", variable.name),
                value_type: variable.value_type.clone(),
            });
        }
    }

    for variable in &state.conversation_variables {
        let selector_key = format!("conversation.{}", variable.name);
        if seen.insert(selector_key.clone()) {
            options.push(WorkflowCodeVariableReferenceOption {
                selector_key,
                label: format!("conversation.{}", variable.name),
                value_type: variable.value_type.clone(),
            });
        }
    }

    if let Some(meta) = state.active_meta() {
        for variable in workflow_system_variables(meta) {
            let key = variable.name.trim_start_matches("sys.");
            let selector_key = format!("sys.{key}");
            if seen.insert(selector_key.clone()) {
                options.push(WorkflowCodeVariableReferenceOption {
                    selector_key,
                    label: variable.name.to_string(),
                    value_type: variable.value_type.to_string(),
                });
            }
        }
    }

    options
}

pub(super) fn upstream_node_ids<'a>(
    state: &'a WorkflowState,
    node_id: &str,
) -> std::collections::HashSet<&'a str> {
    let mut visited = std::collections::HashSet::new();
    let mut frontier = vec![node_id];

    while let Some(target_id) = frontier.pop() {
        for edge in state.document.edges.iter().filter(|edge| edge.target == target_id) {
            if visited.insert(edge.source.as_str()) {
                frontier.push(edge.source.as_str());
            }
        }
    }

    visited
}

pub(super) fn start_node_variable_entries(node: &WorkflowNode) -> Vec<(String, String)> {
    node_data_map(node)
        .and_then(|data| data.get(&serde_yaml::Value::String("variables".to_string())))
        .and_then(serde_yaml::Value::as_sequence)
        .map(|variables| {
            variables
                .iter()
                .filter_map(serde_yaml::Value::as_mapping)
                .filter_map(|item| {
                    let name = item
                        .get(&serde_yaml::Value::String("variable".to_string()))
                        .and_then(serde_yaml::Value::as_str)?
                        .trim()
                        .to_string();
                    if name.is_empty() {
                        return None;
                    }

                    let input_type = item
                        .get(&serde_yaml::Value::String("type".to_string()))
                        .and_then(serde_yaml::Value::as_str)
                        .unwrap_or("text-input");
                    Some((name, start_input_value_type(input_type).to_string()))
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

pub(super) fn node_output_entries(node: &WorkflowNode) -> Vec<(String, String)> {
    node_data_map(node)
        .and_then(|data| data.get(&serde_yaml::Value::String("outputs".to_string())))
        .and_then(serde_yaml::Value::as_mapping)
        .map(|outputs| {
            outputs
                .iter()
                .filter_map(|(key, value)| {
                    let key = key.as_str()?.trim().to_string();
                    if key.is_empty() {
                        return None;
                    }

                    let value_type = value
                        .as_mapping()
                        .and_then(|map| map.get(&serde_yaml::Value::String("type".to_string())))
                        .and_then(serde_yaml::Value::as_str)
                        .unwrap_or("string")
                        .to_string();
                    Some((key, value_type))
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

pub(super) fn node_data_map(node: &WorkflowNode) -> Option<&serde_yaml::Mapping> {
    node.raw_node
        .as_mapping()
        .and_then(|map| map.get(&serde_yaml::Value::String("data".to_string())))
        .and_then(serde_yaml::Value::as_mapping)
}

pub(super) fn start_input_value_type(input_type: &str) -> &'static str {
    match input_type {
        "number" => "number",
        "checkbox" => "boolean",
        "file" => "file",
        "file-list" => "array[file]",
        _ => "string",
    }
}
