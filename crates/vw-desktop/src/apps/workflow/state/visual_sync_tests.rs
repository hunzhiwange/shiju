//! 工作流可视化草稿同步到原始 YAML 的测试，覆盖各节点专用字段。
//! 非法根结构和类型不匹配必须明确失败或保持原文，避免静默改写。
use super::*;

fn editor(text: &str) -> text_editor::Content {
    text_editor::Content::with_text(text)
}

fn yaml_value(text: &str) -> Value {
    serde_yaml::from_str::<Value>(text).expect("test YAML should parse")
}

fn synced_value(
    block_type: &str,
    raw_data_yaml: &str,
    visual_draft: &WorkflowNodeVisualDraft,
) -> Value {
    let yaml = apply_visual_draft_to_yaml(block_type, raw_data_yaml, Some(visual_draft))
        .expect("visual draft should sync");

    yaml_value(&yaml)
}

fn map_value<'a>(value: &'a Value, key: &str) -> &'a Value {
    mapping_value(value.as_mapping().expect("value should be a map"), key)
        .unwrap_or_else(|| panic!("missing key {key}"))
}

fn nested_map_value<'a>(value: &'a Value, keys: &[&str]) -> &'a Value {
    keys.iter().fold(value, |current, key| map_value(current, key))
}

fn start_variable(input_type: &str) -> WorkflowStartVariableDraft {
    let mut variable = default_start_variable_draft();
    variable.label = "输入".to_string();
    variable.variable = "question".to_string();
    variable.input_type = input_type.to_string();
    variable.required = false;
    variable.hidden = true;
    variable.placeholder = "请输入".to_string();
    variable.hint = "提示".to_string();
    variable.max_length_input = String::new();
    variable
}

#[test]
fn apply_visual_draft_without_draft_returns_raw_yaml() {
    let raw_yaml = "answer: old\n";

    let synced = apply_visual_draft_to_yaml("answer", raw_yaml, None).expect("raw YAML returns");

    assert_eq!(synced, raw_yaml);
}

#[test]
fn apply_visual_draft_rejects_non_mapping_root() {
    let draft = WorkflowNodeVisualDraft::Answer { answer_editor: editor("answer") };

    let error = apply_visual_draft_to_yaml("answer", "- item\n", Some(&draft)).unwrap_err();

    assert_eq!(error, "节点 data 必须是对象映射（YAML map）");
}

#[test]
fn apply_visual_draft_reports_raw_yaml_parse_error() {
    let draft = WorkflowNodeVisualDraft::Answer { answer_editor: editor("answer") };

    let error = apply_visual_draft_to_yaml("answer", "answer: [", Some(&draft)).unwrap_err();

    assert!(error.starts_with("节点 data YAML 解析失败:"));
}

#[test]
fn apply_visual_draft_ignores_mismatched_block_type_and_draft() {
    let draft = WorkflowNodeVisualDraft::Answer { answer_editor: editor("answer") };

    let synced = synced_value("llm", "kept: true\n", &draft);

    assert_eq!(map_value(&synced, "kept").as_bool(), Some(true));
    assert!(mapping_value(synced.as_mapping().unwrap(), "answer").is_none());
}

#[test]
fn start_visual_draft_syncs_select_variable_fields() {
    let mut variable = start_variable("select");
    variable.options = vec!["A".to_string(), " ".to_string(), "B".to_string()];
    variable.default_value = "B".to_string();
    variable.max_length_input = "8".to_string();
    variable.raw_variable = yaml_value(
        "legacy: kept\nallowed_file_types:\n  - image\nallowed_file_extensions:\n  - .png\n",
    );
    let draft = WorkflowNodeVisualDraft::Start { variables: vec![variable] };

    let synced = synced_value("start", "variables: []\n", &draft);
    let variables = map_value(&synced, "variables").as_sequence().unwrap();
    let variable = variables[0].as_mapping().unwrap();

    assert_eq!(mapping_value(variable, "label").and_then(Value::as_str), Some("输入"));
    assert_eq!(mapping_value(variable, "variable").and_then(Value::as_str), Some("question"));
    assert_eq!(mapping_value(variable, "type").and_then(Value::as_str), Some("select"));
    assert_eq!(mapping_value(variable, "required").and_then(Value::as_bool), Some(false));
    assert_eq!(mapping_value(variable, "hide").and_then(Value::as_bool), Some(true));
    assert_eq!(mapping_value(variable, "default").and_then(Value::as_str), Some("B"));
    assert_eq!(mapping_value(variable, "max_length").and_then(Value::as_u64), Some(8));
    assert_eq!(mapping_value(variable, "options").and_then(Value::as_sequence).unwrap().len(), 2);
    assert!(mapping_value(variable, "allowed_file_types").is_none());
    assert_eq!(mapping_value(variable, "legacy").and_then(Value::as_str), Some("kept"));
}

#[test]
fn start_visual_draft_syncs_file_list_defaults_and_file_metadata() {
    let mut variable = start_variable("file-list");
    variable.allowed_file_types = vec!["image".to_string()];
    variable.allowed_file_extensions_input = ".png jpg\n.pdf".to_string();
    variable.allowed_file_upload_methods = vec!["local_file".to_string()];
    variable.default_file_values = vec!["https://example.test/a.png".to_string()];
    let draft = WorkflowNodeVisualDraft::Start { variables: vec![variable] };

    let synced = synced_value("start", "", &draft);
    let variables = map_value(&synced, "variables").as_sequence().unwrap();
    let variable = variables[0].as_mapping().unwrap();

    assert_eq!(mapping_value(variable, "type").and_then(Value::as_str), Some("file-list"));
    assert_eq!(
        mapping_value(variable, "default").and_then(Value::as_sequence).unwrap()[0].as_str(),
        Some("https://example.test/a.png")
    );
    assert_eq!(
        mapping_value(variable, "allowed_file_extensions")
            .and_then(Value::as_sequence)
            .unwrap()
            .iter()
            .filter_map(Value::as_str)
            .collect::<Vec<_>>(),
        vec![".jpg", ".pdf", ".png"]
    );
}

#[test]
fn start_visual_draft_reports_invalid_scalar_inputs() {
    let mut variable = start_variable("number");
    variable.default_value = "abc".to_string();
    let draft = WorkflowNodeVisualDraft::Start { variables: vec![variable] };

    let error = apply_visual_draft_to_yaml("start", "", Some(&draft)).unwrap_err();

    assert_eq!(error, "数字类型默认值必须是数字");
}

#[test]
fn start_visual_draft_reports_invalid_checkbox_default() {
    let mut variable = start_variable("checkbox");
    variable.default_value = "yes".to_string();
    let draft = WorkflowNodeVisualDraft::Start { variables: vec![variable] };

    let error = apply_visual_draft_to_yaml("start", "", Some(&draft)).unwrap_err();

    assert_eq!(error, "复选框默认值只能是 true 或 false");
}

#[test]
fn start_visual_draft_reports_invalid_max_length() {
    let mut variable = start_variable("text-input");
    variable.max_length_input = "many".to_string();
    let draft = WorkflowNodeVisualDraft::Start { variables: vec![variable] };

    let error = apply_visual_draft_to_yaml("start", "", Some(&draft)).unwrap_err();

    assert_eq!(error, "开始节点变量 max_length 必须是非负整数");
}

#[test]
fn llm_visual_draft_syncs_model_context_vision_and_prompts() {
    let draft = WorkflowNodeVisualDraft::Llm {
        provider: "openai".to_string(),
        model_name: "gpt-4.1".to_string(),
        model_mode: "chat".to_string(),
        enable_thinking: true,
        context_enabled: true,
        context_selector_input: "sys.query, conversation.answer".to_string(),
        system_prompt_editor: editor("system prompt"),
        user_prompt_editor: editor("user prompt"),
        vision_enabled: false,
    };

    let synced = synced_value(
        "llm",
        "model: legacy\ncontext: disabled\nprompt_template:\n  - role: system\n    text: old\n",
        &draft,
    );

    assert_eq!(nested_map_value(&synced, &["model", "provider"]).as_str(), Some("openai"));
    assert_eq!(nested_map_value(&synced, &["model", "name"]).as_str(), Some("gpt-4.1"));
    assert_eq!(
        nested_map_value(&synced, &["model", "completion_params", "enable_thinking"]).as_bool(),
        Some(true)
    );
    assert_eq!(nested_map_value(&synced, &["context", "enabled"]).as_bool(), Some(true));
    assert_eq!(nested_map_value(&synced, &["vision", "enabled"]).as_bool(), Some(false));
    let prompts = map_value(&synced, "prompt_template").as_sequence().unwrap();
    assert!(prompts.iter().any(|item| {
        let map = item.as_mapping().unwrap();
        mapping_value(map, "role").and_then(Value::as_str) == Some("system")
            && mapping_value(map, "text").and_then(Value::as_str) == Some("system prompt")
    }));
    assert!(prompts.iter().any(|item| {
        let map = item.as_mapping().unwrap();
        mapping_value(map, "role").and_then(Value::as_str) == Some("user")
            && mapping_value(map, "text").and_then(Value::as_str) == Some("user prompt")
    }));
}

#[test]
fn llm_visual_draft_creates_prompt_template_when_missing() {
    let draft = WorkflowNodeVisualDraft::Llm {
        provider: String::new(),
        model_name: String::new(),
        model_mode: "chat".to_string(),
        enable_thinking: false,
        context_enabled: false,
        context_selector_input: " . , sys. ".to_string(),
        system_prompt_editor: editor(""),
        user_prompt_editor: editor("ask"),
        vision_enabled: true,
    };

    let synced = synced_value("llm", "", &draft);

    assert_eq!(nested_map_value(&synced, &["vision", "enabled"]).as_bool(), Some(true));
    assert_eq!(
        nested_map_value(&synced, &["context", "variable_selector"]),
        &yaml_value("- []\n- [sys]\n")
    );
    let prompts = map_value(&synced, "prompt_template").as_sequence().unwrap();
    assert_eq!(prompts.len(), 2);
    assert_eq!(
        prompts
            .iter()
            .filter_map(Value::as_mapping)
            .filter_map(|prompt| mapping_value(prompt, "text"))
            .filter_map(Value::as_str)
            .collect::<Vec<_>>(),
        vec!["", "ask"]
    );
}

#[test]
fn answer_visual_draft_syncs_answer_text() {
    let draft = WorkflowNodeVisualDraft::Answer { answer_editor: editor("新的答案") };

    let synced = synced_value("answer", "answer: old\n", &draft);

    assert_eq!(map_value(&synced, "answer").as_str(), Some("新的答案"));
}

#[test]
fn if_else_visual_draft_syncs_cases_and_default_condition() {
    let draft = WorkflowNodeVisualDraft::IfElse {
        cases: vec![WorkflowIfElseCaseDraft {
            raw_case: yaml_value("legacy: kept\n"),
            case_id: "case-1".to_string(),
            logical_operator: "or".to_string(),
            conditions: Vec::new(),
        }],
    };

    let synced = synced_value("if-else", "cases: []\n", &draft);
    let case = map_value(&synced, "cases").as_sequence().unwrap()[0].as_mapping().unwrap();

    assert_eq!(mapping_value(case, "case_id").and_then(Value::as_str), Some("case-1"));
    assert_eq!(mapping_value(case, "id").and_then(Value::as_str), Some("case-1"));
    assert_eq!(mapping_value(case, "logical_operator").and_then(Value::as_str), Some("or"));
    assert_eq!(mapping_value(case, "legacy").and_then(Value::as_str), Some("kept"));
    assert_eq!(mapping_value(case, "conditions").and_then(Value::as_sequence).unwrap().len(), 1);
}

#[test]
fn if_else_visual_draft_syncs_explicit_condition() {
    let draft = WorkflowNodeVisualDraft::IfElse {
        cases: vec![WorkflowIfElseCaseDraft {
            raw_case: Value::Mapping(Mapping::new()),
            case_id: "case-2".to_string(),
            logical_operator: "and".to_string(),
            conditions: vec![WorkflowIfElseConditionDraft {
                raw_condition: yaml_value("id: condition-1\n"),
                variable_selector_input: "sys.query".to_string(),
                comparison_operator: "contains".to_string(),
                compare_value: "hello".to_string(),
                var_type: "string".to_string(),
            }],
        }],
    };

    let synced = synced_value("if-else", "", &draft);
    let case = map_value(&synced, "cases").as_sequence().unwrap()[0].as_mapping().unwrap();
    let condition = mapping_value(case, "conditions").and_then(Value::as_sequence).unwrap()[0]
        .as_mapping()
        .unwrap();

    assert_eq!(
        mapping_value(condition, "variable_selector")
            .and_then(Value::as_sequence)
            .unwrap()
            .iter()
            .filter_map(Value::as_str)
            .collect::<Vec<_>>(),
        vec!["sys", "query"]
    );
    assert_eq!(
        mapping_value(condition, "comparison_operator").and_then(Value::as_str),
        Some("contains")
    );
}

#[test]
fn knowledge_retrieval_visual_draft_syncs_enabled_score_threshold() {
    let draft = WorkflowNodeVisualDraft::KnowledgeRetrieval {
        query_selector_input: "sys.query".to_string(),
        query_attachment_selector_input: "sys.files".to_string(),
        dataset_ids_input: "dataset-a, dataset-b".to_string(),
        retrieval_mode: "multiple".to_string(),
        top_k_input: " 5 ".to_string(),
        score_threshold_enabled: true,
        score_threshold_input: "0.42".to_string(),
        reranking_enable: true,
        single_model_provider: "openai".to_string(),
        single_model_name: "rerank".to_string(),
        single_model_mode: "rerank".to_string(),
    };

    let synced = synced_value("knowledge-retrieval", "", &draft);

    assert_eq!(
        map_value(&synced, "query_variable_selector")
            .as_sequence()
            .unwrap()
            .iter()
            .filter_map(Value::as_str)
            .collect::<Vec<_>>(),
        vec!["sys", "query"]
    );
    assert_eq!(map_value(&synced, "retrieval_mode").as_str(), Some("multiple"));
    assert_eq!(
        nested_map_value(&synced, &["multiple_retrieval_config", "top_k"]).as_u64(),
        Some(5)
    );
    assert_eq!(
        nested_map_value(&synced, &["multiple_retrieval_config", "score_threshold"]).as_f64(),
        Some(0.42)
    );
    assert_eq!(
        nested_map_value(&synced, &["single_retrieval_config", "model", "provider"]).as_str(),
        Some("openai")
    );
    assert_eq!(
        map_value(&synced, "query_attachment_selector")
            .as_sequence()
            .unwrap()
            .iter()
            .filter_map(Value::as_str)
            .collect::<Vec<_>>(),
        vec!["sys", "files"]
    );
    assert_eq!(
        map_value(&synced, "dataset_ids")
            .as_sequence()
            .unwrap()
            .iter()
            .filter_map(Value::as_str)
            .collect::<Vec<_>>(),
        vec!["dataset-a", "dataset-b"]
    );
    assert!(
        nested_map_value(&synced, &["single_retrieval_config", "model", "completion_params"])
            .as_mapping()
            .unwrap()
            .is_empty()
    );
}

#[test]
fn knowledge_retrieval_visual_draft_syncs_disabled_score_threshold() {
    let draft = WorkflowNodeVisualDraft::KnowledgeRetrieval {
        query_selector_input: String::new(),
        query_attachment_selector_input: String::new(),
        dataset_ids_input: String::new(),
        retrieval_mode: "single".to_string(),
        top_k_input: "1".to_string(),
        score_threshold_enabled: false,
        score_threshold_input: "bad ignored".to_string(),
        reranking_enable: false,
        single_model_provider: String::new(),
        single_model_name: String::new(),
        single_model_mode: String::new(),
    };

    let synced = synced_value("knowledge-retrieval", "", &draft);

    assert!(nested_map_value(&synced, &["multiple_retrieval_config", "score_threshold"]).is_null());
    assert_eq!(
        nested_map_value(&synced, &["multiple_retrieval_config", "reranking_enable"]).as_bool(),
        Some(false)
    );
}

#[test]
fn knowledge_retrieval_visual_draft_preserves_existing_completion_parameters() {
    let draft = WorkflowNodeVisualDraft::KnowledgeRetrieval {
        query_selector_input: "sys.query".to_string(),
        query_attachment_selector_input: String::new(),
        dataset_ids_input: String::new(),
        retrieval_mode: "single".to_string(),
        top_k_input: "3".to_string(),
        score_threshold_enabled: false,
        score_threshold_input: String::new(),
        reranking_enable: false,
        single_model_provider: "openai".to_string(),
        single_model_name: "rerank".to_string(),
        single_model_mode: "rerank".to_string(),
    };

    let synced = synced_value(
        "knowledge-retrieval",
        "single_retrieval_config:\n  model:\n    completion_params:\n      temperature: 0.25\n",
        &draft,
    );

    assert_eq!(
        nested_map_value(
            &synced,
            &["single_retrieval_config", "model", "completion_params", "temperature"],
        )
        .as_f64(),
        Some(0.25)
    );
}

#[test]
fn knowledge_retrieval_visual_draft_replaces_scalar_completion_parameters() {
    let draft = WorkflowNodeVisualDraft::KnowledgeRetrieval {
        query_selector_input: "sys.query".to_string(),
        query_attachment_selector_input: String::new(),
        dataset_ids_input: String::new(),
        retrieval_mode: "single".to_string(),
        top_k_input: "3".to_string(),
        score_threshold_enabled: false,
        score_threshold_input: String::new(),
        reranking_enable: false,
        single_model_provider: "openai".to_string(),
        single_model_name: "rerank".to_string(),
        single_model_mode: "rerank".to_string(),
    };

    let synced = synced_value(
        "knowledge-retrieval",
        "single_retrieval_config:\n  model:\n    completion_params: old\n",
        &draft,
    );

    assert!(
        nested_map_value(&synced, &["single_retrieval_config", "model", "completion_params"])
            .as_mapping()
            .unwrap()
            .is_empty()
    );
}

#[test]
fn knowledge_retrieval_visual_draft_reports_invalid_top_k() {
    let draft = WorkflowNodeVisualDraft::KnowledgeRetrieval {
        query_selector_input: String::new(),
        query_attachment_selector_input: String::new(),
        dataset_ids_input: String::new(),
        retrieval_mode: String::new(),
        top_k_input: "zero".to_string(),
        score_threshold_enabled: false,
        score_threshold_input: String::new(),
        reranking_enable: false,
        single_model_provider: String::new(),
        single_model_name: String::new(),
        single_model_mode: String::new(),
    };

    let error = apply_visual_draft_to_yaml("knowledge-retrieval", "", Some(&draft)).unwrap_err();

    assert_eq!(error, "知识检索 top_k 必须是正整数");
}

#[test]
fn knowledge_retrieval_visual_draft_reports_invalid_score_threshold() {
    let draft = WorkflowNodeVisualDraft::KnowledgeRetrieval {
        query_selector_input: String::new(),
        query_attachment_selector_input: String::new(),
        dataset_ids_input: String::new(),
        retrieval_mode: String::new(),
        top_k_input: "1".to_string(),
        score_threshold_enabled: true,
        score_threshold_input: "high".to_string(),
        reranking_enable: false,
        single_model_provider: String::new(),
        single_model_name: String::new(),
        single_model_mode: String::new(),
    };

    let error = apply_visual_draft_to_yaml("knowledge-retrieval", "", Some(&draft)).unwrap_err();

    assert_eq!(error, "知识检索 score_threshold 必须是数字");
}

#[test]
fn tool_visual_draft_syncs_provider_fields_and_yaml_maps() {
    let draft = WorkflowNodeVisualDraft::Tool {
        provider_id: "provider-id".to_string(),
        provider_type: "builtin".to_string(),
        provider_name: "search".to_string(),
        tool_name: "web_search".to_string(),
        tool_label: "Web Search".to_string(),
        tool_description: "Search web".to_string(),
        credential_id: "cred-1".to_string(),
        plugin_unique_identifier: "plugin@1".to_string(),
        tool_parameters_editor: editor("query:\n  type: string\n"),
        tool_configurations_editor: editor("timeout: 10\n"),
    };

    let synced = synced_value("tool", "", &draft);

    assert_eq!(map_value(&synced, "provider_id").as_str(), Some("provider-id"));
    assert_eq!(map_value(&synced, "credential_id").as_str(), Some("cred-1"));
    assert_eq!(map_value(&synced, "plugin_unique_identifier").as_str(), Some("plugin@1"));
    assert_eq!(map_value(&synced, "tool_node_version").as_str(), Some("2"));
    assert_eq!(
        nested_map_value(&synced, &["tool_parameters", "query", "type"]).as_str(),
        Some("string")
    );
    assert_eq!(nested_map_value(&synced, &["tool_configurations", "timeout"]).as_i64(), Some(10));
}

#[test]
fn tool_visual_draft_removes_blank_optional_ids() {
    let draft = WorkflowNodeVisualDraft::Tool {
        provider_id: String::new(),
        provider_type: String::new(),
        provider_name: String::new(),
        tool_name: String::new(),
        tool_label: String::new(),
        tool_description: String::new(),
        credential_id: " ".to_string(),
        plugin_unique_identifier: String::new(),
        tool_parameters_editor: editor(""),
        tool_configurations_editor: editor(""),
    };

    let synced = synced_value(
        "tool",
        "credential_id: old\nplugin_unique_identifier: old\ntool_parameters: old\n",
        &draft,
    );

    let map = synced.as_mapping().unwrap();
    assert!(mapping_value(map, "credential_id").is_none());
    assert!(mapping_value(map, "plugin_unique_identifier").is_none());
    assert!(map_value(&synced, "tool_parameters").as_mapping().unwrap().is_empty());
}

#[test]
fn tool_visual_draft_reports_invalid_parameter_yaml() {
    let draft = WorkflowNodeVisualDraft::Tool {
        provider_id: String::new(),
        provider_type: String::new(),
        provider_name: String::new(),
        tool_name: String::new(),
        tool_label: String::new(),
        tool_description: String::new(),
        credential_id: String::new(),
        plugin_unique_identifier: String::new(),
        tool_parameters_editor: editor("- item\n"),
        tool_configurations_editor: editor(""),
    };

    let error = apply_visual_draft_to_yaml("tool", "", Some(&draft)).unwrap_err();

    assert_eq!(error, "工具参数 必须是对象映射（YAML map）");
}

#[test]
fn tool_visual_draft_reports_parameter_yaml_parse_error() {
    let draft = WorkflowNodeVisualDraft::Tool {
        provider_id: String::new(),
        provider_type: String::new(),
        provider_name: String::new(),
        tool_name: String::new(),
        tool_label: String::new(),
        tool_description: String::new(),
        credential_id: String::new(),
        plugin_unique_identifier: String::new(),
        tool_parameters_editor: editor("query: ["),
        tool_configurations_editor: editor(""),
    };

    let error = apply_visual_draft_to_yaml("tool", "", Some(&draft)).unwrap_err();

    assert!(error.starts_with("工具参数 YAML 解析失败:"));
}

#[test]
fn tool_visual_draft_reports_invalid_configuration_yaml() {
    let draft = WorkflowNodeVisualDraft::Tool {
        provider_id: String::new(),
        provider_type: String::new(),
        provider_name: String::new(),
        tool_name: String::new(),
        tool_label: String::new(),
        tool_description: String::new(),
        credential_id: String::new(),
        plugin_unique_identifier: String::new(),
        tool_parameters_editor: editor(""),
        tool_configurations_editor: editor("- item\n"),
    };

    let error = apply_visual_draft_to_yaml("tool", "", Some(&draft)).unwrap_err();

    assert_eq!(error, "工具配置 必须是对象映射（YAML map）");
}

#[test]
fn agent_visual_draft_syncs_strategy_maps_and_memory() {
    let draft = WorkflowNodeVisualDraft::Agent {
        strategy_provider_name: "react".to_string(),
        strategy_name: "function_calling".to_string(),
        strategy_label: "Function Calling".to_string(),
        plugin_unique_identifier: "agent@1".to_string(),
        output_schema_editor: editor("answer:\n  type: string\n"),
        parameters_editor: editor("temperature: 0.1\n"),
        memory_enabled: true,
        memory_window_size_input: "12".to_string(),
        memory_prompt_editor: editor("memory prompt"),
    };

    let synced = synced_value("agent", "", &draft);

    assert_eq!(map_value(&synced, "agent_strategy_provider_name").as_str(), Some("react"));
    assert_eq!(map_value(&synced, "plugin_unique_identifier").as_str(), Some("agent@1"));
    assert_eq!(map_value(&synced, "tool_node_version").as_str(), Some("2"));
    assert_eq!(
        nested_map_value(&synced, &["output_schema", "answer", "type"]).as_str(),
        Some("string")
    );
    assert_eq!(nested_map_value(&synced, &["agent_parameters", "temperature"]).as_f64(), Some(0.1));
    assert_eq!(nested_map_value(&synced, &["memory", "window", "enabled"]).as_bool(), Some(true));
    assert_eq!(nested_map_value(&synced, &["memory", "window", "size"]).as_u64(), Some(12));
    assert_eq!(
        nested_map_value(&synced, &["memory", "query_prompt_template"]).as_str(),
        Some("memory prompt")
    );
}

#[test]
fn agent_visual_draft_removes_blank_plugin_identifier() {
    let draft = WorkflowNodeVisualDraft::Agent {
        strategy_provider_name: String::new(),
        strategy_name: String::new(),
        strategy_label: String::new(),
        plugin_unique_identifier: " ".to_string(),
        output_schema_editor: editor(""),
        parameters_editor: editor(""),
        memory_enabled: false,
        memory_window_size_input: "0".to_string(),
        memory_prompt_editor: editor(""),
    };

    let synced = synced_value("agent", "plugin_unique_identifier: old\n", &draft);

    assert!(mapping_value(synced.as_mapping().unwrap(), "plugin_unique_identifier").is_none());
}

#[test]
fn agent_visual_draft_reports_invalid_output_schema_yaml() {
    let draft = WorkflowNodeVisualDraft::Agent {
        strategy_provider_name: String::new(),
        strategy_name: String::new(),
        strategy_label: String::new(),
        plugin_unique_identifier: String::new(),
        output_schema_editor: editor("- item\n"),
        parameters_editor: editor(""),
        memory_enabled: false,
        memory_window_size_input: "1".to_string(),
        memory_prompt_editor: editor(""),
    };

    let error = apply_visual_draft_to_yaml("agent", "", Some(&draft)).unwrap_err();

    assert_eq!(error, "Agent 输出结构 必须是对象映射（YAML map）");
}

#[test]
fn agent_visual_draft_reports_output_schema_yaml_parse_error() {
    let draft = WorkflowNodeVisualDraft::Agent {
        strategy_provider_name: String::new(),
        strategy_name: String::new(),
        strategy_label: String::new(),
        plugin_unique_identifier: String::new(),
        output_schema_editor: editor("answer: ["),
        parameters_editor: editor(""),
        memory_enabled: false,
        memory_window_size_input: "1".to_string(),
        memory_prompt_editor: editor(""),
    };

    let error = apply_visual_draft_to_yaml("agent", "", Some(&draft)).unwrap_err();

    assert!(error.starts_with("Agent 输出结构 YAML 解析失败:"));
}

#[test]
fn agent_visual_draft_reports_invalid_parameters_yaml() {
    let draft = WorkflowNodeVisualDraft::Agent {
        strategy_provider_name: String::new(),
        strategy_name: String::new(),
        strategy_label: String::new(),
        plugin_unique_identifier: String::new(),
        output_schema_editor: editor(""),
        parameters_editor: editor("- item\n"),
        memory_enabled: false,
        memory_window_size_input: "1".to_string(),
        memory_prompt_editor: editor(""),
    };

    let error = apply_visual_draft_to_yaml("agent", "", Some(&draft)).unwrap_err();

    assert_eq!(error, "Agent 参数 必须是对象映射（YAML map）");
}

#[test]
fn agent_visual_draft_reports_invalid_memory_window_size() {
    let draft = WorkflowNodeVisualDraft::Agent {
        strategy_provider_name: String::new(),
        strategy_name: String::new(),
        strategy_label: String::new(),
        plugin_unique_identifier: String::new(),
        output_schema_editor: editor(""),
        parameters_editor: editor(""),
        memory_enabled: false,
        memory_window_size_input: "wide".to_string(),
        memory_prompt_editor: editor(""),
    };

    let error = apply_visual_draft_to_yaml("agent", "", Some(&draft)).unwrap_err();

    assert_eq!(error, "Agent memory window size 必须是正整数");
}

#[test]
fn code_visual_draft_syncs_default_value_error_strategy() {
    let draft = WorkflowNodeVisualDraft::Code {
        language: "python3".to_string(),
        inputs: vec![WorkflowCodeVariableDraft {
            variable: "query".to_string(),
            value_type: "string".to_string(),
            selector: vec!["sys".to_string(), "query".to_string()],
        }],
        code_editor: editor("def main():\n    return {'answer': 'ok'}\n"),
        outputs: vec![WorkflowCodeOutputDraft {
            key: "answer".to_string(),
            value_type: "string".to_string(),
        }],
        retry_config: WorkflowNodeRetryDraft { enabled: true, max_retries: 3, retry_interval: 100 },
        error_strategy: "default-value".to_string(),
        default_value_editor: editor("- key: answer\n  type: string\n  value: fallback\n"),
    };

    let synced = synced_value("code", "", &draft);

    assert_eq!(map_value(&synced, "code_language").as_str(), Some("python3"));
    assert_eq!(nested_map_value(&synced, &["retry_config", "retry_enabled"]).as_bool(), Some(true));
    assert_eq!(nested_map_value(&synced, &["retry_config", "max_retries"]).as_u64(), Some(3));
    assert_eq!(map_value(&synced, "error_strategy").as_str(), Some("default-value"));
    assert_eq!(map_value(&synced, "variables").as_sequence().unwrap().len(), 1);
    assert_eq!(nested_map_value(&synced, &["outputs", "answer", "type"]).as_str(), Some("string"));
    assert_eq!(map_value(&synced, "default_value").as_sequence().unwrap().len(), 1);
}

#[test]
fn code_visual_draft_syncs_fail_branch_and_removes_default_value() {
    let draft = WorkflowNodeVisualDraft::Code {
        language: "javascript".to_string(),
        inputs: Vec::new(),
        code_editor: editor(""),
        outputs: Vec::new(),
        retry_config: WorkflowNodeRetryDraft { enabled: false, max_retries: 0, retry_interval: 0 },
        error_strategy: "fail-branch".to_string(),
        default_value_editor: editor("[]"),
    };

    let synced = synced_value("code", "default_value:\n  - old\n", &draft);

    assert_eq!(map_value(&synced, "error_strategy").as_str(), Some("fail-branch"));
    assert!(mapping_value(synced.as_mapping().unwrap(), "default_value").is_none());
}

#[test]
fn code_visual_draft_removes_unknown_error_strategy() {
    let draft = WorkflowNodeVisualDraft::Code {
        language: String::new(),
        inputs: Vec::new(),
        code_editor: editor(""),
        outputs: Vec::new(),
        retry_config: WorkflowNodeRetryDraft { enabled: false, max_retries: 0, retry_interval: 0 },
        error_strategy: "none".to_string(),
        default_value_editor: editor("[]"),
    };

    let synced = synced_value("code", "error_strategy: old\ndefault_value:\n  - old\n", &draft);

    let map = synced.as_mapping().unwrap();
    assert!(mapping_value(map, "error_strategy").is_none());
    assert!(mapping_value(map, "default_value").is_none());
}

#[test]
fn code_visual_draft_reports_invalid_default_value_yaml() {
    let draft = WorkflowNodeVisualDraft::Code {
        language: String::new(),
        inputs: Vec::new(),
        code_editor: editor(""),
        outputs: Vec::new(),
        retry_config: WorkflowNodeRetryDraft { enabled: false, max_retries: 0, retry_interval: 0 },
        error_strategy: "default-value".to_string(),
        default_value_editor: editor("key: value\n"),
    };

    let error = apply_visual_draft_to_yaml("code", "", Some(&draft)).unwrap_err();

    assert_eq!(error, "代码节点 default_value 必须是数组（YAML sequence）");
}

#[test]
fn code_visual_draft_reports_default_value_yaml_parse_error() {
    let draft = WorkflowNodeVisualDraft::Code {
        language: String::new(),
        inputs: Vec::new(),
        code_editor: editor(""),
        outputs: Vec::new(),
        retry_config: WorkflowNodeRetryDraft { enabled: false, max_retries: 0, retry_interval: 0 },
        error_strategy: "default-value".to_string(),
        default_value_editor: editor("["),
    };

    let error = apply_visual_draft_to_yaml("code", "", Some(&draft)).unwrap_err();

    assert!(error.starts_with("代码节点 default_value YAML 解析失败:"));
}

#[test]
fn parse_node_data_yaml_value_accepts_blank_and_mapping() {
    assert!(parse_node_data_yaml_value("").unwrap().as_mapping().unwrap().is_empty());
    assert_eq!(
        map_value(&parse_node_data_yaml_value("answer: ok\n").unwrap(), "answer").as_str(),
        Some("ok")
    );
}

#[test]
fn parse_node_data_yaml_value_reports_syntax_error() {
    let error = parse_node_data_yaml_value("answer: [").unwrap_err();

    assert!(error.starts_with("节点 data YAML 解析失败:"));
}

#[test]
fn ensure_root_mapping_replaces_non_mapping_values() {
    assert!(
        ensure_root_mapping(Value::String("text".to_string())).as_mapping().unwrap().is_empty()
    );
    assert_eq!(
        map_value(&ensure_root_mapping(yaml_value("key: value\n")), "key").as_str(),
        Some("value")
    );
}

#[test]
fn selector_input_conversions_skip_invalid_and_empty_parts() {
    let value =
        yaml_value("- [sys, query]\n- [conversation, answer]\n- [only, 7]\n- invalid\n- []\n");

    assert_eq!(selector_input_from_value(Some(&value)), "sys.query, conversation.answer, only");
    assert_eq!(selector_input_from_value(None), "");
    assert_eq!(selector_input_from_value(Some(&Value::String("bad".to_string()))), "");
    assert_eq!(
        selector_value_from_input(" sys.query, , answer.text "),
        yaml_value("- [sys, query]\n- [answer, text]\n")
    );
    assert_eq!(selector_value_from_input(" , "), yaml_value("[]"));
}

#[test]
fn selector_path_conversions_skip_invalid_and_empty_parts() {
    let value = yaml_value("[sys, query, 7]");

    assert_eq!(selector_path_input_from_value(Some(&value)), "sys.query");
    assert_eq!(selector_path_input_from_value(Some(&Value::String("bad".to_string()))), "");
    assert_eq!(selector_path_input_from_value(None), "");
    assert_eq!(selector_path_value_from_input(" sys . query . "), yaml_value("[sys, query]"));
    assert_eq!(selector_path_value_from_input(" . "), yaml_value("[]"));
}

#[test]
fn code_variable_value_keeps_selector_and_type() {
    let value = code_variable_value(&WorkflowCodeVariableDraft {
        variable: "query".to_string(),
        value_type: "string".to_string(),
        selector: vec!["sys".to_string(), "query".to_string()],
    });

    assert_eq!(map_value(&value, "variable").as_str(), Some("query"));
    assert_eq!(map_value(&value, "value_type").as_str(), Some("string"));
    assert_eq!(
        map_value(&value, "value_selector")
            .as_sequence()
            .unwrap()
            .iter()
            .filter_map(Value::as_str)
            .collect::<Vec<_>>(),
        vec!["sys", "query"]
    );
}

#[test]
fn code_outputs_value_builds_output_mapping() {
    let value = code_outputs_value(&[WorkflowCodeOutputDraft {
        key: "answer".to_string(),
        value_type: "string".to_string(),
    }]);

    assert_eq!(nested_map_value(&value, &["answer", "type"]).as_str(), Some("string"));
    assert!(nested_map_value(&value, &["answer", "children"]).is_null());
}

#[test]
fn code_outputs_value_allows_empty_outputs() {
    let value = code_outputs_value(&[]);

    assert!(value.as_mapping().unwrap().is_empty());
}
