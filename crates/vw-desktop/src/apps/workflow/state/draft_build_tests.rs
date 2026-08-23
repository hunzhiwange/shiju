//! 工作流节点可视化草稿构建的测试，覆盖各节点类型与 YAML 字段映射。
//! fixture 同时验证默认值、非法结构和编辑后回写，防止丢失受支持字段。
#[test]
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("draft_build_tests"));
}

use super::*;

fn yaml_value_from(text: &str) -> Value {
    serde_yaml::from_str(text).expect("yaml should parse")
}

fn map_value<'a>(value: &'a Value, key: &str) -> &'a Value {
    value.as_mapping().expect("value should be a map").get(key).expect("key should exist")
}

#[test]
fn build_node_visual_draft_handles_start_answer_if_else_unknown_and_invalid_yaml() {
    let start_yaml = r#"
variables:
  - label: Query
    variable: query
    type: paragraph
"#;
    let start = build_node_visual_draft("start", start_yaml).expect("start draft");
    match start {
        Some(WorkflowNodeVisualDraft::Start { variables }) => {
            assert_eq!(variables.len(), 1);
            assert_eq!(variables[0].variable, "query");
            assert_eq!(variables[0].input_type, "paragraph");
        }
        _ => panic!("expected start visual draft"),
    }

    let answer = build_node_visual_draft("answer", "answer: hello\n").expect("answer draft");
    match answer {
        Some(WorkflowNodeVisualDraft::Answer { answer_editor }) => {
            assert_eq!(answer_editor.text(), "hello");
        }
        _ => panic!("expected answer visual draft"),
    }

    let if_else_yaml = r#"
cases:
  - case_id: case-a
    logical_operator: or
    conditions:
      - variable_selector: [start, query]
        comparison_operator: contains
        value: rust
        varType: string
"#;
    let if_else = build_node_visual_draft("if-else", if_else_yaml).expect("if-else draft");
    match if_else {
        Some(WorkflowNodeVisualDraft::IfElse { cases }) => {
            assert_eq!(cases.len(), 1);
            assert_eq!(cases[0].case_id, "case-a");
            assert_eq!(cases[0].conditions[0].variable_selector_input, "start.query");
        }
        _ => panic!("expected if-else visual draft"),
    }

    assert!(build_node_visual_draft("custom", "answer: ok\n").unwrap().is_none());
    assert!(build_node_visual_draft("answer", "- item\n").unwrap_err().contains("对象映射"));
}

#[test]
fn build_node_visual_draft_reads_llm_nested_model_prompt_context_and_vision() {
    let yaml = r#"
model:
  provider: openai
  name: gpt-4.1
  mode: chat
  completion_params:
    enable_thinking: true
context:
  enabled: true
  variable_selector:
    - [sys, query]
    - [conversation, last_answer]
prompt_template:
  - role: system
    text: system prompt
  - role: user
    text: user prompt
vision:
  enabled: true
"#;

    let draft = build_node_visual_draft("llm", yaml).expect("llm draft");

    match draft {
        Some(WorkflowNodeVisualDraft::Llm {
            provider,
            model_name,
            model_mode,
            enable_thinking,
            context_enabled,
            context_selector_input,
            system_prompt_editor,
            user_prompt_editor,
            vision_enabled,
        }) => {
            assert_eq!(provider, "openai");
            assert_eq!(model_name, "gpt-4.1");
            assert_eq!(model_mode, "chat");
            assert!(enable_thinking);
            assert!(context_enabled);
            assert_eq!(context_selector_input, "sys.query, conversation.last_answer");
            assert_eq!(system_prompt_editor.text(), "system prompt");
            assert_eq!(user_prompt_editor.text(), "user prompt");
            assert!(vision_enabled);
        }
        _ => panic!("expected llm visual draft"),
    }
}

#[test]
fn build_node_visual_draft_reads_knowledge_tool_and_agent_fields() {
    let knowledge_yaml = r#"
query_variable_selector: [start, query]
query_attachment_selector: [start, files]
dataset_ids: [dataset-a, dataset-b]
retrieval_mode: single
multiple_retrieval_config:
  top_k: 8
  score_threshold: 0.42
  reranking_enable: true
single_retrieval_config:
  model:
    provider: openai
    name: text-embedding
    mode: embedding
"#;
    let knowledge =
        build_node_visual_draft("knowledge-retrieval", knowledge_yaml).expect("knowledge draft");
    match knowledge {
        Some(WorkflowNodeVisualDraft::KnowledgeRetrieval {
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
        }) => {
            assert_eq!(query_selector_input, "start.query");
            assert_eq!(query_attachment_selector_input, "start.files");
            assert_eq!(dataset_ids_input, "dataset-a, dataset-b");
            assert_eq!(retrieval_mode, "single");
            assert_eq!(top_k_input, "8");
            assert!(score_threshold_enabled);
            assert_eq!(score_threshold_input, "0.42");
            assert!(reranking_enable);
            assert_eq!(single_model_provider, "openai");
            assert_eq!(single_model_name, "text-embedding");
            assert_eq!(single_model_mode, "embedding");
        }
        _ => panic!("expected knowledge draft"),
    }

    let tool_yaml = r#"
provider_id: provider
provider_type: builtin
provider_name: search
tool_name: web_search
tool_label: Web Search
tool_description: Search the web
credential_id: credential
plugin_unique_identifier: plugin
tool_parameters:
  query: rust
tool_configurations:
  timeout: 30
"#;
    let tool = build_node_visual_draft("tool", tool_yaml).expect("tool draft");
    match tool {
        Some(WorkflowNodeVisualDraft::Tool {
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
        }) => {
            assert_eq!(provider_id, "provider");
            assert_eq!(provider_type, "builtin");
            assert_eq!(provider_name, "search");
            assert_eq!(tool_name, "web_search");
            assert_eq!(tool_label, "Web Search");
            assert_eq!(tool_description, "Search the web");
            assert_eq!(credential_id, "credential");
            assert_eq!(plugin_unique_identifier, "plugin");
            assert!(tool_parameters_editor.text().contains("query: rust"));
            assert!(tool_configurations_editor.text().contains("timeout: 30"));
        }
        _ => panic!("expected tool draft"),
    }

    let agent_yaml = r#"
agent_strategy_provider_name: provider
agent_strategy_name: strategy
agent_strategy_label: Strategy
plugin_unique_identifier: plugin
output_schema:
  answer:
    type: string
agent_parameters:
  temperature: 0.2
memory:
  window:
    enabled: true
    size: 6
  query_prompt_template: remember this
"#;
    let agent = build_node_visual_draft("agent", agent_yaml).expect("agent draft");
    match agent {
        Some(WorkflowNodeVisualDraft::Agent {
            strategy_provider_name,
            strategy_name,
            strategy_label,
            plugin_unique_identifier,
            output_schema_editor,
            parameters_editor,
            memory_enabled,
            memory_window_size_input,
            memory_prompt_editor,
        }) => {
            assert_eq!(strategy_provider_name, "provider");
            assert_eq!(strategy_name, "strategy");
            assert_eq!(strategy_label, "Strategy");
            assert_eq!(plugin_unique_identifier, "plugin");
            assert!(output_schema_editor.text().contains("answer:"));
            assert!(parameters_editor.text().contains("temperature: 0.2"));
            assert!(memory_enabled);
            assert_eq!(memory_window_size_input, "6");
            assert_eq!(memory_prompt_editor.text(), "remember this");
        }
        _ => panic!("expected agent draft"),
    }
}

#[test]
fn build_node_visual_draft_reads_code_fields_and_default_value_editor() {
    let yaml = r#"
code_language: python3
variables:
  - variable: user_query
    value_type: string
    value_selector: [start, query]
code: |
  def main(user_query):
      return {"answer": user_query}
outputs:
  answer:
    type: string
  count:
    type: number
retry_config:
  retry_enabled: true
  max_retries: 11
  retry_interval: 20
error_strategy: default-value
"#;

    let draft = build_node_visual_draft("code", yaml).expect("code draft");

    match draft {
        Some(WorkflowNodeVisualDraft::Code {
            language,
            inputs,
            code_editor,
            outputs,
            retry_config,
            error_strategy,
            default_value_editor,
        }) => {
            assert_eq!(language, "python3");
            assert_eq!(inputs.len(), 1);
            assert_eq!(inputs[0].variable, "user_query");
            assert_eq!(inputs[0].selector, vec!["start".to_string(), "query".to_string()]);
            assert!(code_editor.text().contains("def main"));
            let mut output_keys = outputs.iter().map(|item| item.key.as_str()).collect::<Vec<_>>();
            output_keys.sort_unstable();
            assert_eq!(output_keys, vec!["answer", "count"]);
            assert_eq!(
                retry_config,
                WorkflowNodeRetryDraft { enabled: true, max_retries: 10, retry_interval: 100 }
            );
            assert_eq!(error_strategy, "default-value");
            assert!(default_value_editor.text().contains("key: answer"));
            assert!(default_value_editor.text().contains("type: number"));
        }
        _ => panic!("expected code visual draft"),
    }
}

#[test]
fn build_code_visual_draft_preserves_existing_default_value_by_strategy() {
    let default_value_yaml = r#"
outputs:
  answer:
    type: string
error_strategy: default-value
default_value:
  - key: answer
    type: string
    value: custom answer
"#;

    match build_node_visual_draft("code", default_value_yaml).expect("code draft") {
        Some(WorkflowNodeVisualDraft::Code { default_value_editor, error_strategy, .. }) => {
            assert_eq!(error_strategy, "default-value");
            assert!(default_value_editor.text().contains("custom answer"));
        }
        _ => panic!("expected code visual draft"),
    }

    let fail_branch_yaml = r#"
error_strategy: fail-branch
default_value:
  status: ignored by strategy
"#;

    match build_node_visual_draft("code", fail_branch_yaml).expect("code draft") {
        Some(WorkflowNodeVisualDraft::Code { default_value_editor, error_strategy, .. }) => {
            assert_eq!(error_strategy, "fail-branch");
            assert!(default_value_editor.text().contains("status: ignored by strategy"));
        }
        _ => panic!("expected code visual draft"),
    }
}

#[test]
fn code_draft_helpers_handle_defaults_fallbacks_clamps_and_errors() {
    let variables = build_code_variable_drafts(Some(&yaml_value_from(
        r#"
- value_selector: [node, output]
- variable: explicit
  value_type: number
  value_selector: [node, number]
- ignored
"#,
    )));
    assert_eq!(variables.len(), 2);
    assert_eq!(variables[0].variable, "output");
    assert_eq!(variables[0].value_type, "string");
    assert_eq!(variables[1].variable, "explicit");
    assert!(build_code_variable_drafts(Some(&Value::String("bad".to_string()))).is_empty());

    let outputs = build_code_output_drafts(Some(&yaml_value_from(
        r#"
answer:
  type: string
count:
  type: number
"#,
    )));
    let mut output_keys = outputs.iter().map(|item| item.key.as_str()).collect::<Vec<_>>();
    output_keys.sort_unstable();
    assert_eq!(output_keys, vec!["answer", "count"]);
    assert!(build_code_output_drafts(Some(&Value::Sequence(Vec::new()))).is_empty());

    let retry = build_code_retry_draft(Some(&yaml_value_from(
        r#"
retry_enabled: true
max_retries: 0
retry_interval: 9000
"#,
    )));
    assert_eq!(
        retry,
        WorkflowNodeRetryDraft { enabled: true, max_retries: 1, retry_interval: 5000 }
    );
    let retry_defaults =
        build_code_retry_draft(Some(&yaml_value_from("max_retries: -1\nretry_interval: -2\n")));
    assert_eq!(retry_defaults.max_retries, 3);
    assert_eq!(retry_defaults.retry_interval, 1000);

    assert_eq!(
        build_code_error_strategy(Some(&Value::String("fail-branch".to_string()))),
        "fail-branch"
    );
    assert_eq!(build_code_error_strategy(Some(&Value::String("bad".to_string()))), "none");

    let defaults = default_code_default_value_value(&outputs);
    let default_items = defaults.as_sequence().unwrap();
    let answer_default = default_items
        .iter()
        .find(|item| map_value(item, "key").as_str() == Some("answer"))
        .expect("answer default should exist");
    let count_default = default_items
        .iter()
        .find(|item| map_value(item, "key").as_str() == Some("count"))
        .expect("count default should exist");
    assert_eq!(map_value(answer_default, "value").as_str(), Some(""));
    assert_eq!(map_value(count_default, "value"), &serde_yaml::to_value(0_u64).unwrap());
    assert_eq!(parse_code_default_value_yaml("").unwrap(), Value::Sequence(Vec::new()));
    assert!(parse_code_default_value_yaml("answer: ok\n").unwrap_err().contains("数组"));
    assert!(parse_code_default_value_yaml("[").unwrap_err().contains("YAML 解析失败"));
}

#[test]
fn code_default_value_editor_text_uses_existing_sequence_or_generated_defaults_by_strategy() {
    let outputs = vec![WorkflowCodeOutputDraft {
        key: "enabled".to_string(),
        value_type: "boolean".to_string(),
    }];
    let existing = yaml_value_from("- key: custom\n  type: string\n  value: custom\n");

    assert_eq!(
        code_default_value_editor_text(&outputs, Some(&existing), "default-value"),
        value_yaml_for_editor(&existing)
    );
    let generated = code_default_value_editor_text(&outputs, None, "default-value");
    assert!(generated.contains("key: enabled"));
    assert!(generated.contains("value: false"));
    assert_eq!(code_default_value_editor_text(&outputs, None, "none"), "[]\n");
}

#[test]
fn sync_node_editor_raw_from_visual_merges_visual_fields_into_raw_yaml() {
    let mut editor = WorkflowNodeEditorDraft {
        mode: WorkflowNodeEditorMode::Create,
        active_tab: WorkflowNodeEditorTab::Visual,
        block_type: "answer".to_string(),
        title: "Answer".to_string(),
        description: String::new(),
        description_editor: text_editor::Content::with_text(""),
        position: Point::new(0.0, 0.0),
        visual_draft: Some(WorkflowNodeVisualDraft::Answer {
            answer_editor: text_editor::Content::with_text("new answer"),
        }),
        validation: WorkflowNodeEditorValidation::default(),
        show_raw_data_editor: false,
        raw_data_editor: text_editor::Content::with_text("answer: old answer\nkept: true\n"),
        hovered_start_variable_index: None,
        start_variable_focus_index: None,
        start_variable_editor: None,
    };

    sync_node_editor_raw_from_visual(&mut editor).expect("visual draft should sync");

    let synced = serde_yaml::from_str::<Value>(&editor.raw_data_editor.text())
        .expect("synced yaml should parse");
    assert_eq!(map_value(&synced, "answer").as_str(), Some("new answer"));
    assert_eq!(map_value(&synced, "kept").as_bool(), Some(true));
}

#[test]
fn build_node_visual_draft_rejects_malformed_yaml_and_defaults_empty_node_shapes() {
    assert!(build_node_visual_draft("answer", "[").unwrap_err().contains("YAML"));

    match build_node_visual_draft("start", "{}\n").expect("empty start draft") {
        Some(WorkflowNodeVisualDraft::Start { variables }) => assert!(variables.is_empty()),
        _ => panic!("expected start visual draft"),
    }

    match build_node_visual_draft("llm", "{}\n").expect("empty llm draft") {
        Some(WorkflowNodeVisualDraft::Llm {
            provider,
            model_name,
            model_mode,
            enable_thinking,
            context_enabled,
            context_selector_input,
            system_prompt_editor,
            user_prompt_editor,
            vision_enabled,
        }) => {
            assert!(provider.is_empty());
            assert!(model_name.is_empty());
            assert_eq!(model_mode, "chat");
            assert!(!enable_thinking);
            assert!(!context_enabled);
            assert!(context_selector_input.is_empty());
            assert!(system_prompt_editor.text().is_empty());
            assert!(user_prompt_editor.text().is_empty());
            assert!(!vision_enabled);
        }
        _ => panic!("expected llm visual draft"),
    }

    match build_node_visual_draft("answer", "{}\n").expect("empty answer draft") {
        Some(WorkflowNodeVisualDraft::Answer { answer_editor }) => {
            assert!(answer_editor.text().is_empty());
        }
        _ => panic!("expected answer visual draft"),
    }

    match build_node_visual_draft("if-else", "{}\n").expect("empty if-else draft") {
        Some(WorkflowNodeVisualDraft::IfElse { cases }) => assert!(cases.is_empty()),
        _ => panic!("expected if-else visual draft"),
    }
}

#[test]
fn build_node_visual_draft_defaults_knowledge_null_threshold_and_missing_models() {
    match build_node_visual_draft("knowledge-retrieval", "{}\n")
        .expect("knowledge draft without retrieval config")
    {
        Some(WorkflowNodeVisualDraft::KnowledgeRetrieval {
            top_k_input,
            score_threshold_enabled,
            score_threshold_input,
            ..
        }) => {
            assert_eq!(top_k_input, "5");
            assert!(!score_threshold_enabled);
            assert!(score_threshold_input.is_empty());
        }
        _ => panic!("expected knowledge visual draft"),
    }

    let yaml = r#"
multiple_retrieval_config:
  score_threshold: null
"#;

    match build_node_visual_draft("knowledge-retrieval", yaml).expect("knowledge draft") {
        Some(WorkflowNodeVisualDraft::KnowledgeRetrieval {
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
        }) => {
            assert!(query_selector_input.is_empty());
            assert!(query_attachment_selector_input.is_empty());
            assert!(dataset_ids_input.is_empty());
            assert_eq!(retrieval_mode, "multiple");
            assert_eq!(top_k_input, "5");
            assert!(!score_threshold_enabled);
            assert!(score_threshold_input.is_empty());
            assert!(!reranking_enable);
            assert!(single_model_provider.is_empty());
            assert!(single_model_name.is_empty());
            assert_eq!(single_model_mode, "chat");
        }
        _ => panic!("expected knowledge visual draft"),
    }
}

#[test]
fn build_node_visual_draft_defaults_empty_tool_agent_and_code_fields() {
    match build_node_visual_draft("tool", "{}\n").expect("empty tool draft") {
        Some(WorkflowNodeVisualDraft::Tool {
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
        }) => {
            assert!(provider_id.is_empty());
            assert!(provider_type.is_empty());
            assert!(provider_name.is_empty());
            assert!(tool_name.is_empty());
            assert!(tool_label.is_empty());
            assert!(tool_description.is_empty());
            assert!(credential_id.is_empty());
            assert!(plugin_unique_identifier.is_empty());
            assert!(
                yaml_value_from(&tool_parameters_editor.text()).as_mapping().unwrap().is_empty()
            );
            assert!(
                yaml_value_from(&tool_configurations_editor.text())
                    .as_mapping()
                    .unwrap()
                    .is_empty()
            );
        }
        _ => panic!("expected tool visual draft"),
    }

    match build_node_visual_draft("agent", "{}\n").expect("empty agent draft") {
        Some(WorkflowNodeVisualDraft::Agent {
            strategy_provider_name,
            strategy_name,
            strategy_label,
            plugin_unique_identifier,
            output_schema_editor,
            parameters_editor,
            memory_enabled,
            memory_window_size_input,
            memory_prompt_editor,
        }) => {
            assert!(strategy_provider_name.is_empty());
            assert!(strategy_name.is_empty());
            assert!(strategy_label.is_empty());
            assert!(plugin_unique_identifier.is_empty());
            assert!(yaml_value_from(&output_schema_editor.text()).as_mapping().unwrap().is_empty());
            assert!(yaml_value_from(&parameters_editor.text()).as_mapping().unwrap().is_empty());
            assert!(!memory_enabled);
            assert_eq!(memory_window_size_input, "3");
            assert!(memory_prompt_editor.text().is_empty());
        }
        _ => panic!("expected agent visual draft"),
    }

    match build_node_visual_draft("code", "{}\n").expect("empty code draft") {
        Some(WorkflowNodeVisualDraft::Code {
            language,
            inputs,
            code_editor,
            outputs,
            retry_config,
            error_strategy,
            default_value_editor,
        }) => {
            assert_eq!(language, "python3");
            assert!(inputs.is_empty());
            assert!(code_editor.text().is_empty());
            assert!(outputs.is_empty());
            assert_eq!(
                retry_config,
                WorkflowNodeRetryDraft { enabled: false, max_retries: 3, retry_interval: 1000 }
            );
            assert_eq!(error_strategy, "none");
            assert_eq!(default_value_editor.text(), "[]\n");
        }
        _ => panic!("expected code visual draft"),
    }
}

#[test]
fn code_draft_helpers_filter_invalid_parts_and_default_missing_types() {
    let variables = build_code_variable_drafts(Some(&yaml_value_from(
        r#"
- variable: "   "
  value_selector: [node, 7, output]
- variable: ""
  value_selector: [3]
"#,
    )));
    assert_eq!(variables.len(), 2);
    assert_eq!(variables[0].variable, "output");
    assert_eq!(variables[0].selector, vec!["node".to_string(), "output".to_string()]);
    assert_eq!(variables[0].value_type, "string");
    assert!(variables[1].variable.is_empty());
    assert!(variables[1].selector.is_empty());

    let outputs = build_code_output_drafts(Some(&yaml_value_from(
        r#"
"   ":
  type: number
1:
  type: boolean
answer: scalar
count:
  type: 17
enabled:
  type: boolean
"#,
    )));
    assert_eq!(outputs.len(), 3);
    assert_eq!(outputs.iter().find(|output| output.key == "answer").unwrap().value_type, "string");
    assert_eq!(outputs.iter().find(|output| output.key == "count").unwrap().value_type, "string");
    assert_eq!(
        outputs.iter().find(|output| output.key == "enabled").unwrap().value_type,
        "boolean"
    );
}

#[test]
fn code_default_values_cover_supported_shapes_and_parse_valid_sequences() {
    let outputs = [
        ("count", "number"),
        ("enabled", "boolean"),
        ("metadata", "object"),
        ("items", "array[object]"),
        ("fallback", "unsupported"),
    ]
    .into_iter()
    .map(|(key, value_type)| WorkflowCodeOutputDraft {
        key: key.to_string(),
        value_type: value_type.to_string(),
    })
    .collect::<Vec<_>>();

    let defaults = default_code_default_value_value(&outputs);
    let items = defaults.as_sequence().expect("defaults should be a sequence");
    let default_for = |key: &str| {
        items
            .iter()
            .find(|item| map_value(item, "key").as_str() == Some(key))
            .map(|item| map_value(item, "value"))
            .expect("default should exist")
    };
    assert_eq!(default_for("count").as_u64(), Some(0));
    assert_eq!(default_for("enabled").as_bool(), Some(false));
    assert!(default_for("metadata").as_mapping().unwrap().is_empty());
    assert!(default_for("items").as_sequence().unwrap().is_empty());
    assert_eq!(default_for("fallback").as_str(), Some(""));

    let parsed = parse_code_default_value_yaml("- key: answer\n  type: string\n  value: ok\n")
        .expect("default value sequence should parse");
    assert_eq!(parsed.as_sequence().unwrap().len(), 1);

    let empty_existing = Value::Sequence(Vec::new());
    let generated =
        code_default_value_editor_text(&outputs, Some(&empty_existing), "default-value");
    assert!(generated.contains("key: count"));
    let mapping_existing = yaml_value_from("key: stale\n");
    let generated =
        code_default_value_editor_text(&outputs, Some(&mapping_existing), "default-value");
    assert!(generated.contains("key: enabled"));
    assert!(!generated.contains("key: stale"));
    assert_eq!(
        code_default_value_editor_text(&outputs, Some(&mapping_existing), "fail-branch"),
        value_yaml_for_editor(&mapping_existing)
    );
}

#[test]
fn sync_node_editor_raw_from_visual_preserves_raw_yaml_when_parsing_fails() {
    let raw_yaml = "[";
    let mut editor = WorkflowNodeEditorDraft {
        mode: WorkflowNodeEditorMode::Create,
        active_tab: WorkflowNodeEditorTab::Visual,
        block_type: "answer".to_string(),
        title: "Answer".to_string(),
        description: String::new(),
        description_editor: text_editor::Content::with_text(""),
        position: Point::new(0.0, 0.0),
        visual_draft: Some(WorkflowNodeVisualDraft::Answer {
            answer_editor: text_editor::Content::with_text("new answer"),
        }),
        validation: WorkflowNodeEditorValidation::default(),
        show_raw_data_editor: false,
        raw_data_editor: text_editor::Content::with_text(raw_yaml),
        hovered_start_variable_index: None,
        start_variable_focus_index: None,
        start_variable_editor: None,
    };

    let error = sync_node_editor_raw_from_visual(&mut editor).unwrap_err();

    assert!(error.contains("YAML"));
    assert_eq!(editor.raw_data_editor.text(), raw_yaml);
}
