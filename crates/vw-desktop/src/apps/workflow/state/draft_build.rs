//! # Workflow 草稿构建
//!
//! 该模块负责在 YAML 数据与可视化草稿之间转换，重点处理开始变量和条件分支草稿。

use super::*;

pub(super) fn build_node_visual_draft(
    block_type: &str,
    raw_data_yaml: &str,
) -> Result<Option<WorkflowNodeVisualDraft>, String> {
    let data_value = parse_node_data_yaml_value(raw_data_yaml)?;
    let data_map = data_value
        .as_mapping()
        .ok_or_else(|| "节点 data 必须是对象映射（YAML map）".to_string())?;

    match block_type {
        "start" => Ok(Some(WorkflowNodeVisualDraft::Start {
            variables: mapping_value(data_map, "variables")
                .and_then(Value::as_sequence)
                .map(|variables: &Vec<Value>| {
                    variables.iter().map(build_start_variable_draft).collect::<Vec<_>>()
                })
                .unwrap_or_default(),
        })),
        "end" => Ok(Some(WorkflowNodeVisualDraft::End {
            outputs: build_named_value_drafts(mapping_value(data_map, "outputs")),
        })),
        "template-transform" => Ok(Some(WorkflowNodeVisualDraft::TemplateTransform {
            variables: build_named_value_drafts(mapping_value(data_map, "variables")),
            template_editor: text_editor::Content::with_text(
                mapping_value(data_map, "template").and_then(Value::as_str).unwrap_or_default(),
            ),
        })),
        "document-extractor" => Ok(Some(WorkflowNodeVisualDraft::DocumentExtractor {
            variable_selector: WorkflowVariableSelectorDraft {
                path: mapping_value(data_map, "variable_selector")
                    .and_then(Value::as_sequence)
                    .map(|items| {
                        items.iter().filter_map(Value::as_str).map(str::to_string).collect()
                    })
                    .unwrap_or_default(),
            },
        })),
        "question-classifier" => Ok(Some(WorkflowNodeVisualDraft::QuestionClassifier {
            query: selector_draft(mapping_value(data_map, "query_variable_selector")),
            model: model_draft(mapping_value(data_map, "model")),
            classes: mapping_value(data_map, "classes")
                .and_then(Value::as_sequence)
                .map(|items| {
                    items
                        .iter()
                        .filter_map(Value::as_mapping)
                        .map(|item| WorkflowClassifierClassDraft {
                            raw_class: Value::Mapping(item.clone()),
                            id: mapping_value(item, "id")
                                .and_then(Value::as_str)
                                .unwrap_or_default()
                                .to_string(),
                            name: mapping_value(item, "name")
                                .and_then(Value::as_str)
                                .unwrap_or_default()
                                .to_string(),
                        })
                        .collect()
                })
                .unwrap_or_default(),
            instruction_editor: text_editor::Content::with_text(
                mapping_value(data_map, "instruction").and_then(Value::as_str).unwrap_or_default(),
            ),
        })),
        "parameter-extractor" => Ok(Some(WorkflowNodeVisualDraft::ParameterExtractor {
            query: selector_draft(mapping_value(data_map, "query")),
            model: model_draft(mapping_value(data_map, "model")),
            parameters: mapping_value(data_map, "parameters")
                .and_then(Value::as_sequence)
                .map(|items| {
                    items
                        .iter()
                        .filter_map(Value::as_mapping)
                        .map(|item| WorkflowParameterDraft {
                            raw_parameter: Value::Mapping(item.clone()),
                            name: mapping_value(item, "name")
                                .and_then(Value::as_str)
                                .unwrap_or_default()
                                .to_string(),
                            value_type: mapping_value(item, "type")
                                .and_then(Value::as_str)
                                .unwrap_or("string")
                                .to_string(),
                            description: mapping_value(item, "description")
                                .and_then(Value::as_str)
                                .unwrap_or_default()
                                .to_string(),
                            required: mapping_value(item, "required")
                                .and_then(Value::as_bool)
                                .unwrap_or(false),
                        })
                        .collect()
                })
                .unwrap_or_default(),
            reasoning_mode: mapping_value(data_map, "reasoning_mode")
                .and_then(Value::as_str)
                .unwrap_or("prompt")
                .to_string(),
            instruction_editor: text_editor::Content::with_text(
                mapping_value(data_map, "instruction").and_then(Value::as_str).unwrap_or_default(),
            ),
        })),
        "llm" => {
            let model_map = mapping_value(data_map, "model").and_then(Value::as_mapping);
            let completion_params = model_map
                .and_then(|model| mapping_value(model, "completion_params"))
                .and_then(Value::as_mapping);
            let context_map = mapping_value(data_map, "context").and_then(Value::as_mapping);
            let vision_map = mapping_value(data_map, "vision").and_then(Value::as_mapping);
            let prompt_template =
                mapping_value(data_map, "prompt_template").and_then(Value::as_sequence);

            Ok(Some(WorkflowNodeVisualDraft::Llm {
                provider: model_map
                    .and_then(|model| mapping_value(model, "provider"))
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                model_name: model_map
                    .and_then(|model| mapping_value(model, "name"))
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                model_mode: model_map
                    .and_then(|model| mapping_value(model, "mode"))
                    .and_then(Value::as_str)
                    .unwrap_or("chat")
                    .to_string(),
                enable_thinking: completion_params
                    .and_then(|params| mapping_value(params, "enable_thinking"))
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
                context_enabled: context_map
                    .and_then(|context| mapping_value(context, "enabled"))
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
                context_selector_input: selector_input_from_value(
                    context_map.and_then(|context| mapping_value(context, "variable_selector")),
                ),
                system_prompt_editor: text_editor::Content::with_text(&prompt_text_by_role(
                    prompt_template,
                    "system",
                )),
                user_prompt_editor: text_editor::Content::with_text(&prompt_text_by_role(
                    prompt_template,
                    "user",
                )),
                vision_enabled: vision_map
                    .and_then(|vision| mapping_value(vision, "enabled"))
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
            }))
        }
        "answer" => Ok(Some(WorkflowNodeVisualDraft::Answer {
            answer_editor: text_editor::Content::with_text(
                mapping_value(data_map, "answer").and_then(Value::as_str).unwrap_or_default(),
            ),
        })),
        "if-else" => Ok(Some(WorkflowNodeVisualDraft::IfElse {
            cases: mapping_value(data_map, "cases")
                .and_then(Value::as_sequence)
                .map(|cases: &Vec<Value>| {
                    cases.iter().map(build_if_else_case_draft).collect::<Vec<_>>()
                })
                .unwrap_or_default(),
        })),
        "knowledge-retrieval" => {
            let multiple_config =
                mapping_value(data_map, "multiple_retrieval_config").and_then(Value::as_mapping);
            let single_model = mapping_value(data_map, "single_retrieval_config")
                .and_then(Value::as_mapping)
                .and_then(|config| mapping_value(config, "model"))
                .and_then(Value::as_mapping);

            Ok(Some(WorkflowNodeVisualDraft::KnowledgeRetrieval {
                query_selector_input: selector_path_input_from_value(mapping_value(
                    data_map,
                    "query_variable_selector",
                )),
                query_attachment_selector_input: selector_path_input_from_value(mapping_value(
                    data_map,
                    "query_attachment_selector",
                )),
                dataset_ids_input: string_list_input_from_value(mapping_value(
                    data_map,
                    "dataset_ids",
                )),
                retrieval_mode: mapping_value(data_map, "retrieval_mode")
                    .and_then(Value::as_str)
                    .unwrap_or("multiple")
                    .to_string(),
                top_k_input: multiple_config
                    .and_then(|config| mapping_value(config, "top_k"))
                    .map(scalar_value_to_string)
                    .unwrap_or_else(|| "5".to_string()),
                score_threshold_enabled: matches!(
                    multiple_config.and_then(|config| mapping_value(config, "score_threshold")),
                    Some(value) if !value.is_null()
                ),
                score_threshold_input: multiple_config
                    .and_then(|config| mapping_value(config, "score_threshold"))
                    .and_then(|value| (!value.is_null()).then(|| scalar_value_to_string(value)))
                    .unwrap_or_default(),
                reranking_enable: multiple_config
                    .and_then(|config| mapping_value(config, "reranking_enable"))
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
                single_model_provider: single_model
                    .and_then(|model| mapping_value(model, "provider"))
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                single_model_name: single_model
                    .and_then(|model| mapping_value(model, "name"))
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                single_model_mode: single_model
                    .and_then(|model| mapping_value(model, "mode"))
                    .and_then(Value::as_str)
                    .unwrap_or("chat")
                    .to_string(),
            }))
        }
        "tool" => {
            let parameters_yaml = value_yaml_for_editor(
                mapping_value(data_map, "tool_parameters")
                    .unwrap_or(&Value::Mapping(Mapping::new())),
            );
            let configurations_yaml = value_yaml_for_editor(
                mapping_value(data_map, "tool_configurations")
                    .unwrap_or(&Value::Mapping(Mapping::new())),
            );

            Ok(Some(WorkflowNodeVisualDraft::Tool {
                provider_id: mapping_value(data_map, "provider_id")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                provider_type: mapping_value(data_map, "provider_type")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                provider_name: mapping_value(data_map, "provider_name")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                tool_name: mapping_value(data_map, "tool_name")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                tool_label: mapping_value(data_map, "tool_label")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                tool_description: mapping_value(data_map, "tool_description")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                credential_id: mapping_value(data_map, "credential_id")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                plugin_unique_identifier: mapping_value(data_map, "plugin_unique_identifier")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                tool_parameters_editor: text_editor::Content::with_text(&parameters_yaml),
                tool_configurations_editor: text_editor::Content::with_text(&configurations_yaml),
            }))
        }
        "agent" => {
            let output_schema_yaml = value_yaml_for_editor(
                mapping_value(data_map, "output_schema").unwrap_or(&Value::Mapping(Mapping::new())),
            );
            let parameters_yaml = value_yaml_for_editor(
                mapping_value(data_map, "agent_parameters")
                    .unwrap_or(&Value::Mapping(Mapping::new())),
            );
            let memory_map = mapping_value(data_map, "memory").and_then(Value::as_mapping);
            let memory_window = memory_map
                .and_then(|memory| mapping_value(memory, "window"))
                .and_then(Value::as_mapping);

            Ok(Some(WorkflowNodeVisualDraft::Agent {
                strategy_provider_name: mapping_value(data_map, "agent_strategy_provider_name")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                strategy_name: mapping_value(data_map, "agent_strategy_name")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                strategy_label: mapping_value(data_map, "agent_strategy_label")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                plugin_unique_identifier: mapping_value(data_map, "plugin_unique_identifier")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                output_schema_editor: text_editor::Content::with_text(&output_schema_yaml),
                parameters_editor: text_editor::Content::with_text(&parameters_yaml),
                memory_enabled: memory_window
                    .and_then(|window| mapping_value(window, "enabled"))
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
                memory_window_size_input: memory_window
                    .and_then(|window| mapping_value(window, "size"))
                    .map(scalar_value_to_string)
                    .unwrap_or_else(|| "3".to_string()),
                memory_prompt_editor: text_editor::Content::with_text(
                    memory_map
                        .and_then(|memory| mapping_value(memory, "query_prompt_template"))
                        .and_then(Value::as_str)
                        .unwrap_or_default(),
                ),
            }))
        }
        "code" => {
            let outputs = build_code_output_drafts(mapping_value(data_map, "outputs"));
            let error_strategy =
                build_code_error_strategy(mapping_value(data_map, "error_strategy"));
            Ok(Some(WorkflowNodeVisualDraft::Code {
                language: mapping_value(data_map, "code_language")
                    .and_then(Value::as_str)
                    .unwrap_or("python3")
                    .to_string(),
                inputs: build_code_variable_drafts(mapping_value(data_map, "variables")),
                code_editor: text_editor::Content::with_text(
                    mapping_value(data_map, "code").and_then(Value::as_str).unwrap_or_default(),
                ),
                outputs: outputs.clone(),
                retry_config: build_code_retry_draft(mapping_value(data_map, "retry_config")),
                error_strategy: error_strategy.clone(),
                default_value_editor: text_editor::Content::with_text(
                    &code_default_value_editor_text(
                        &outputs,
                        mapping_value(data_map, "default_value"),
                        &error_strategy,
                    ),
                ),
            }))
        }
        _ => Ok(None),
    }
}

fn build_named_value_drafts(value: Option<&Value>) -> Vec<WorkflowNamedValueDraft> {
    value
        .and_then(Value::as_sequence)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_mapping)
                .map(|item| WorkflowNamedValueDraft {
                    id: mapping_value(item, "id")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                    name: mapping_value(item, "variable")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                    value_type: mapping_value(item, "value_type")
                        .or_else(|| mapping_value(item, "type"))
                        .and_then(Value::as_str)
                        .unwrap_or("string")
                        .to_string(),
                    selector: WorkflowVariableSelectorDraft {
                        path: mapping_value(item, "value_selector")
                            .and_then(Value::as_sequence)
                            .map(|selector| {
                                selector
                                    .iter()
                                    .filter_map(Value::as_str)
                                    .map(str::to_string)
                                    .collect()
                            })
                            .unwrap_or_default(),
                    },
                })
                .collect()
        })
        .unwrap_or_default()
}

fn selector_draft(value: Option<&Value>) -> WorkflowVariableSelectorDraft {
    WorkflowVariableSelectorDraft {
        path: value
            .and_then(Value::as_sequence)
            .map(|parts| parts.iter().filter_map(Value::as_str).map(str::to_string).collect())
            .unwrap_or_default(),
    }
}

fn model_draft(value: Option<&Value>) -> WorkflowModelDraft {
    let model = value.and_then(Value::as_mapping);
    WorkflowModelDraft {
        provider: model
            .and_then(|map| mapping_value(map, "provider"))
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        name: model
            .and_then(|map| mapping_value(map, "name"))
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        mode: model
            .and_then(|map| mapping_value(map, "mode"))
            .and_then(Value::as_str)
            .unwrap_or("chat")
            .to_string(),
    }
}

pub(super) fn sync_node_editor_raw_from_visual(
    editor: &mut WorkflowNodeEditorDraft,
) -> Result<(), String> {
    let merged_yaml = apply_visual_draft_to_yaml(
        &editor.block_type,
        &editor.raw_data_editor.text(),
        editor.visual_draft.as_ref(),
    )?;
    editor.raw_data_editor = text_editor::Content::with_text(&merged_yaml);
    Ok(())
}

pub(super) fn build_code_variable_drafts(value: Option<&Value>) -> Vec<WorkflowCodeVariableDraft> {
    value
        .and_then(Value::as_sequence)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_mapping)
                .map(|item| {
                    let selector = item
                        .get(&yaml_key("value_selector"))
                        .and_then(Value::as_sequence)
                        .map(|parts| {
                            parts
                                .iter()
                                .filter_map(Value::as_str)
                                .map(|part| part.to_string())
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default();
                    let variable = item
                        .get(&yaml_key("variable"))
                        .and_then(Value::as_str)
                        .map(str::to_string)
                        .filter(|text| !text.trim().is_empty())
                        .or_else(|| selector.last().cloned())
                        .unwrap_or_default();

                    WorkflowCodeVariableDraft {
                        variable,
                        value_type: item
                            .get(&yaml_key("value_type"))
                            .and_then(Value::as_str)
                            .unwrap_or("string")
                            .to_string(),
                        selector,
                    }
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

pub(super) fn build_code_output_drafts(value: Option<&Value>) -> Vec<WorkflowCodeOutputDraft> {
    value
        .and_then(Value::as_mapping)
        .map(|items| {
            items
                .iter()
                .filter_map(|(key, value)| {
                    let key = key.as_str()?.trim();
                    if key.is_empty() {
                        return None;
                    }

                    let value_type = value
                        .as_mapping()
                        .and_then(|map| map.get(&yaml_key("type")))
                        .and_then(Value::as_str)
                        .unwrap_or("string");

                    Some(WorkflowCodeOutputDraft {
                        key: key.to_string(),
                        value_type: value_type.to_string(),
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

pub(super) fn build_code_retry_draft(value: Option<&Value>) -> WorkflowNodeRetryDraft {
    let map = value.and_then(Value::as_mapping);
    let max_retries = map
        .and_then(|item| item.get(&yaml_key("max_retries")))
        .and_then(value_as_u64)
        .unwrap_or(3)
        .clamp(1, 10) as u8;
    let retry_interval = map
        .and_then(|item| item.get(&yaml_key("retry_interval")))
        .and_then(value_as_u64)
        .unwrap_or(1000)
        .clamp(100, 5000) as u16;

    WorkflowNodeRetryDraft {
        enabled: map
            .and_then(|item| item.get(&yaml_key("retry_enabled")))
            .and_then(Value::as_bool)
            .unwrap_or(false),
        max_retries,
        retry_interval,
    }
}

pub(super) fn build_code_error_strategy(value: Option<&Value>) -> String {
    match value.and_then(Value::as_str).unwrap_or("none") {
        "default-value" => "default-value".to_string(),
        "fail-branch" => "fail-branch".to_string(),
        _ => "none".to_string(),
    }
}

pub(super) fn code_default_value_editor_text(
    outputs: &[WorkflowCodeOutputDraft],
    existing: Option<&Value>,
    error_strategy: &str,
) -> String {
    if error_strategy != "default-value" {
        return value_yaml_for_editor(existing.unwrap_or(&Value::Sequence(Vec::new())));
    }

    match existing {
        Some(value) if value.is_sequence() && !value.as_sequence().is_some_and(Vec::is_empty) => {
            value_yaml_for_editor(value)
        }
        _ => value_yaml_for_editor(&default_code_default_value_value(outputs)),
    }
}

pub(super) fn default_code_default_value_value(outputs: &[WorkflowCodeOutputDraft]) -> Value {
    Value::Sequence(
        outputs
            .iter()
            .map(|output| {
                yaml_map(vec![
                    ("key", Value::String(output.key.clone())),
                    ("type", Value::String(output.value_type.clone())),
                    ("value", default_code_default_value_for_type(&output.value_type)),
                ])
            })
            .collect(),
    )
}

pub(super) fn parse_code_default_value_yaml(text: &str) -> Result<Value, String> {
    if text.trim().is_empty() {
        return Ok(Value::Sequence(Vec::new()));
    }

    let value = serde_yaml::from_str::<Value>(text)
        .map_err(|error| format!("代码节点 default_value YAML 解析失败: {error}"))?;
    if value.is_sequence() {
        Ok(value)
    } else {
        Err("代码节点 default_value 必须是数组（YAML sequence）".to_string())
    }
}

fn default_code_default_value_for_type(value_type: &str) -> Value {
    match value_type {
        "number" => serde_yaml::to_value(0_u64).unwrap_or(Value::Null),
        "boolean" => Value::Bool(false),
        "object" => Value::Mapping(Mapping::new()),
        "array[number]" | "array[string]" | "array[boolean]" | "array[object]" => {
            Value::Sequence(Vec::new())
        }
        _ => Value::String(String::new()),
    }
}

fn value_as_u64(value: &Value) -> Option<u64> {
    value
        .as_u64()
        .or_else(|| value.as_i64().filter(|number| *number >= 0).map(|number| number as u64))
}
