//! 工作流节点校验的测试，覆盖高级 YAML 及各可视化节点的必填约束。
//! 错误路径断言确保开始变量、代码和知识节点能定位到具体字段。
use super::*;
use iced::widget::text_editor;

fn error_paths(validation: &WorkflowNodeEditorValidation) -> Vec<&str> {
    validation.field_errors.iter().map(|error| error.path.as_str()).collect()
}

fn validate(block_type: &str, visual_draft: Option<&WorkflowNodeVisualDraft>) -> Vec<String> {
    validate_node_editor_draft(block_type, "标题", "", "{}", visual_draft)
        .field_errors
        .into_iter()
        .map(|error| error.path)
        .collect()
}

fn valid_start_variable(input_type: &str) -> WorkflowStartVariableDraft {
    let mut variable = default_start_variable_draft();
    variable.label = "上传文件".to_string();
    variable.variable = "upload".to_string();
    variable.input_type = input_type.to_string();
    variable.default_value.clear();
    variable.max_length_input = "5".to_string();
    variable.allowed_file_types = vec!["document".to_string()];
    variable.allowed_file_upload_methods = vec!["local_file".to_string()];
    variable
}

fn valid_if_else_case() -> WorkflowIfElseCaseDraft {
    let mut case = default_if_else_case_draft();
    case.conditions = vec![WorkflowIfElseConditionDraft {
        raw_condition: Value::Mapping(Mapping::new()),
        variable_selector_input: "start.query".to_string(),
        comparison_operator: "contains".to_string(),
        compare_value: "rust".to_string(),
        var_type: "string".to_string(),
    }];
    case
}

fn valid_code_draft() -> WorkflowNodeVisualDraft {
    WorkflowNodeVisualDraft::Code {
        language: "python3".to_string(),
        inputs: vec![WorkflowCodeVariableDraft {
            variable: "query".to_string(),
            value_type: "string".to_string(),
            selector: vec!["start".to_string(), "query".to_string()],
        }],
        code_editor: text_editor::Content::with_text(
            "def main(query):\n    return {\"result\": query}\n",
        ),
        outputs: vec![WorkflowCodeOutputDraft {
            key: "result".to_string(),
            value_type: "string".to_string(),
        }],
        retry_config: WorkflowNodeRetryDraft { enabled: true, max_retries: 3, retry_interval: 500 },
        error_strategy: "default-value".to_string(),
        default_value_editor: text_editor::Content::with_text(
            "- key: result\n  type: string\n  value: fallback\n",
        ),
    }
}

fn knowledge_draft(retrieval_mode: &str) -> WorkflowNodeVisualDraft {
    WorkflowNodeVisualDraft::KnowledgeRetrieval {
        query_selector_input: "start.query".to_string(),
        query_attachment_selector_input: String::new(),
        dataset_ids_input: "dataset".to_string(),
        retrieval_mode: retrieval_mode.to_string(),
        top_k_input: "1".to_string(),
        score_threshold_enabled: false,
        score_threshold_input: String::new(),
        reranking_enable: false,
        reranking_model_editor: text_editor::Content::with_text("{}"),
        metadata_filtering_mode: "disabled".to_string(),
        metadata_model_config_editor: text_editor::Content::with_text("{}"),
        single_model_provider: "openai".to_string(),
        single_model_name: "embedding".to_string(),
        single_model_mode: "embedding".to_string(),
    }
}

#[test]
fn advanced_yaml_rejects_invalid_or_non_mapping_data_without_visual_draft() {
    let invalid_yaml = validate_node_editor_draft("answer", "", "", "[", None);
    let non_mapping_yaml = validate_node_editor_draft("answer", "", "", "- item\n", None);

    assert_eq!(error_paths(&invalid_yaml), vec!["advanced_yaml.raw_data"]);
    assert_eq!(error_paths(&non_mapping_yaml), vec!["advanced_yaml.raw_data"]);
}

#[test]
fn advanced_yaml_accepts_valid_mapping_without_visual_draft() {
    let validation = validate_node_editor_draft("answer", "", "", "answer: ok\n", None);

    assert!(validation.field_errors.is_empty());
}

#[test]
fn visual_draft_reports_raw_data_error_when_existing_yaml_is_not_mapping() {
    let draft =
        WorkflowNodeVisualDraft::Answer { answer_editor: text_editor::Content::with_text("Done") };

    let validation = validate_node_editor_draft("answer", "", "", "- item\n", Some(&draft));

    assert_eq!(error_paths(&validation), vec!["advanced_yaml.raw_data"]);
}

#[test]
fn visual_drafts_accept_complete_valid_fields() {
    let start =
        WorkflowNodeVisualDraft::Start { variables: vec![valid_start_variable("file-list")] };
    let if_else = WorkflowNodeVisualDraft::IfElse { cases: vec![valid_if_else_case()] };
    let knowledge = WorkflowNodeVisualDraft::KnowledgeRetrieval {
        query_selector_input: "start.query".to_string(),
        query_attachment_selector_input: String::new(),
        dataset_ids_input: "dataset-a, dataset-b".to_string(),
        retrieval_mode: "single".to_string(),
        top_k_input: "3".to_string(),
        score_threshold_enabled: true,
        score_threshold_input: "0.8".to_string(),
        reranking_enable: true,
        reranking_model_editor: text_editor::Content::with_text("{}"),
        metadata_filtering_mode: "disabled".to_string(),
        metadata_model_config_editor: text_editor::Content::with_text("{}"),
        single_model_provider: "openai".to_string(),
        single_model_name: "text-embedding-3-large".to_string(),
        single_model_mode: "embedding".to_string(),
    };
    let tool = WorkflowNodeVisualDraft::Tool {
        provider_id: "provider".to_string(),
        provider_type: "builtin".to_string(),
        provider_name: "search".to_string(),
        tool_name: "web_search".to_string(),
        tool_label: "Web Search".to_string(),
        tool_description: String::new(),
        credential_id: String::new(),
        plugin_unique_identifier: String::new(),
        tool_parameters_editor: text_editor::Content::with_text("query: rust\n"),
        tool_configurations_editor: text_editor::Content::with_text("timeout: 30\n"),
    };
    let agent = WorkflowNodeVisualDraft::Agent {
        strategy_provider_name: "function_calling".to_string(),
        strategy_name: "react".to_string(),
        strategy_label: "React".to_string(),
        plugin_unique_identifier: String::new(),
        output_schema_editor: text_editor::Content::with_text("result:\n  type: string\n"),
        parameters_editor: text_editor::Content::with_text("temperature: 0\n"),
        memory_enabled: true,
        memory_window_size_input: "5".to_string(),
        memory_prompt_editor: text_editor::Content::with_text("remember relevant turns"),
    };
    let llm = WorkflowNodeVisualDraft::Llm {
        provider: "openai".to_string(),
        model_name: "gpt-4.1".to_string(),
        model_mode: "chat".to_string(),
        enable_thinking: false,
        completion_params_editor: text_editor::Content::with_text("{}"),
        structured_output_editor: text_editor::Content::with_text("{}"),
        memory_editor: text_editor::Content::with_text("{}"),
        context_enabled: true,
        context_selector_input: "start.query".to_string(),
        system_prompt_editor: text_editor::Content::with_text("You are helpful."),
        user_prompt_editor: text_editor::Content::with_text("{{query}}"),
        vision_enabled: false,
        vision_selector_input: String::new(),
    };
    let answer =
        WorkflowNodeVisualDraft::Answer { answer_editor: text_editor::Content::with_text("Done") };
    let code = valid_code_draft();

    assert!(validate("start", Some(&start)).is_empty());
    assert!(validate("if-else", Some(&if_else)).is_empty());
    assert!(validate("knowledge-retrieval", Some(&knowledge)).is_empty());
    assert!(validate("tool", Some(&tool)).is_empty());
    assert!(validate("agent", Some(&agent)).is_empty());
    assert!(validate("llm", Some(&llm)).is_empty());
    assert!(validate("answer", Some(&answer)).is_empty());
    assert!(validate("code", Some(&code)).is_empty());
}

#[test]
fn start_validation_reports_text_select_number_and_file_errors() {
    let mut text = valid_start_variable("text-input");
    text.label = " ".to_string();
    text.variable.clear();
    text.max_length_input = "wide".to_string();
    let mut select = valid_start_variable("select");
    select.variable = "choice".to_string();
    select.options = vec!["a".to_string(), "a".to_string()];
    select.default_value = "b".to_string();
    let mut number = valid_start_variable("number");
    number.variable = "amount".to_string();
    number.default_value = "12 apples".to_string();
    let mut file = valid_start_variable("file-list");
    file.variable = "files".to_string();
    file.allowed_file_types = vec!["custom".to_string()];
    file.allowed_file_extensions.clear();
    file.allowed_file_upload_methods.clear();
    file.max_length_input = "1".to_string();
    file.default_file_values = vec!["a.txt".to_string(), "b.txt".to_string()];
    let draft = WorkflowNodeVisualDraft::Start { variables: vec![text, select, number, file] };

    let paths = validate("start", Some(&draft));

    assert!(paths.contains(&"start.variables[0].label".to_string()));
    assert!(paths.contains(&"start.variables[0].variable".to_string()));
    assert!(paths.contains(&"start.variables[0].max_length".to_string()));
    assert!(paths.contains(&"start.variables[1].options".to_string()));
    assert!(paths.contains(&"start.variables[1].default".to_string()));
    assert!(paths.contains(&"start.variables[2].default".to_string()));
    assert!(paths.contains(&"start.variables[3].allowed_file_extensions".to_string()));
    assert!(paths.contains(&"start.variables[3].allowed_file_upload_methods".to_string()));
    assert!(paths.contains(&"start.variables[3].default".to_string()));
}

#[test]
fn start_validation_reports_empty_select_options_and_file_list_max_length() {
    let mut select = valid_start_variable("select");
    select.options = vec![" ".to_string()];
    let mut file_list = valid_start_variable("file-list");
    file_list.max_length_input = "11".to_string();
    let draft = WorkflowNodeVisualDraft::Start { variables: vec![select, file_list] };

    let paths = validate("start", Some(&draft));

    assert_eq!(paths, vec!["start.variables[0].options", "start.variables[1].max_length"]);
}

#[test]
fn start_file_validation_requires_at_least_one_file_type() {
    let mut file = valid_start_variable("file");
    file.allowed_file_types.clear();
    let draft = WorkflowNodeVisualDraft::Start { variables: vec![file] };

    let paths = validate("start", Some(&draft));

    assert_eq!(paths, vec!["start.variables[0].allowed_file_types"]);
}

#[test]
fn if_else_validation_reports_missing_case_and_condition_fields() {
    let draft = WorkflowNodeVisualDraft::IfElse {
        cases: vec![
            WorkflowIfElseCaseDraft {
                raw_case: Value::Mapping(Mapping::new()),
                case_id: "case-a".to_string(),
                logical_operator: String::new(),
                conditions: Vec::new(),
            },
            WorkflowIfElseCaseDraft {
                raw_case: Value::Mapping(Mapping::new()),
                case_id: "case-b".to_string(),
                logical_operator: "and".to_string(),
                conditions: vec![WorkflowIfElseConditionDraft {
                    raw_condition: Value::Mapping(Mapping::new()),
                    variable_selector_input: String::new(),
                    comparison_operator: "contains".to_string(),
                    compare_value: String::new(),
                    var_type: String::new(),
                }],
            },
            WorkflowIfElseCaseDraft {
                raw_case: Value::Mapping(Mapping::new()),
                case_id: "case-c".to_string(),
                logical_operator: "or".to_string(),
                conditions: vec![WorkflowIfElseConditionDraft {
                    raw_condition: Value::Mapping(Mapping::new()),
                    variable_selector_input: "start.query".to_string(),
                    comparison_operator: "empty".to_string(),
                    compare_value: String::new(),
                    var_type: "string".to_string(),
                }],
            },
        ],
    };

    let paths = validate("if-else", Some(&draft));

    assert!(paths.contains(&"if_else.cases[0].logical_operator".to_string()));
    assert!(paths.contains(&"if_else.cases[0].conditions".to_string()));
    assert!(paths.contains(&"if_else.cases[1].conditions[0].selector".to_string()));
    assert!(paths.contains(&"if_else.cases[1].conditions[0].var_type".to_string()));
    assert!(paths.contains(&"if_else.cases[1].conditions[0].value".to_string()));
    assert!(!paths.iter().any(|path| path == "if_else.cases[2].conditions[0].value"));
}

#[test]
fn if_else_validation_reports_missing_comparison_operator_separately_from_value() {
    let draft = WorkflowNodeVisualDraft::IfElse {
        cases: vec![WorkflowIfElseCaseDraft {
            raw_case: Value::Mapping(Mapping::new()),
            case_id: "case-a".to_string(),
            logical_operator: "and".to_string(),
            conditions: vec![WorkflowIfElseConditionDraft {
                raw_condition: Value::Mapping(Mapping::new()),
                variable_selector_input: "start.query".to_string(),
                comparison_operator: String::new(),
                compare_value: String::new(),
                var_type: "string".to_string(),
            }],
        }],
    };

    let paths = validate("if-else", Some(&draft));

    assert_eq!(
        paths,
        vec!["if_else.cases[0].conditions[0].operator", "if_else.cases[0].conditions[0].value"]
    );
}

#[test]
fn knowledge_validation_reports_mode_query_dataset_and_model_errors() {
    let multiple = WorkflowNodeVisualDraft::KnowledgeRetrieval {
        query_selector_input: String::new(),
        query_attachment_selector_input: String::new(),
        dataset_ids_input: " , ".to_string(),
        retrieval_mode: "multiple".to_string(),
        top_k_input: "0".to_string(),
        score_threshold_enabled: true,
        score_threshold_input: "2".to_string(),
        reranking_enable: false,
        reranking_model_editor: text_editor::Content::with_text("{}"),
        metadata_filtering_mode: "disabled".to_string(),
        metadata_model_config_editor: text_editor::Content::with_text("{}"),
        single_model_provider: String::new(),
        single_model_name: String::new(),
        single_model_mode: String::new(),
    };
    let single = WorkflowNodeVisualDraft::KnowledgeRetrieval {
        query_selector_input: "start.query".to_string(),
        query_attachment_selector_input: String::new(),
        dataset_ids_input: "dataset".to_string(),
        retrieval_mode: "single".to_string(),
        top_k_input: "1".to_string(),
        score_threshold_enabled: false,
        score_threshold_input: String::new(),
        reranking_enable: false,
        reranking_model_editor: text_editor::Content::with_text("{}"),
        metadata_filtering_mode: "disabled".to_string(),
        metadata_model_config_editor: text_editor::Content::with_text("{}"),
        single_model_provider: String::new(),
        single_model_name: String::new(),
        single_model_mode: String::new(),
    };
    let invalid_mode = knowledge_draft("hybrid");

    let multiple_paths = validate("knowledge-retrieval", Some(&multiple));
    let single_paths = validate("knowledge-retrieval", Some(&single));
    let invalid_mode_paths = validate("knowledge-retrieval", Some(&invalid_mode));

    assert!(multiple_paths.contains(&"knowledge.dataset_ids".to_string()));
    assert!(multiple_paths.contains(&"knowledge.query_selector".to_string()));
    assert!(multiple_paths.contains(&"knowledge.multiple.top_k".to_string()));
    assert!(multiple_paths.contains(&"knowledge.multiple.score_threshold".to_string()));
    assert!(single_paths.contains(&"knowledge.single.provider".to_string()));
    assert!(single_paths.contains(&"knowledge.single.model_name".to_string()));
    assert!(single_paths.contains(&"knowledge.single.model_mode".to_string()));
    assert_eq!(invalid_mode_paths, vec!["knowledge.retrieval_mode"]);
}

#[test]
fn knowledge_multiple_validation_accepts_attachment_query_and_threshold_boundaries() {
    for threshold in ["0", "1"] {
        let draft = WorkflowNodeVisualDraft::KnowledgeRetrieval {
            query_selector_input: String::new(),
            query_attachment_selector_input: "start.files".to_string(),
            dataset_ids_input: " , dataset-a , ".to_string(),
            retrieval_mode: "multiple".to_string(),
            top_k_input: "3".to_string(),
            score_threshold_enabled: true,
            score_threshold_input: threshold.to_string(),
            reranking_enable: false,
            reranking_model_editor: text_editor::Content::with_text("{}"),
            metadata_filtering_mode: "disabled".to_string(),
            metadata_model_config_editor: text_editor::Content::with_text("{}"),
            single_model_provider: String::new(),
            single_model_name: String::new(),
            single_model_mode: String::new(),
        };

        assert!(validate("knowledge-retrieval", Some(&draft)).is_empty());
    }
}

#[test]
fn tool_and_agent_validation_report_missing_ids_maps_and_memory_errors() {
    let tool = WorkflowNodeVisualDraft::Tool {
        provider_id: String::new(),
        provider_type: String::new(),
        provider_name: String::new(),
        tool_name: String::new(),
        tool_label: String::new(),
        tool_description: String::new(),
        credential_id: String::new(),
        plugin_unique_identifier: String::new(),
        tool_parameters_editor: text_editor::Content::with_text("- item\n"),
        tool_configurations_editor: text_editor::Content::with_text("- item\n"),
    };
    let agent = WorkflowNodeVisualDraft::Agent {
        strategy_provider_name: String::new(),
        strategy_name: String::new(),
        strategy_label: String::new(),
        plugin_unique_identifier: String::new(),
        output_schema_editor: text_editor::Content::with_text("- item\n"),
        parameters_editor: text_editor::Content::with_text("- item\n"),
        memory_enabled: true,
        memory_window_size_input: "0".to_string(),
        memory_prompt_editor: text_editor::Content::with_text(""),
    };

    let tool_paths = validate("tool", Some(&tool));
    let agent_paths = validate("agent", Some(&agent));

    assert_eq!(
        tool_paths,
        vec![
            "advanced_yaml.raw_data",
            "tool.provider_id",
            "tool.provider_type",
            "tool.provider_name",
            "tool.tool_name",
            "tool.tool_parameters",
            "tool.tool_configurations",
        ]
    );
    assert_eq!(
        agent_paths,
        vec![
            "advanced_yaml.raw_data",
            "agent.strategy_provider",
            "agent.strategy_name",
            "agent.strategy_label",
            "agent.output_schema",
            "agent.parameters",
            "agent.memory.window_size",
        ]
    );
}

#[test]
fn agent_validation_skips_memory_window_when_memory_is_disabled() {
    let agent = WorkflowNodeVisualDraft::Agent {
        strategy_provider_name: "function_calling".to_string(),
        strategy_name: "react".to_string(),
        strategy_label: "React".to_string(),
        plugin_unique_identifier: String::new(),
        output_schema_editor: text_editor::Content::with_text("{}"),
        parameters_editor: text_editor::Content::with_text("{}"),
        memory_enabled: false,
        memory_window_size_input: "0".to_string(),
        memory_prompt_editor: text_editor::Content::with_text(""),
    };

    let paths = validate("agent", Some(&agent));

    assert!(!paths.contains(&"agent.memory.window_size".to_string()));
}

#[test]
fn llm_answer_and_code_validation_report_required_fields() {
    let llm = WorkflowNodeVisualDraft::Llm {
        provider: String::new(),
        model_name: String::new(),
        model_mode: String::new(),
        enable_thinking: false,
        completion_params_editor: text_editor::Content::with_text("{}"),
        structured_output_editor: text_editor::Content::with_text("{}"),
        memory_editor: text_editor::Content::with_text("{}"),
        context_enabled: true,
        context_selector_input: String::new(),
        system_prompt_editor: text_editor::Content::with_text(""),
        user_prompt_editor: text_editor::Content::with_text(""),
        vision_enabled: false,
        vision_selector_input: String::new(),
    };
    let answer =
        WorkflowNodeVisualDraft::Answer { answer_editor: text_editor::Content::with_text(" ") };
    let code = WorkflowNodeVisualDraft::Code {
        language: String::new(),
        inputs: vec![
            WorkflowCodeVariableDraft {
                variable: "query".to_string(),
                value_type: "string".to_string(),
                selector: vec!["start".to_string()],
            },
            WorkflowCodeVariableDraft {
                variable: "query".to_string(),
                value_type: String::new(),
                selector: vec![" ".to_string()],
            },
        ],
        code_editor: text_editor::Content::with_text(" "),
        outputs: vec![
            WorkflowCodeOutputDraft { key: "result".to_string(), value_type: "xml".to_string() },
            WorkflowCodeOutputDraft { key: "result".to_string(), value_type: "string".to_string() },
        ],
        retry_config: WorkflowNodeRetryDraft { enabled: true, max_retries: 0, retry_interval: 99 },
        error_strategy: "panic".to_string(),
        default_value_editor: text_editor::Content::with_text("not: checked\n"),
    };
    let default_value_code = WorkflowNodeVisualDraft::Code {
        language: "python3".to_string(),
        inputs: vec![WorkflowCodeVariableDraft {
            variable: "query".to_string(),
            value_type: "string".to_string(),
            selector: vec!["start".to_string(), "query".to_string()],
        }],
        code_editor: text_editor::Content::with_text(
            "def main(query):\n    return {\"result\": query}\n",
        ),
        outputs: vec![WorkflowCodeOutputDraft {
            key: "result".to_string(),
            value_type: "string".to_string(),
        }],
        retry_config: WorkflowNodeRetryDraft {
            enabled: false,
            max_retries: 3,
            retry_interval: 500,
        },
        error_strategy: "default-value".to_string(),
        default_value_editor: text_editor::Content::with_text("not: a sequence\n"),
    };

    let llm_paths = validate("llm", Some(&llm));
    let answer_paths = validate("answer", Some(&answer));
    let code_paths = validate("code", Some(&code));
    let default_value_code_paths = validate("code", Some(&default_value_code));

    assert_eq!(
        llm_paths,
        vec!["llm.provider", "llm.model_name", "llm.model_mode", "llm.context_selector"]
    );
    assert_eq!(answer_paths, vec!["answer.text"]);
    assert!(code_paths.contains(&"code.language".to_string()));
    assert!(code_paths.contains(&"code.body".to_string()));
    assert!(code_paths.contains(&"code.inputs[1].variable".to_string()));
    assert!(code_paths.contains(&"code.inputs[1].selector".to_string()));
    assert!(code_paths.contains(&"code.inputs[1].value_type".to_string()));
    assert!(code_paths.contains(&"code.outputs[0].type".to_string()));
    assert!(code_paths.contains(&"code.outputs[1].key".to_string()));
    assert!(code_paths.contains(&"code.retry.max_retries".to_string()));
    assert!(code_paths.contains(&"code.retry.retry_interval".to_string()));
    assert!(code_paths.contains(&"code.error_strategy".to_string()));
    assert!(default_value_code_paths.contains(&"advanced_yaml.raw_data".to_string()));
    assert!(default_value_code_paths.contains(&"code.default_value".to_string()));
}

#[test]
fn code_validation_reports_empty_input_variable_and_empty_outputs() {
    let code = WorkflowNodeVisualDraft::Code {
        language: "python3".to_string(),
        inputs: vec![WorkflowCodeVariableDraft {
            variable: String::new(),
            value_type: "string".to_string(),
            selector: Vec::new(),
        }],
        code_editor: text_editor::Content::with_text("def main():\n    return {}\n"),
        outputs: Vec::new(),
        retry_config: WorkflowNodeRetryDraft {
            enabled: false,
            max_retries: 3,
            retry_interval: 500,
        },
        error_strategy: "none".to_string(),
        default_value_editor: text_editor::Content::with_text("[]"),
    };

    let paths = validate("code", Some(&code));

    assert!(paths.contains(&"code.inputs[0].variable".to_string()));
    assert!(paths.contains(&"code.inputs[0].selector".to_string()));
    assert!(paths.contains(&"code.outputs".to_string()));
}

#[test]
fn code_validation_accepts_each_supported_output_type_after_trimming() {
    let supported_types = [
        "string",
        "number",
        "boolean",
        "array[number]",
        "array[string]",
        "array[boolean]",
        "array[object]",
        "object",
    ];

    let code = WorkflowNodeVisualDraft::Code {
        language: "python3".to_string(),
        inputs: Vec::new(),
        code_editor: text_editor::Content::with_text("def main():\n    return {}\n"),
        outputs: supported_types
            .iter()
            .enumerate()
            .map(|(index, value_type)| WorkflowCodeOutputDraft {
                key: format!("result_{index}"),
                value_type: format!(" {value_type} "),
            })
            .collect(),
        retry_config: WorkflowNodeRetryDraft { enabled: false, max_retries: 0, retry_interval: 0 },
        error_strategy: "fail-branch".to_string(),
        default_value_editor: text_editor::Content::with_text("not used"),
    };

    let paths = validate("code", Some(&code));

    assert!(paths.iter().all(|path| !path.starts_with("code.outputs")));
    assert!(!paths.contains(&"code.retry.max_retries".to_string()));
    assert!(!paths.contains(&"code.retry.retry_interval".to_string()));
    assert!(!paths.contains(&"code.default_value".to_string()));
}

#[test]
fn code_validation_rejects_case_changed_output_types() {
    assert!(!is_supported_code_output_type(" String "));
    assert!(!is_supported_code_output_type("ARRAY[string]"));
    assert!(!is_supported_code_output_type("array[integer]"));
}
