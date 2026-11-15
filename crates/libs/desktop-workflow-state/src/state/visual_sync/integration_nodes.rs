//! Node-specific visual draft serialization.
use super::*;

pub(super) fn apply(
    data_map: &mut Mapping,
    block_type: &str,
    visual_draft: &WorkflowNodeVisualDraft,
) -> Result<(), desktop_i18n::workflow::Feedback> {
    match (block_type, visual_draft) {
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
                reranking_model_editor,
                metadata_filtering_mode,
                metadata_model_config_editor,
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
            let top_k = top_k_input.trim().parse::<u64>().map_err(|_| {
                desktop_i18n::workflow::Feedback::new("知识检索 top_k 必须是正整数", vec![])
            })?;
            multiple_config.insert(
                yaml_key("top_k"),
                serde_yaml::to_value(top_k).map_err(|error| {
                    desktop_i18n::workflow::Feedback::new(
                        "知识检索 top_k 序列化失败: %s",
                        vec![error.to_string()],
                    )
                })?,
            );
            set_mapping_bool(multiple_config, "reranking_enable", *reranking_enable);
            multiple_config.insert(
                yaml_key("reranking_model"),
                parse_mapping_yaml(&reranking_model_editor.text(), "知识检索 reranking model")?,
            );
            set_mapping_string(multiple_config, "metadata_filtering_mode", metadata_filtering_mode);
            multiple_config.insert(
                yaml_key("metadata_model_config"),
                parse_mapping_yaml(
                    &metadata_model_config_editor.text(),
                    "知识检索 metadata model config",
                )?,
            );
            if *score_threshold_enabled {
                let score_threshold =
                    score_threshold_input.trim().parse::<f64>().map_err(|_| {
                        desktop_i18n::workflow::Feedback::new(
                            "知识检索 score_threshold 必须是数字",
                            vec![],
                        )
                    })?;
                multiple_config.insert(
                    yaml_key("score_threshold"),
                    serde_yaml::to_value(score_threshold).map_err(|error| {
                        desktop_i18n::workflow::Feedback::new(
                            "知识检索 score_threshold 序列化失败: %s",
                            vec![error.to_string()],
                        )
                    })?,
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
            let window_size = memory_window_size_input.trim().parse::<u64>().map_err(|_| {
                desktop_i18n::workflow::Feedback::new(
                    "Agent memory window size 必须是正整数",
                    vec![],
                )
            })?;
            window_map.insert(
                yaml_key("size"),
                serde_yaml::to_value(window_size).map_err(|error| {
                    desktop_i18n::workflow::Feedback::new(
                        "Agent memory window size 序列化失败: %s",
                        vec![error.to_string()],
                    )
                })?,
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
                serde_yaml::to_value(retry_config.max_retries).map_err(|error| {
                    desktop_i18n::workflow::Feedback::new(
                        "代码节点 max_retries 序列化失败: %s",
                        vec![error.to_string()],
                    )
                })?,
            );
            retry_map.insert(
                yaml_key("retry_interval"),
                serde_yaml::to_value(retry_config.retry_interval).map_err(|error| {
                    desktop_i18n::workflow::Feedback::new(
                        "代码节点 retry_interval 序列化失败: %s",
                        vec![error.to_string()],
                    )
                })?,
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
    Ok(())
}
