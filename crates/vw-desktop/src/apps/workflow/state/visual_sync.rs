//! # Workflow 可视化同步
//!
//! 该模块将节点原始 YAML 解析成可视化编辑草稿，并将可视化草稿同步回 YAML。

use super::*;

pub(super) fn apply_visual_draft_to_yaml(
    block_type: &str,
    raw_data_yaml: &str,
    visual_draft: Option<&WorkflowNodeVisualDraft>,
) -> Result<String, String> {
    let Some(visual_draft) = visual_draft else {
        return Ok(raw_data_yaml.to_string());
    };

    let mut data_value = parse_node_data_yaml_value(raw_data_yaml)?;
    let data_map = data_value
        .as_mapping_mut()
        .ok_or_else(|| "节点 data 必须是对象映射（YAML map）".to_string())?;

    match (block_type, visual_draft) {
        ("start", WorkflowNodeVisualDraft::Start { variables }) => {
            data_map.insert(
                yaml_key("variables"),
                Value::Sequence(
                    variables
                        .iter()
                        .map(merge_start_variable_value)
                        .collect::<Result<Vec<_>, _>>()?,
                ),
            );
        }
        (
            "llm",
            WorkflowNodeVisualDraft::Llm {
                provider,
                model_name,
                model_mode,
                enable_thinking,
                context_enabled,
                context_selector_input,
                system_prompt_editor,
                user_prompt_editor,
                vision_enabled,
            },
        ) => {
            let model_map = ensure_mapping_entry(data_map, "model");
            set_mapping_string(model_map, "provider", provider);
            set_mapping_string(model_map, "name", model_name);
            set_mapping_string(model_map, "mode", model_mode);

            let completion_params = ensure_mapping_entry(model_map, "completion_params");
            set_mapping_bool(completion_params, "enable_thinking", *enable_thinking);

            let context_map = ensure_mapping_entry(data_map, "context");
            set_mapping_bool(context_map, "enabled", *context_enabled);
            context_map.insert(
                yaml_key("variable_selector"),
                selector_value_from_input(context_selector_input),
            );

            let vision_map = ensure_mapping_entry(data_map, "vision");
            set_mapping_bool(vision_map, "enabled", *vision_enabled);

            let prompt_value = data_map
                .remove(&yaml_key("prompt_template"))
                .unwrap_or_else(|| Value::Sequence(Vec::new()));
            let merged_prompt = merge_prompt_template_value(
                prompt_value,
                system_prompt_editor.text(),
                user_prompt_editor.text(),
            );
            data_map.insert(yaml_key("prompt_template"), merged_prompt);
        }
        ("answer", WorkflowNodeVisualDraft::Answer { answer_editor }) => {
            set_mapping_string(data_map, "answer", &answer_editor.text());
        }
        (
            "http-request",
            WorkflowNodeVisualDraft::HttpRequest {
                method,
                url,
                auth_type,
                auth_username,
                auth_password,
                auth_token,
                headers_editor,
                params_editor,
                body_type,
                body_editor,
                timeout_connect,
                timeout_read,
                timeout_write,
                ssl_verify,
                retry_config,
                error_strategy,
            },
        ) => {
            set_mapping_string(data_map, "method", &method.to_ascii_lowercase());
            set_mapping_string(data_map, "url", url);
            let auth = ensure_mapping_entry(data_map, "authorization");
            set_mapping_string(auth, "type", auth_type);
            match auth_type.as_str() {
                "basic" => {
                    set_mapping_string(auth, "username", auth_username);
                    set_mapping_string(auth, "password", auth_password);
                    auth.remove(&yaml_key("token"));
                }
                "bearer" | "api-key" => {
                    set_mapping_string(auth, "token", auth_token);
                    auth.remove(&yaml_key("username"));
                    auth.remove(&yaml_key("password"));
                }
                _ => {
                    auth.remove(&yaml_key("username"));
                    auth.remove(&yaml_key("password"));
                    auth.remove(&yaml_key("token"));
                }
            }
            data_map
                .insert(yaml_key("headers"), parse_http_value(&headers_editor.text(), "请求头")?);
            data_map
                .insert(yaml_key("params"), parse_http_value(&params_editor.text(), "查询参数")?);
            let body = ensure_mapping_entry(data_map, "body");
            set_mapping_string(body, "type", body_type);
            body.insert(yaml_key("data"), parse_http_value(&body_editor.text(), "请求体")?);
            let timeout = ensure_mapping_entry(data_map, "timeout");
            for (key, input) in
                [("connect", timeout_connect), ("read", timeout_read), ("write", timeout_write)]
            {
                let value = input
                    .trim()
                    .parse::<u64>()
                    .map_err(|_| format!("HTTP {key} timeout 必须是正整数"))?;
                timeout.insert(
                    yaml_key(key),
                    serde_yaml::to_value(value)
                        .map_err(|e| format!("HTTP timeout 序列化失败: {e}"))?,
                );
            }
            set_mapping_bool(data_map, "ssl_verify", *ssl_verify);
            let retry = ensure_mapping_entry(data_map, "retry_config");
            set_mapping_bool(retry, "retry_enabled", retry_config.enabled);
            retry.insert(
                yaml_key("max_retries"),
                serde_yaml::to_value(retry_config.max_retries).map_err(|e| e.to_string())?,
            );
            retry.insert(
                yaml_key("retry_interval"),
                serde_yaml::to_value(retry_config.retry_interval).map_err(|e| e.to_string())?,
            );
            set_mapping_string(data_map, "error_strategy", error_strategy);
        }
        ("end", WorkflowNodeVisualDraft::End { outputs }) => {
            data_map.insert(yaml_key("outputs"), named_values_value(outputs, true));
        }
        (
            "template-transform",
            WorkflowNodeVisualDraft::TemplateTransform { variables, template_editor },
        ) => {
            data_map.insert(yaml_key("variables"), named_values_value(variables, false));
            set_mapping_string(data_map, "template", &template_editor.text());
        }
        (
            "document-extractor",
            WorkflowNodeVisualDraft::DocumentExtractor { variable_selector },
        ) => {
            data_map.insert(
                yaml_key("variable_selector"),
                Value::Sequence(
                    variable_selector.path.iter().cloned().map(Value::String).collect(),
                ),
            );
        }
        ("variable-assigner", WorkflowNodeVisualDraft::VariableAssigner { items }) => {
            data_map.insert(
                yaml_key("items"),
                Value::Sequence(
                    items
                        .iter()
                        .map(|item| {
                            let mut map = item.raw_item.as_mapping().cloned().unwrap_or_default();
                            map.insert(
                                yaml_key("variable_selector"),
                                selector_path_value(&item.variable_selector),
                            );
                            set_mapping_string(&mut map, "operation", &item.operation);
                            set_mapping_string(&mut map, "input_type", &item.input_type);
                            map.insert(yaml_key("value"), item.value.clone());
                            Value::Mapping(map)
                        })
                        .collect(),
                ),
            );
        }
        (
            "variable-aggregator",
            WorkflowNodeVisualDraft::VariableAggregator { variables, output_type },
        ) => {
            data_map.insert(
                yaml_key("variables"),
                Value::Sequence(
                    variables
                        .iter()
                        .map(|item| {
                            selector_path_value(&WorkflowVariableSelectorDraft::from_input(item))
                        })
                        .collect(),
                ),
            );
            set_mapping_string(data_map, "output_type", output_type);
        }
        (
            "list-operator",
            WorkflowNodeVisualDraft::ListOperator {
                variable,
                conditions,
                filter_enabled,
                order_enabled,
                order_key,
                order_direction,
                limit_enabled,
                limit_size,
            },
        ) => {
            data_map.insert(
                yaml_key("variable"),
                selector_path_value(&WorkflowVariableSelectorDraft::from_input(variable)),
            );
            let filter = ensure_mapping_entry(data_map, "filter_by");
            set_mapping_bool(filter, "enabled", *filter_enabled);
            filter.insert(
                yaml_key("conditions"),
                Value::Sequence(
                    conditions
                        .iter()
                        .map(|condition| {
                            let mut map =
                                condition.raw_condition.as_mapping().cloned().unwrap_or_default();
                            set_mapping_string(&mut map, "key", &condition.key);
                            set_mapping_string(
                                &mut map,
                                "comparison_operator",
                                &condition.comparison_operator,
                            );
                            map.insert(yaml_key("value"), condition.value.clone());
                            Value::Mapping(map)
                        })
                        .collect(),
                ),
            );
            let order = ensure_mapping_entry(data_map, "order_by");
            set_mapping_bool(order, "enabled", *order_enabled);
            set_mapping_string(order, "key", order_key);
            set_mapping_string(order, "value", order_direction);
            let limit = ensure_mapping_entry(data_map, "limit");
            set_mapping_bool(limit, "enabled", *limit_enabled);
            let size = limit_size
                .trim()
                .parse::<u64>()
                .map_err(|_| "列表截取数量必须是非负整数".to_string())?;
            limit.insert(yaml_key("size"), serde_yaml::to_value(size).map_err(|e| e.to_string())?);
        }
        (
            "question-classifier",
            WorkflowNodeVisualDraft::QuestionClassifier {
                query,
                model,
                classes,
                instruction_editor,
            },
        ) => {
            data_map.insert(yaml_key("query_variable_selector"), selector_path_value(query));
            merge_model_value(data_map, model);
            data_map.insert(
                yaml_key("classes"),
                Value::Sequence(
                    classes
                        .iter()
                        .map(|item| {
                            let mut map = item.raw_class.as_mapping().cloned().unwrap_or_default();
                            set_mapping_string(&mut map, "id", &item.id);
                            set_mapping_string(&mut map, "name", &item.name);
                            Value::Mapping(map)
                        })
                        .collect(),
                ),
            );
            set_mapping_string(data_map, "instruction", &instruction_editor.text());
        }
        (
            "parameter-extractor",
            WorkflowNodeVisualDraft::ParameterExtractor {
                query,
                model,
                parameters,
                reasoning_mode,
                instruction_editor,
            },
        ) => {
            data_map.insert(yaml_key("query"), selector_path_value(query));
            merge_model_value(data_map, model);
            data_map.insert(
                yaml_key("parameters"),
                Value::Sequence(
                    parameters
                        .iter()
                        .map(|item| {
                            let mut map =
                                item.raw_parameter.as_mapping().cloned().unwrap_or_default();
                            set_mapping_string(&mut map, "name", &item.name);
                            set_mapping_string(&mut map, "type", &item.value_type);
                            set_mapping_string(&mut map, "description", &item.description);
                            set_mapping_bool(&mut map, "required", item.required);
                            Value::Mapping(map)
                        })
                        .collect(),
                ),
            );
            set_mapping_string(data_map, "reasoning_mode", reasoning_mode);
            set_mapping_string(data_map, "instruction", &instruction_editor.text());
        }
        ("if-else", WorkflowNodeVisualDraft::IfElse { cases }) => {
            data_map.insert(
                yaml_key("cases"),
                Value::Sequence(
                    cases.iter().map(merge_if_else_case_value).collect::<Result<Vec<_>, _>>()?,
                ),
            );
        }
        (
            "knowledge-retrieval",
            WorkflowNodeVisualDraft::KnowledgeRetrieval {
                query_selector_input,
                query_attachment_selector_input,
                dataset_ids_input,
                retrieval_mode,
                top_k_input,
                score_threshold_enabled,
                score_threshold_input,
                reranking_enable,
                single_model_provider,
                single_model_name,
                single_model_mode,
            },
        ) => {
            data_map.insert(
                yaml_key("query_variable_selector"),
                selector_path_value_from_input(query_selector_input),
            );
            data_map.insert(
                yaml_key("query_attachment_selector"),
                selector_path_value_from_input(query_attachment_selector_input),
            );
            data_map
                .insert(yaml_key("dataset_ids"), string_list_value_from_input(dataset_ids_input));
            set_mapping_string(data_map, "retrieval_mode", retrieval_mode);

            let multiple_config = ensure_mapping_entry(data_map, "multiple_retrieval_config");
            let top_k = top_k_input
                .trim()
                .parse::<u64>()
                .map_err(|_| "知识检索 top_k 必须是正整数".to_string())?;
            multiple_config.insert(
                yaml_key("top_k"),
                serde_yaml::to_value(top_k)
                    .map_err(|error| format!("知识检索 top_k 序列化失败: {error}"))?,
            );
            set_mapping_bool(multiple_config, "reranking_enable", *reranking_enable);
            if *score_threshold_enabled {
                let score_threshold = score_threshold_input
                    .trim()
                    .parse::<f64>()
                    .map_err(|_| "知识检索 score_threshold 必须是数字".to_string())?;
                multiple_config.insert(
                    yaml_key("score_threshold"),
                    serde_yaml::to_value(score_threshold)
                        .map_err(|error| format!("知识检索 score_threshold 序列化失败: {error}"))?,
                );
            } else {
                multiple_config.insert(yaml_key("score_threshold"), Value::Null);
            }

            let single_model = ensure_mapping_entry(
                ensure_mapping_entry(data_map, "single_retrieval_config"),
                "model",
            );
            set_mapping_string(single_model, "provider", single_model_provider);
            set_mapping_string(single_model, "name", single_model_name);
            set_mapping_string(single_model, "mode", single_model_mode);
            let completion_params = ensure_mapping_entry(single_model, "completion_params");
            if completion_params.is_empty() {
                *completion_params = Mapping::new();
            }
        }
        (
            "tool",
            WorkflowNodeVisualDraft::Tool {
                provider_id,
                provider_type,
                provider_name,
                tool_name,
                tool_label,
                tool_description,
                credential_id,
                plugin_unique_identifier,
                tool_parameters_editor,
                tool_configurations_editor,
            },
        ) => {
            set_mapping_string(data_map, "provider_id", provider_id);
            set_mapping_string(data_map, "provider_type", provider_type);
            set_mapping_string(data_map, "provider_name", provider_name);
            set_mapping_string(data_map, "tool_name", tool_name);
            set_mapping_string(data_map, "tool_label", tool_label);
            set_mapping_string(data_map, "tool_description", tool_description);
            if credential_id.trim().is_empty() {
                data_map.remove(&yaml_key("credential_id"));
            } else {
                set_mapping_string(data_map, "credential_id", credential_id);
            }
            if plugin_unique_identifier.trim().is_empty() {
                data_map.remove(&yaml_key("plugin_unique_identifier"));
            } else {
                set_mapping_string(data_map, "plugin_unique_identifier", plugin_unique_identifier);
            }
            set_mapping_string(data_map, "tool_node_version", "2");
            data_map.insert(
                yaml_key("tool_parameters"),
                parse_mapping_yaml(&tool_parameters_editor.text(), "工具参数")?,
            );
            data_map.insert(
                yaml_key("tool_configurations"),
                parse_mapping_yaml(&tool_configurations_editor.text(), "工具配置")?,
            );
        }
        (
            "agent",
            WorkflowNodeVisualDraft::Agent {
                strategy_provider_name,
                strategy_name,
                strategy_label,
                plugin_unique_identifier,
                output_schema_editor,
                parameters_editor,
                memory_enabled,
                memory_window_size_input,
                memory_prompt_editor,
            },
        ) => {
            set_mapping_string(data_map, "agent_strategy_provider_name", strategy_provider_name);
            set_mapping_string(data_map, "agent_strategy_name", strategy_name);
            set_mapping_string(data_map, "agent_strategy_label", strategy_label);
            if plugin_unique_identifier.trim().is_empty() {
                data_map.remove(&yaml_key("plugin_unique_identifier"));
            } else {
                set_mapping_string(data_map, "plugin_unique_identifier", plugin_unique_identifier);
            }
            set_mapping_string(data_map, "tool_node_version", "2");
            data_map.insert(
                yaml_key("output_schema"),
                parse_mapping_yaml(&output_schema_editor.text(), "Agent 输出结构")?,
            );
            data_map.insert(
                yaml_key("agent_parameters"),
                parse_mapping_yaml(&parameters_editor.text(), "Agent 参数")?,
            );

            let memory_map = ensure_mapping_entry(data_map, "memory");
            let window_map = ensure_mapping_entry(memory_map, "window");
            set_mapping_bool(window_map, "enabled", *memory_enabled);
            let window_size = memory_window_size_input
                .trim()
                .parse::<u64>()
                .map_err(|_| "Agent memory window size 必须是正整数".to_string())?;
            window_map.insert(
                yaml_key("size"),
                serde_yaml::to_value(window_size)
                    .map_err(|error| format!("Agent memory window size 序列化失败: {error}"))?,
            );
            set_mapping_string(memory_map, "query_prompt_template", &memory_prompt_editor.text());
        }
        (
            "code",
            WorkflowNodeVisualDraft::Code {
                language,
                inputs,
                code_editor,
                outputs,
                retry_config,
                error_strategy,
                default_value_editor,
            },
        ) => {
            set_mapping_string(data_map, "code_language", language);
            set_mapping_string(data_map, "code", &code_editor.text());
            data_map.insert(
                yaml_key("variables"),
                Value::Sequence(inputs.iter().map(code_variable_value).collect()),
            );
            data_map.insert(yaml_key("outputs"), code_outputs_value(outputs));

            let mut retry_map = Mapping::new();
            set_mapping_bool(&mut retry_map, "retry_enabled", retry_config.enabled);
            retry_map.insert(
                yaml_key("max_retries"),
                serde_yaml::to_value(retry_config.max_retries)
                    .map_err(|error| format!("代码节点 max_retries 序列化失败: {error}"))?,
            );
            retry_map.insert(
                yaml_key("retry_interval"),
                serde_yaml::to_value(retry_config.retry_interval)
                    .map_err(|error| format!("代码节点 retry_interval 序列化失败: {error}"))?,
            );
            data_map.insert(yaml_key("retry_config"), Value::Mapping(retry_map));

            match error_strategy.as_str() {
                "default-value" => {
                    set_mapping_string(data_map, "error_strategy", error_strategy);
                    data_map.insert(
                        yaml_key("default_value"),
                        parse_code_default_value_yaml(&default_value_editor.text())?,
                    );
                }
                "fail-branch" => {
                    set_mapping_string(data_map, "error_strategy", error_strategy);
                    data_map.remove(&yaml_key("default_value"));
                }
                _ => {
                    data_map.remove(&yaml_key("error_strategy"));
                    data_map.remove(&yaml_key("default_value"));
                }
            }
        }
        _ => {}
    }

    let yaml = serde_yaml::to_string(&data_value)
        .map_err(|error| format!("生成节点 data YAML 失败: {error}"))?;
    Ok(yaml.strip_prefix("---\n").unwrap_or(&yaml).to_string())
}

pub(super) fn parse_http_value(input: &str, label: &str) -> Result<Value, String> {
    if input.trim().is_empty() {
        return Ok(Value::String(String::new()));
    }
    serde_yaml::from_str(input).map_err(|_| format!("{label}必须是合法 YAML"))
}

fn named_values_value(values: &[WorkflowNamedValueDraft], include_type: bool) -> Value {
    Value::Sequence(
        values
            .iter()
            .map(|item| {
                let mut map = Mapping::new();
                if !item.id.is_empty() {
                    set_mapping_string(&mut map, "id", &item.id);
                }
                set_mapping_string(&mut map, "variable", &item.name);
                if include_type {
                    set_mapping_string(&mut map, "value_type", &item.value_type);
                }
                map.insert(
                    yaml_key("value_selector"),
                    Value::Sequence(
                        item.selector.path.iter().cloned().map(Value::String).collect(),
                    ),
                );
                Value::Mapping(map)
            })
            .collect(),
    )
}

fn selector_path_value(selector: &WorkflowVariableSelectorDraft) -> Value {
    Value::Sequence(selector.path.iter().cloned().map(Value::String).collect())
}

fn merge_model_value(data_map: &mut Mapping, model: &WorkflowModelDraft) {
    let model_map = ensure_mapping_entry(data_map, "model");
    set_mapping_string(model_map, "provider", &model.provider);
    set_mapping_string(model_map, "name", &model.name);
    set_mapping_string(model_map, "mode", &model.mode);
}

pub(super) fn parse_node_data_yaml_value(text: &str) -> Result<Value, String> {
    if text.trim().is_empty() {
        return Ok(Value::Mapping(Mapping::new()));
    }

    let value = serde_yaml::from_str::<Value>(text)
        .map_err(|error| format!("节点 data YAML 解析失败: {error}"))?;
    if value.is_mapping() {
        Ok(value)
    } else {
        Err("节点 data 必须是对象映射（YAML map）".to_string())
    }
}

pub(super) fn ensure_root_mapping(value: Value) -> Value {
    if value.is_mapping() { value } else { Value::Mapping(Mapping::new()) }
}

pub(super) fn selector_input_from_value(value: Option<&Value>) -> String {
    value
        .and_then(Value::as_sequence)
        .map(|selectors| {
            selectors
                .iter()
                .filter_map(Value::as_sequence)
                .map(|parts| parts.iter().filter_map(Value::as_str).collect::<Vec<_>>().join("."))
                .filter(|selector| !selector.trim().is_empty())
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_default()
}

pub(super) fn selector_path_input_from_value(value: Option<&Value>) -> String {
    value
        .and_then(Value::as_sequence)
        .map(|parts| parts.iter().filter_map(Value::as_str).collect::<Vec<_>>().join("."))
        .unwrap_or_default()
}

pub(super) fn selector_value_from_input(input: &str) -> Value {
    let selectors = input
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(|item| {
            Value::Sequence(
                item.split('.')
                    .map(str::trim)
                    .filter(|part| !part.is_empty())
                    .map(|part| Value::String(part.to_string()))
                    .collect(),
            )
        })
        .collect::<Vec<_>>();
    Value::Sequence(selectors)
}

pub(super) fn selector_path_value_from_input(input: &str) -> Value {
    Value::Sequence(
        input
            .split('.')
            .map(str::trim)
            .filter(|part| !part.is_empty())
            .map(|part| Value::String(part.to_string()))
            .collect(),
    )
}

fn code_variable_value(variable: &WorkflowCodeVariableDraft) -> Value {
    yaml_map(vec![
        ("variable", Value::String(variable.variable.clone())),
        (
            "value_selector",
            Value::Sequence(
                variable.selector.iter().map(|part| Value::String(part.clone())).collect(),
            ),
        ),
        ("value_type", Value::String(variable.value_type.clone())),
    ])
}

fn code_outputs_value(outputs: &[WorkflowCodeOutputDraft]) -> Value {
    let mut mapping = Mapping::new();

    for output in outputs {
        mapping.insert(
            yaml_key(&output.key),
            yaml_map(vec![
                ("children", Value::Null),
                ("type", Value::String(output.value_type.clone())),
            ]),
        );
    }

    Value::Mapping(mapping)
}

#[cfg(test)]
#[path = "visual_sync_tests.rs"]
mod visual_sync_tests;
