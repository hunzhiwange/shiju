//! # Workflow 节点校验
//!
//! 该模块对各类节点可视化草稿与高级 YAML 输入做字段级校验并生成错误列表。

use super::*;

pub(super) fn validate_node_editor_draft(
    block_type: &str,
    _title: &str,
    _description: &str,
    raw_data_yaml: &str,
    visual_draft: Option<&WorkflowNodeVisualDraft>,
) -> WorkflowNodeEditorValidation {
    let mut field_errors = Vec::new();

    let parse_result = if visual_draft.is_some() {
        apply_visual_draft_to_yaml(block_type, raw_data_yaml, visual_draft)
            .map(|_| Value::Mapping(Mapping::new()))
    } else {
        parse_node_data_yaml_value(raw_data_yaml)
    };

    if let Err(error) = parse_result {
        push_validation_error(&mut field_errors, "advanced_yaml.raw_data", error);
    }

    match visual_draft {
        Some(WorkflowNodeVisualDraft::Start { variables }) => {
            for (index, variable) in variables.iter().enumerate() {
                if variable.label.trim().is_empty() {
                    push_validation_error(
                        &mut field_errors,
                        &format!("start.variables[{index}].label"),
                        "变量显示名称不能为空",
                    );
                }
                if variable.variable.trim().is_empty() {
                    push_validation_error(
                        &mut field_errors,
                        &format!("start.variables[{index}].variable"),
                        "变量名不能为空",
                    );
                }
                if variable.input_type == "file-list" {
                    if !is_valid_start_variable_file_list_max_length(&variable.max_length_input) {
                        push_validation_error(
                            &mut field_errors,
                            &format!("start.variables[{index}].max_length"),
                            "最大上传数必须在 1 到 10 之间",
                        );
                    }
                } else if !variable.max_length_input.trim().is_empty()
                    && variable.max_length_input.trim().parse::<u64>().is_err()
                {
                    push_validation_error(
                        &mut field_errors,
                        &format!("start.variables[{index}].max_length"),
                        "最大长度必须是非负整数",
                    );
                }

                if variable.input_type == "select" {
                    let options = variable
                        .options
                        .iter()
                        .map(|item| item.trim())
                        .filter(|item| !item.is_empty())
                        .collect::<Vec<_>>();

                    if options.is_empty() {
                        push_validation_error(
                            &mut field_errors,
                            &format!("start.variables[{index}].options"),
                            "下拉选项至少需要一个有效选项",
                        );
                    }

                    let mut seen = std::collections::HashSet::new();
                    if options.iter().any(|item| !seen.insert((*item).to_string())) {
                        push_validation_error(
                            &mut field_errors,
                            &format!("start.variables[{index}].options"),
                            "下拉选项不能重复",
                        );
                    }

                    if !variable.default_value.trim().is_empty()
                        && !options.iter().any(|item| *item == variable.default_value.trim())
                    {
                        push_validation_error(
                            &mut field_errors,
                            &format!("start.variables[{index}].default"),
                            "默认值必须在下拉选项中",
                        );
                    }
                }

                if variable.input_type == "number"
                    && !is_valid_start_variable_number_default_value(&variable.default_value)
                {
                    push_validation_error(
                        &mut field_errors,
                        &format!("start.variables[{index}].default"),
                        "数字类型默认值必须是数字",
                    );
                }

                if matches!(variable.input_type.as_str(), "file" | "file-list") {
                    if variable.allowed_file_types.is_empty() {
                        push_validation_error(
                            &mut field_errors,
                            &format!("start.variables[{index}].allowed_file_types"),
                            "至少选择一种支持的文件类型",
                        );
                    }

                    if variable.allowed_file_types.iter().any(|item| item == "custom")
                        && variable.allowed_file_extensions.is_empty()
                    {
                        push_validation_error(
                            &mut field_errors,
                            &format!("start.variables[{index}].allowed_file_extensions"),
                            "选择自定义文件类型时，必须填写扩展名",
                        );
                    }

                    if variable.allowed_file_upload_methods.is_empty() {
                        push_validation_error(
                            &mut field_errors,
                            &format!("start.variables[{index}].allowed_file_upload_methods"),
                            "至少选择一种上传方式",
                        );
                    }

                    if variable.input_type == "file-list"
                        && variable.default_file_values.len()
                            > usize::from(normalized_start_variable_file_list_max_length(
                                &variable.max_length_input,
                            ))
                    {
                        push_validation_error(
                            &mut field_errors,
                            &format!("start.variables[{index}].default"),
                            "默认文件数量不能超过最大上传数",
                        );
                    }
                }
            }
        }
        Some(WorkflowNodeVisualDraft::IfElse { cases }) => {
            for (case_index, case) in cases.iter().enumerate() {
                if case.logical_operator.trim().is_empty() {
                    push_validation_error(
                        &mut field_errors,
                        &format!("if_else.cases[{case_index}].logical_operator"),
                        "逻辑运算不能为空",
                    );
                }
                if case.conditions.is_empty() {
                    push_validation_error(
                        &mut field_errors,
                        &format!("if_else.cases[{case_index}].conditions"),
                        "至少需要一条条件",
                    );
                }
                for (condition_index, condition) in case.conditions.iter().enumerate() {
                    if condition.variable_selector_input.trim().is_empty() {
                        push_validation_error(
                            &mut field_errors,
                            &format!(
                                "if_else.cases[{case_index}].conditions[{condition_index}].selector"
                            ),
                            "变量选择器不能为空",
                        );
                    }
                    if condition.comparison_operator.trim().is_empty() {
                        push_validation_error(
                            &mut field_errors,
                            &format!(
                                "if_else.cases[{case_index}].conditions[{condition_index}].operator"
                            ),
                            "比较符不能为空",
                        );
                    }
                    if condition.var_type.trim().is_empty() {
                        push_validation_error(
                            &mut field_errors,
                            &format!(
                                "if_else.cases[{case_index}].conditions[{condition_index}].var_type"
                            ),
                            "变量类型不能为空",
                        );
                    }
                    let operator = condition.comparison_operator.trim().to_ascii_lowercase();
                    if !matches!(operator.as_str(), "empty" | "not empty")
                        && condition.compare_value.trim().is_empty()
                    {
                        push_validation_error(
                            &mut field_errors,
                            &format!(
                                "if_else.cases[{case_index}].conditions[{condition_index}].value"
                            ),
                            "当前比较符需要填写比较值",
                        );
                    }
                }
            }
        }
        Some(WorkflowNodeVisualDraft::End { outputs }) => {
            validate_named_values(&mut field_errors, "end.outputs", outputs, true);
        }
        Some(WorkflowNodeVisualDraft::TemplateTransform { variables, template_editor }) => {
            validate_named_values(&mut field_errors, "template.variables", variables, false);
            if template_editor.text().trim().is_empty() {
                push_validation_error(&mut field_errors, "template.template", "模板内容不能为空");
            }
        }
        Some(WorkflowNodeVisualDraft::DocumentExtractor { variable_selector }) => {
            if variable_selector.path.is_empty() {
                push_validation_error(
                    &mut field_errors,
                    "document_extractor.variable_selector",
                    "文件变量选择器不能为空",
                );
            }
        }
        Some(WorkflowNodeVisualDraft::QuestionClassifier { query, model, classes, .. }) => {
            validate_structured_model(&mut field_errors, "classifier", query, model);
            if classes.is_empty() {
                push_validation_error(&mut field_errors, "classifier.classes", "至少需要一个分类");
            }
            let mut ids = std::collections::HashSet::new();
            for (index, class) in classes.iter().enumerate() {
                if class.id.trim().is_empty() {
                    push_validation_error(
                        &mut field_errors,
                        &format!("classifier.classes[{index}].id"),
                        "分类 ID 不能为空",
                    );
                } else if !ids.insert(class.id.trim().to_string()) {
                    push_validation_error(
                        &mut field_errors,
                        &format!("classifier.classes[{index}].id"),
                        "分类 ID 不能重复",
                    );
                }
                if class.name.trim().is_empty() {
                    push_validation_error(
                        &mut field_errors,
                        &format!("classifier.classes[{index}].name"),
                        "分类名称不能为空",
                    );
                }
            }
        }
        Some(WorkflowNodeVisualDraft::ParameterExtractor {
            query,
            model,
            parameters,
            reasoning_mode,
            ..
        }) => {
            validate_structured_model(&mut field_errors, "parameter_extractor", query, model);
            if reasoning_mode.trim().is_empty() {
                push_validation_error(
                    &mut field_errors,
                    "parameter_extractor.reasoning_mode",
                    "推理模式不能为空",
                );
            }
            let mut names = std::collections::HashSet::new();
            for (index, parameter) in parameters.iter().enumerate() {
                let name = parameter.name.trim();
                let path = format!("parameter_extractor.parameters[{index}].name");
                if name.is_empty() {
                    push_validation_error(&mut field_errors, &path, "参数名不能为空");
                } else if matches!(name, "__is_success" | "__reason") {
                    push_validation_error(&mut field_errors, &path, "参数名为系统保留名");
                } else if !names.insert(name.to_string()) {
                    push_validation_error(&mut field_errors, &path, "参数名不能重复");
                }
                if !matches!(
                    parameter.value_type.trim(),
                    "string" | "number" | "boolean" | "array" | "object"
                ) {
                    push_validation_error(
                        &mut field_errors,
                        &format!("parameter_extractor.parameters[{index}].type"),
                        "参数类型不受支持",
                    );
                }
            }
        }
        Some(WorkflowNodeVisualDraft::KnowledgeRetrieval {
            query_selector_input,
            query_attachment_selector_input,
            dataset_ids_input,
            retrieval_mode,
            top_k_input,
            score_threshold_enabled,
            score_threshold_input,
            single_model_provider,
            single_model_name,
            single_model_mode,
            ..
        }) => {
            if dataset_ids_input.split(',').map(str::trim).all(|item| item.is_empty()) {
                push_validation_error(
                    &mut field_errors,
                    "knowledge.dataset_ids",
                    "至少填写一个知识库 ID",
                );
            }
            if query_selector_input.trim().is_empty()
                && query_attachment_selector_input.trim().is_empty()
            {
                push_validation_error(
                    &mut field_errors,
                    "knowledge.query_selector",
                    "查询变量和附件变量至少填写一个",
                );
            }
            let mode = retrieval_mode.trim();
            if !matches!(mode, "single" | "multiple") {
                push_validation_error(
                    &mut field_errors,
                    "knowledge.retrieval_mode",
                    "检索模式只能是 single 或 multiple",
                );
            }
            if mode == "multiple" {
                if top_k_input.trim().parse::<u64>().ok().filter(|value| *value > 0).is_none() {
                    push_validation_error(
                        &mut field_errors,
                        "knowledge.multiple.top_k",
                        "top_k 必须是正整数",
                    );
                }
                if *score_threshold_enabled {
                    match score_threshold_input.trim().parse::<f64>() {
                        Ok(value) if (0.0..=1.0).contains(&value) => {}
                        _ => push_validation_error(
                            &mut field_errors,
                            "knowledge.multiple.score_threshold",
                            "score_threshold 必须是 0 到 1 之间的数字",
                        ),
                    }
                }
            }
            if mode == "single" {
                if single_model_provider.trim().is_empty() {
                    push_validation_error(
                        &mut field_errors,
                        "knowledge.single.provider",
                        "单路检索模型 provider 不能为空",
                    );
                }
                if single_model_name.trim().is_empty() {
                    push_validation_error(
                        &mut field_errors,
                        "knowledge.single.model_name",
                        "单路检索模型名称不能为空",
                    );
                }
                if single_model_mode.trim().is_empty() {
                    push_validation_error(
                        &mut field_errors,
                        "knowledge.single.model_mode",
                        "单路检索模型 mode 不能为空",
                    );
                }
            }
        }
        Some(WorkflowNodeVisualDraft::Tool {
            provider_id,
            provider_type,
            provider_name,
            tool_name,
            tool_parameters_editor,
            tool_configurations_editor,
            ..
        }) => {
            if provider_id.trim().is_empty() {
                push_validation_error(
                    &mut field_errors,
                    "tool.provider_id",
                    "provider_id 不能为空",
                );
            }
            if provider_type.trim().is_empty() {
                push_validation_error(
                    &mut field_errors,
                    "tool.provider_type",
                    "provider_type 不能为空",
                );
            }
            if provider_name.trim().is_empty() {
                push_validation_error(
                    &mut field_errors,
                    "tool.provider_name",
                    "provider_name 不能为空",
                );
            }
            if tool_name.trim().is_empty() {
                push_validation_error(&mut field_errors, "tool.tool_name", "tool_name 不能为空");
            }
            if parse_mapping_yaml(&tool_parameters_editor.text(), "工具参数").is_err() {
                push_validation_error(
                    &mut field_errors,
                    "tool.tool_parameters",
                    "工具参数必须是合法 YAML map",
                );
            }
            if parse_mapping_yaml(&tool_configurations_editor.text(), "工具配置").is_err() {
                push_validation_error(
                    &mut field_errors,
                    "tool.tool_configurations",
                    "工具配置必须是合法 YAML map",
                );
            }
        }
        Some(WorkflowNodeVisualDraft::Agent {
            strategy_provider_name,
            strategy_name,
            strategy_label,
            output_schema_editor,
            parameters_editor,
            memory_enabled,
            memory_window_size_input,
            ..
        }) => {
            if strategy_provider_name.trim().is_empty() {
                push_validation_error(
                    &mut field_errors,
                    "agent.strategy_provider",
                    "策略 provider 不能为空",
                );
            }
            if strategy_name.trim().is_empty() {
                push_validation_error(&mut field_errors, "agent.strategy_name", "策略名称不能为空");
            }
            if strategy_label.trim().is_empty() {
                push_validation_error(
                    &mut field_errors,
                    "agent.strategy_label",
                    "策略显示名称不能为空",
                );
            }
            if parse_mapping_yaml(&output_schema_editor.text(), "Agent 输出结构").is_err() {
                push_validation_error(
                    &mut field_errors,
                    "agent.output_schema",
                    "输出结构必须是合法 YAML map",
                );
            }
            if parse_mapping_yaml(&parameters_editor.text(), "Agent 参数").is_err() {
                push_validation_error(
                    &mut field_errors,
                    "agent.parameters",
                    "Agent 参数必须是合法 YAML map",
                );
            }
            if *memory_enabled
                && memory_window_size_input
                    .trim()
                    .parse::<u64>()
                    .ok()
                    .filter(|value| *value > 0)
                    .is_none()
            {
                push_validation_error(
                    &mut field_errors,
                    "agent.memory.window_size",
                    "memory window size 必须是正整数",
                );
            }
        }
        Some(WorkflowNodeVisualDraft::Llm {
            provider,
            model_name,
            model_mode,
            context_enabled,
            context_selector_input,
            ..
        }) => {
            if provider.trim().is_empty() {
                push_validation_error(&mut field_errors, "llm.provider", "模型 provider 不能为空");
            }
            if model_name.trim().is_empty() {
                push_validation_error(&mut field_errors, "llm.model_name", "模型名称不能为空");
            }
            if model_mode.trim().is_empty() {
                push_validation_error(&mut field_errors, "llm.model_mode", "模型 mode 不能为空");
            }
            if *context_enabled && context_selector_input.trim().is_empty() {
                push_validation_error(
                    &mut field_errors,
                    "llm.context_selector",
                    "启用上下文后必须填写变量选择器",
                );
            }
        }
        Some(WorkflowNodeVisualDraft::Answer { answer_editor }) => {
            if answer_editor.text().trim().is_empty() {
                push_validation_error(&mut field_errors, "answer.text", "回复内容不能为空");
            }
        }
        Some(WorkflowNodeVisualDraft::HttpRequest {
            method,
            url,
            auth_type,
            auth_username,
            auth_password,
            auth_token,
            headers_editor,
            params_editor,
            body_editor,
            timeout_connect,
            timeout_read,
            timeout_write,
            retry_config,
            error_strategy,
            ..
        }) => {
            if !matches!(
                method.to_ascii_lowercase().as_str(),
                "get" | "post" | "put" | "patch" | "delete" | "head" | "options"
            ) {
                push_validation_error(&mut field_errors, "http.method", "HTTP 方法不受支持");
            }
            let lower_url = url.trim().to_ascii_lowercase();
            if !(lower_url.starts_with("http://") || lower_url.starts_with("https://"))
                || url.trim().split_once("://").is_none_or(|(_, rest)| rest.trim().is_empty())
            {
                push_validation_error(
                    &mut field_errors,
                    "http.url",
                    "URL 必须是有效的 HTTP 或 HTTPS 地址",
                );
            }
            let auth_invalid = match auth_type.as_str() {
                "no-auth" => false,
                "basic" => auth_username.trim().is_empty() || auth_password.trim().is_empty(),
                "bearer" | "api-key" => auth_token.trim().is_empty(),
                _ => true,
            };
            if auth_invalid {
                push_validation_error(&mut field_errors, "http.authorization", "鉴权配置不完整");
            }
            for (path, value) in [
                ("http.timeout.connect", timeout_connect),
                ("http.timeout.read", timeout_read),
                ("http.timeout.write", timeout_write),
            ] {
                if value.trim().parse::<u64>().ok().filter(|v| *v > 0 && *v <= 3600).is_none() {
                    push_validation_error(&mut field_errors, path, "超时必须是 1 到 3600 秒的整数");
                }
            }
            for (path, editor) in [
                ("http.headers", headers_editor),
                ("http.params", params_editor),
                ("http.body", body_editor),
            ] {
                if parse_http_value(&editor.text(), path).is_err() {
                    push_validation_error(&mut field_errors, path, "必须是合法 YAML");
                }
            }
            if retry_config.enabled {
                if !(1..=10).contains(&retry_config.max_retries) {
                    push_validation_error(
                        &mut field_errors,
                        "http.retry.max_retries",
                        "最大重试次数必须在 1 到 10 之间",
                    );
                }
                if !(100..=5000).contains(&retry_config.retry_interval) {
                    push_validation_error(
                        &mut field_errors,
                        "http.retry.retry_interval",
                        "重试间隔必须在 100 到 5000 毫秒之间",
                    );
                }
            }
            if !matches!(error_strategy.as_str(), "none" | "fail-branch") {
                push_validation_error(&mut field_errors, "http.error_strategy", "错误策略不受支持");
            }
        }
        Some(WorkflowNodeVisualDraft::Code {
            language,
            inputs,
            code_editor,
            outputs,
            retry_config,
            error_strategy,
            default_value_editor,
        }) => {
            if language.trim().is_empty() {
                push_validation_error(&mut field_errors, "code.language", "代码语言不能为空");
            }
            if code_editor.text().trim().is_empty() {
                push_validation_error(&mut field_errors, "code.body", "代码内容不能为空");
            }

            let mut input_names = std::collections::HashSet::new();
            for (index, input) in inputs.iter().enumerate() {
                if input.variable.trim().is_empty() {
                    push_validation_error(
                        &mut field_errors,
                        &format!("code.inputs[{index}].variable"),
                        "输入变量名不能为空",
                    );
                } else if !input_names.insert(input.variable.trim().to_string()) {
                    push_validation_error(
                        &mut field_errors,
                        &format!("code.inputs[{index}].variable"),
                        "输入变量名不能重复",
                    );
                }

                if input.selector.is_empty()
                    || input.selector.iter().any(|part| part.trim().is_empty())
                {
                    push_validation_error(
                        &mut field_errors,
                        &format!("code.inputs[{index}].selector"),
                        "请选择有效的输入变量引用",
                    );
                }

                if input.value_type.trim().is_empty() {
                    push_validation_error(
                        &mut field_errors,
                        &format!("code.inputs[{index}].value_type"),
                        "输入变量类型不能为空",
                    );
                }
            }

            if outputs.is_empty() {
                push_validation_error(&mut field_errors, "code.outputs", "至少需要一个输出变量");
            }

            let mut output_names = std::collections::HashSet::new();
            for (index, output) in outputs.iter().enumerate() {
                if output.key.trim().is_empty() {
                    push_validation_error(
                        &mut field_errors,
                        &format!("code.outputs[{index}].key"),
                        "输出变量名不能为空",
                    );
                } else if !output_names.insert(output.key.trim().to_string()) {
                    push_validation_error(
                        &mut field_errors,
                        &format!("code.outputs[{index}].key"),
                        "输出变量名不能重复",
                    );
                }

                if !is_supported_code_output_type(&output.value_type) {
                    push_validation_error(
                        &mut field_errors,
                        &format!("code.outputs[{index}].type"),
                        "输出变量类型不受支持",
                    );
                }
            }

            if retry_config.enabled {
                if !(1..=10).contains(&retry_config.max_retries) {
                    push_validation_error(
                        &mut field_errors,
                        "code.retry.max_retries",
                        "最大重试次数必须在 1 到 10 之间",
                    );
                }
                if !(100..=5000).contains(&retry_config.retry_interval) {
                    push_validation_error(
                        &mut field_errors,
                        "code.retry.retry_interval",
                        "重试间隔必须在 100 到 5000 毫秒之间",
                    );
                }
            }

            if !matches!(error_strategy.as_str(), "none" | "default-value" | "fail-branch") {
                push_validation_error(
                    &mut field_errors,
                    "code.error_strategy",
                    "异常处理仅支持 none / default-value / fail-branch",
                );
            }

            if error_strategy == "default-value"
                && parse_code_default_value_yaml(&default_value_editor.text()).is_err()
            {
                push_validation_error(
                    &mut field_errors,
                    "code.default_value",
                    "默认值必须是合法 YAML sequence",
                );
            }
        }
        Some(WorkflowNodeVisualDraft::VariableAssigner { items }) => {
            for (index, item) in items.iter().enumerate() {
                if item.variable_selector.path.is_empty() {
                    push_validation_error(
                        &mut field_errors,
                        &format!("assigner.items[{index}].target"),
                        "目标变量不能为空",
                    );
                }
                if item.operation.trim().is_empty() {
                    push_validation_error(
                        &mut field_errors,
                        &format!("assigner.items[{index}].operation"),
                        "赋值操作不能为空",
                    );
                }
                if item.input_type == "variable"
                    && !matches!(&item.value, Value::Sequence(parts) if !parts.is_empty())
                {
                    push_validation_error(
                        &mut field_errors,
                        &format!("assigner.items[{index}].value"),
                        "变量来源必须是选择器",
                    );
                }
            }
        }
        Some(WorkflowNodeVisualDraft::VariableAggregator { variables, output_type }) => {
            if variables.is_empty() {
                push_validation_error(
                    &mut field_errors,
                    "aggregator.variables",
                    "至少需要一个候选变量",
                );
            }
            if variables
                .iter()
                .any(|item| WorkflowVariableSelectorDraft::from_input(item).path.is_empty())
            {
                push_validation_error(
                    &mut field_errors,
                    "aggregator.variables",
                    "候选变量不能为空",
                );
            }
            if output_type.trim().is_empty() {
                push_validation_error(
                    &mut field_errors,
                    "aggregator.output_type",
                    "输出类型不能为空",
                );
            }
        }
        Some(WorkflowNodeVisualDraft::ListOperator {
            variable,
            conditions,
            filter_enabled,
            limit_enabled,
            limit_size,
            ..
        }) => {
            if WorkflowVariableSelectorDraft::from_input(variable).path.is_empty() {
                push_validation_error(&mut field_errors, "list.variable", "输入数组不能为空");
            }
            if *filter_enabled && conditions.is_empty() {
                push_validation_error(
                    &mut field_errors,
                    "list.conditions",
                    "启用过滤时至少需要一条条件",
                );
            }
            if *limit_enabled && limit_size.trim().parse::<u64>().is_err() {
                push_validation_error(&mut field_errors, "list.limit", "截取数量必须是非负整数");
            }
        }
        Some(WorkflowNodeVisualDraft::Iteration {
            iterator_selector,
            output_selector,
            output_type,
            parallel_nums,
            error_handle_mode,
            ..
        }) => {
            if iterator_selector.trim().is_empty() {
                push_validation_error(
                    &mut field_errors,
                    "iteration.iterator_selector",
                    "迭代输入不能为空",
                );
            }
            if output_selector.trim().is_empty() {
                push_validation_error(
                    &mut field_errors,
                    "iteration.output_selector",
                    "迭代输出不能为空",
                );
            }
            if output_type.trim().is_empty() {
                push_validation_error(
                    &mut field_errors,
                    "iteration.output_type",
                    "输出类型不能为空",
                );
            }
            if !matches!(parallel_nums.trim().parse::<u64>(), Ok(1..)) {
                push_validation_error(
                    &mut field_errors,
                    "iteration.parallel_nums",
                    "并行数量必须是正整数",
                );
            }
            if error_handle_mode.trim().is_empty() {
                push_validation_error(
                    &mut field_errors,
                    "iteration.error_handle_mode",
                    "错误处理方式不能为空",
                );
            }
        }
        Some(WorkflowNodeVisualDraft::Loop {
            variables,
            break_conditions,
            logical_operator,
            max_loop_count,
        }) => {
            if !matches!(max_loop_count.trim().parse::<u64>(), Ok(1..)) {
                push_validation_error(
                    &mut field_errors,
                    "loop.max_loop_count",
                    "最大循环次数必须是正整数",
                );
            }
            if !matches!(logical_operator.as_str(), "and" | "or") {
                push_validation_error(
                    &mut field_errors,
                    "loop.logical_operator",
                    "逻辑运算仅支持 and 或 or",
                );
            }
            for (index, item) in variables.iter().enumerate() {
                if item.id.trim().is_empty() {
                    push_validation_error(
                        &mut field_errors,
                        &format!("loop.variables[{index}].id"),
                        "循环变量 ID 不能为空",
                    );
                }
                if item.label.trim().is_empty() {
                    push_validation_error(
                        &mut field_errors,
                        &format!("loop.variables[{index}].label"),
                        "循环变量名称不能为空",
                    );
                }
            }
            for (index, item) in break_conditions.iter().enumerate() {
                if item.selector.trim().is_empty() {
                    push_validation_error(
                        &mut field_errors,
                        &format!("loop.break_conditions[{index}].selector"),
                        "终止条件变量不能为空",
                    );
                }
                if item.comparison_operator.trim().is_empty() {
                    push_validation_error(
                        &mut field_errors,
                        &format!("loop.break_conditions[{index}].operator"),
                        "终止条件操作符不能为空",
                    );
                }
            }
        }
        None => {}
    }

    WorkflowNodeEditorValidation { field_errors }
}

fn validate_structured_model(
    errors: &mut Vec<WorkflowNodeValidationError>,
    prefix: &str,
    query: &WorkflowVariableSelectorDraft,
    model: &WorkflowModelDraft,
) {
    if query.path.is_empty() {
        push_validation_error(errors, &format!("{prefix}.query"), "输入变量不能为空");
    }
    for (field, value, message) in [
        ("provider", model.provider.as_str(), "模型 provider 不能为空"),
        ("model_name", model.name.as_str(), "模型名称不能为空"),
        ("model_mode", model.mode.as_str(), "模型 mode 不能为空"),
    ] {
        if value.trim().is_empty() {
            push_validation_error(errors, &format!("{prefix}.{field}"), message);
        }
    }
}

fn push_validation_error(
    field_errors: &mut Vec<WorkflowNodeValidationError>,
    path: &str,
    message: impl Into<String>,
) {
    field_errors
        .push(WorkflowNodeValidationError { path: path.to_string(), message: message.into() });
}

fn validate_named_values(
    errors: &mut Vec<WorkflowNodeValidationError>,
    prefix: &str,
    values: &[WorkflowNamedValueDraft],
    require_type: bool,
) {
    let mut names = std::collections::HashSet::new();
    for (index, item) in values.iter().enumerate() {
        if item.name.trim().is_empty() {
            push_validation_error(errors, &format!("{prefix}[{index}].name"), "变量名不能为空");
        } else if !names.insert(item.name.trim()) {
            push_validation_error(errors, &format!("{prefix}[{index}].name"), "变量名不能重复");
        }
        if require_type && item.value_type.trim().is_empty() {
            push_validation_error(errors, &format!("{prefix}[{index}].type"), "变量类型不能为空");
        }
        if item.selector.path.is_empty() {
            push_validation_error(
                errors,
                &format!("{prefix}[{index}].selector"),
                "变量选择器不能为空",
            );
        }
    }
}

fn is_supported_code_output_type(value_type: &str) -> bool {
    matches!(
        value_type.trim(),
        "string"
            | "number"
            | "boolean"
            | "array[number]"
            | "array[string]"
            | "array[boolean]"
            | "array[object]"
            | "object"
    )
}

#[cfg(test)]
#[path = "node_validation_tests.rs"]
mod node_validation_tests;
