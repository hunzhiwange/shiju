//! Node-specific visual draft serialization.
use super::*;

pub(super) fn apply(
    data_map: &mut Mapping,
    block_type: &str,
    visual_draft: &WorkflowNodeVisualDraft,
) -> Result<(), desktop_i18n::workflow::Feedback> {
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
                completion_params_editor,
                structured_output_editor,
                memory_editor,
                context_enabled,
                context_selector_input,
                system_prompt_editor,
                user_prompt_editor,
                vision_enabled,
                vision_selector_input,
            },
        ) => {
            let model_map = ensure_mapping_entry(data_map, "model");
            set_mapping_string(model_map, "provider", provider);
            set_mapping_string(model_map, "name", model_name);
            set_mapping_string(model_map, "mode", model_mode);

            let completion_params_value =
                parse_mapping_yaml(&completion_params_editor.text(), "LLM completion params")?;
            let mut completion_params = completion_params_value
                .as_mapping()
                .cloned()
                .expect("parse_mapping_yaml returns a mapping");
            set_mapping_bool(&mut completion_params, "enable_thinking", *enable_thinking);
            model_map.insert(yaml_key("completion_params"), Value::Mapping(completion_params));
            data_map.insert(
                yaml_key("structured_output"),
                parse_mapping_yaml(&structured_output_editor.text(), "LLM structured output")?,
            );
            data_map.insert(
                yaml_key("memory"),
                parse_mapping_yaml(&memory_editor.text(), "LLM memory")?,
            );

            let context_map = ensure_mapping_entry(data_map, "context");
            set_mapping_bool(context_map, "enabled", *context_enabled);
            context_map.insert(
                yaml_key("variable_selector"),
                selector_value_from_input(context_selector_input),
            );

            let vision_map = ensure_mapping_entry(data_map, "vision");
            set_mapping_bool(vision_map, "enabled", *vision_enabled);
            vision_map.insert(
                yaml_key("variable_selector"),
                selector_path_value_from_input(vision_selector_input),
            );

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
                let value = input.trim().parse::<u64>().map_err(|_| {
                    desktop_i18n::workflow::Feedback::new(
                        "HTTP %s timeout 必须是正整数",
                        vec![key.to_string()],
                    )
                })?;
                timeout.insert(
                    yaml_key(key),
                    serde_yaml::to_value(value).map_err(|e| {
                        desktop_i18n::workflow::Feedback::new(
                            "HTTP timeout 序列化失败: %s",
                            vec![e.to_string()],
                        )
                    })?,
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
        _ => {}
    }
    Ok(())
}
