use super::*;
use serde_yaml::{Mapping, Value};

fn open_editor(state: &mut WorkflowState, block_type: &str) {
    state
        .open_create_node_editor(block_type, Point::new(12.0, 34.0))
        .expect("node editor should open");
}

fn editor(state: &WorkflowState) -> &WorkflowNodeEditorDraft {
    state.node_editor.as_ref().expect("node editor should be open")
}

fn visual_draft(state: &WorkflowState) -> &WorkflowNodeVisualDraft {
    editor(state).visual_draft.as_ref().expect("visual draft should exist")
}

fn yaml_key_value<'a>(mapping: &'a Mapping, key: &str) -> &'a Value {
    mapping.get(&yaml_key(key)).expect("yaml key should exist")
}

fn submitted_data_map(state: &WorkflowState) -> Mapping {
    let yaml = node_data_yaml(&state.document.nodes[0]).expect("node data yaml should render");
    serde_yaml::from_str::<Value>(&yaml)
        .expect("node data yaml should parse")
        .as_mapping()
        .expect("node data should be a map")
        .clone()
}

fn scroll_action() -> text_editor::Action {
    text_editor::Action::Scroll { lines: 1 }
}

#[test]
fn tool_editor_updates_fields_and_submit_serializes_yaml() {
    let mut state = WorkflowState::default();
    open_editor(&mut state, "tool");

    state.set_node_editor_tool_provider_id("provider-id".to_string());
    state.set_node_editor_tool_provider_type("builtin".to_string());
    state.set_node_editor_tool_provider_name("Provider".to_string());
    state.set_node_editor_tool_name("search".to_string());
    state.set_node_editor_tool_label("Search".to_string());
    state.set_node_editor_tool_description("Find records".to_string());
    state.set_node_editor_tool_credential_id("credential-1".to_string());
    state.set_node_editor_tool_plugin_unique_identifier("plugin:search".to_string());
    state.node_editor_tool_parameters_action(scroll_action());
    state.node_editor_tool_configurations_action(scroll_action());

    let WorkflowNodeVisualDraft::Tool {
        provider_id,
        provider_type,
        provider_name,
        tool_name,
        tool_label,
        tool_description,
        credential_id,
        plugin_unique_identifier,
        ..
    } = visual_draft(&state)
    else {
        panic!("expected tool visual draft");
    };
    assert_eq!(provider_id, "provider-id");
    assert_eq!(provider_type, "builtin");
    assert_eq!(provider_name, "Provider");
    assert_eq!(tool_name, "search");
    assert_eq!(tool_label, "Search");
    assert_eq!(tool_description, "Find records");
    assert_eq!(credential_id, "credential-1");
    assert_eq!(plugin_unique_identifier, "plugin:search");

    state.submit_node_editor().expect("tool editor should submit");
    let data = submitted_data_map(&state);
    assert_eq!(yaml_key_value(&data, "provider_id").as_str(), Some("provider-id"));
    assert_eq!(yaml_key_value(&data, "provider_type").as_str(), Some("builtin"));
    assert_eq!(yaml_key_value(&data, "provider_name").as_str(), Some("Provider"));
    assert_eq!(yaml_key_value(&data, "tool_name").as_str(), Some("search"));
    assert_eq!(yaml_key_value(&data, "tool_label").as_str(), Some("Search"));
    assert_eq!(yaml_key_value(&data, "tool_description").as_str(), Some("Find records"));
    assert_eq!(yaml_key_value(&data, "credential_id").as_str(), Some("credential-1"));
    assert_eq!(yaml_key_value(&data, "plugin_unique_identifier").as_str(), Some("plugin:search"));
}

#[test]
fn agent_editor_updates_fields_and_submit_serializes_yaml() {
    let mut state = WorkflowState::default();
    open_editor(&mut state, "agent");

    state.set_node_editor_agent_strategy_provider("langgenius".to_string());
    state.set_node_editor_agent_strategy_name("function_calling".to_string());
    state.set_node_editor_agent_strategy_label("Function Calling".to_string());
    state.set_node_editor_agent_plugin_unique_identifier("agent:plugin".to_string());
    state.set_node_editor_agent_memory_enabled(true);
    state.set_node_editor_agent_memory_window_size("8".to_string());
    state.node_editor_agent_output_schema_action(scroll_action());
    state.node_editor_agent_parameters_action(scroll_action());
    state.node_editor_agent_memory_prompt_action(scroll_action());

    let WorkflowNodeVisualDraft::Agent {
        strategy_provider_name,
        strategy_name,
        strategy_label,
        plugin_unique_identifier,
        memory_enabled,
        memory_window_size_input,
        ..
    } = visual_draft(&state)
    else {
        panic!("expected agent visual draft");
    };
    assert_eq!(strategy_provider_name, "langgenius");
    assert_eq!(strategy_name, "function_calling");
    assert_eq!(strategy_label, "Function Calling");
    assert_eq!(plugin_unique_identifier, "agent:plugin");
    assert!(*memory_enabled);
    assert_eq!(memory_window_size_input, "8");

    state.submit_node_editor().expect("agent editor should submit");
    let data = submitted_data_map(&state);
    assert_eq!(yaml_key_value(&data, "agent_strategy_provider_name").as_str(), Some("langgenius"));
    assert_eq!(yaml_key_value(&data, "agent_strategy_name").as_str(), Some("function_calling"));
    let memory = yaml_key_value(&data, "memory").as_mapping().expect("memory should be a map");
    let window = yaml_key_value(memory, "window").as_mapping().expect("window should be a map");
    assert_eq!(yaml_key_value(window, "enabled").as_bool(), Some(true));
    assert_eq!(yaml_key_value(window, "size").as_u64(), Some(8));
}

#[test]
fn llm_and_answer_editors_update_visual_drafts() {
    let mut state = WorkflowState::default();
    open_editor(&mut state, "llm");

    state.set_node_editor_llm_provider("openai".to_string());
    state.set_node_editor_llm_model_name("gpt-4.1".to_string());
    state.set_node_editor_llm_model_mode("chat".to_string());
    state.set_node_editor_llm_enable_thinking(true);
    state.set_node_editor_llm_context_enabled(true);
    state.set_node_editor_llm_context_selector_input("sys.query, start.topic".to_string());
    state.set_node_editor_llm_vision_enabled(true);
    state.node_editor_llm_system_prompt_action(scroll_action());
    state.node_editor_llm_user_prompt_action(scroll_action());

    let WorkflowNodeVisualDraft::Llm {
        provider,
        model_name,
        model_mode,
        enable_thinking,
        context_enabled,
        context_selector_input,
        vision_enabled,
        ..
    } = visual_draft(&state)
    else {
        panic!("expected llm visual draft");
    };
    assert_eq!(provider, "openai");
    assert_eq!(model_name, "gpt-4.1");
    assert_eq!(model_mode, "chat");
    assert!(*enable_thinking);
    assert!(*context_enabled);
    assert_eq!(context_selector_input, "sys.query, start.topic");
    assert!(*vision_enabled);

    open_editor(&mut state, "answer");
    state.node_editor_answer_action(scroll_action());
    assert!(matches!(visual_draft(&state), WorkflowNodeVisualDraft::Answer { .. }));
}

#[test]
fn code_editor_updates_collections_clamps_values_and_syncs_defaults() {
    let mut state = WorkflowState::default();
    open_editor(&mut state, "code");

    state.set_node_editor_code_language("javascript".to_string());
    state.add_node_editor_code_input_variable();
    state.set_node_editor_code_input_variable_selector(
        0,
        "start.query".to_string(),
        "number".to_string(),
    );
    state.set_node_editor_code_input_variable_name(0, "question".to_string());
    state.remove_node_editor_code_input_variable(99);
    state.add_node_editor_code_input_variable();
    state.remove_node_editor_code_input_variable(1);

    state.add_node_editor_code_output_variable();
    state.add_node_editor_code_output_variable();
    state.set_node_editor_code_error_strategy("default-value".to_string());
    state.set_node_editor_code_output_name(0, "total".to_string());
    state.set_node_editor_code_output_type(0, "number".to_string());
    state.remove_node_editor_code_output_variable(99);
    state.remove_node_editor_code_output_variable(1);

    state.set_node_editor_code_retry_enabled(true);
    state.set_node_editor_code_retry_max_retries(0);
    state.set_node_editor_code_retry_interval(10);
    state.node_editor_code_action(scroll_action());
    state.node_editor_code_default_value_action(scroll_action());

    let WorkflowNodeVisualDraft::Code {
        language,
        inputs,
        code_editor,
        outputs,
        retry_config,
        error_strategy,
        default_value_editor,
    } = visual_draft(&state)
    else {
        panic!("expected code visual draft");
    };
    assert_eq!(language, "javascript");
    assert!(code_editor.text().contains("function main()"));
    assert_eq!(inputs.len(), 1);
    assert_eq!(inputs[0].variable, "question");
    assert_eq!(inputs[0].value_type, "number");
    assert_eq!(inputs[0].selector, vec!["start".to_string(), "query".to_string()]);
    assert_eq!(outputs.len(), 1);
    assert_eq!(outputs[0].key, "total");
    assert_eq!(outputs[0].value_type, "number");
    assert!(retry_config.enabled);
    assert_eq!(retry_config.max_retries, 1);
    assert_eq!(retry_config.retry_interval, 100);
    assert_eq!(error_strategy, "default-value");
    assert!(default_value_editor.text().contains("total"));
    assert!(default_value_editor.text().contains("number"));

    state.submit_node_editor().expect("code editor should submit");
    let data = submitted_data_map(&state);
    assert_eq!(yaml_key_value(&data, "code_language").as_str(), Some("javascript"));
    assert_eq!(yaml_key_value(&data, "error_strategy").as_str(), Some("default-value"));
    let retry_config =
        yaml_key_value(&data, "retry_config").as_mapping().expect("retry config should be a map");
    assert_eq!(yaml_key_value(retry_config, "retry_enabled").as_bool(), Some(true));
    assert_eq!(yaml_key_value(retry_config, "max_retries").as_u64(), Some(1));
    assert_eq!(yaml_key_value(retry_config, "retry_interval").as_u64(), Some(100));
}

#[test]
fn code_language_and_error_strategy_normalize_unsupported_values() {
    let mut state = WorkflowState::default();
    open_editor(&mut state, "code");

    state.set_node_editor_code_language("ruby".to_string());
    state.set_node_editor_code_error_strategy("retry-later".to_string());

    let WorkflowNodeVisualDraft::Code { language, error_strategy, code_editor, .. } =
        visual_draft(&state)
    else {
        panic!("expected code visual draft");
    };
    assert_eq!(language, "python3");
    assert_eq!(error_strategy, "none");
    assert!(code_editor.text().contains("def main()"));
}

#[test]
fn code_language_preserves_custom_code_and_clamps_retry_upper_bounds() {
    let mut state = WorkflowState::default();
    open_editor(&mut state, "code");

    if let WorkflowNodeVisualDraft::Code { code_editor, .. } = state
        .node_editor
        .as_mut()
        .expect("node editor")
        .visual_draft
        .as_mut()
        .expect("visual draft")
    {
        *code_editor = text_editor::Content::with_text("print('custom')\n");
    } else {
        panic!("expected code visual draft");
    }

    state.set_node_editor_code_language("javascript".to_string());
    state.set_node_editor_code_retry_max_retries(99);
    state.set_node_editor_code_retry_interval(9000);
    state.add_node_editor_code_output_variable();
    state.set_node_editor_code_error_strategy("default-value".to_string());
    if let WorkflowNodeVisualDraft::Code { default_value_editor, .. } = state
        .node_editor
        .as_mut()
        .expect("node editor")
        .visual_draft
        .as_mut()
        .expect("visual draft")
    {
        *default_value_editor = text_editor::Content::with_text("manual: true\n");
    }
    state.set_node_editor_code_output_name(0, "renamed".to_string());

    let WorkflowNodeVisualDraft::Code {
        language,
        code_editor,
        retry_config,
        default_value_editor,
        ..
    } = visual_draft(&state)
    else {
        panic!("expected code visual draft");
    };
    assert_eq!(language, "javascript");
    assert_eq!(code_editor.text(), "print('custom')\n");
    assert_eq!(retry_config.max_retries, 10);
    assert_eq!(retry_config.retry_interval, 5000);
    assert_eq!(default_value_editor.text(), "manual: true\n");
}

#[test]
fn code_input_selector_uses_last_selector_part_as_default_variable_name() {
    let mut state = WorkflowState::default();
    open_editor(&mut state, "code");

    state.add_node_editor_code_input_variable();
    state.set_node_editor_code_input_variable_selector(
        0,
        " start . payload . text ".to_string(),
        "string".to_string(),
    );

    let WorkflowNodeVisualDraft::Code { inputs, .. } = visual_draft(&state) else {
        panic!("expected code visual draft");
    };
    assert_eq!(
        inputs[0].selector,
        vec!["start".to_string(), "payload".to_string(), "text".to_string()]
    );
    assert_eq!(inputs[0].variable, "text");
}

#[test]
fn integration_setters_ignore_non_matching_visual_drafts() {
    let mut state = WorkflowState::default();
    open_editor(&mut state, "answer");
    let before = editor(&state).raw_data_editor.text();

    state.set_node_editor_tool_provider_id("ignored".to_string());
    state.set_node_editor_tool_provider_type("ignored".to_string());
    state.set_node_editor_tool_provider_name("ignored".to_string());
    state.set_node_editor_tool_name("ignored".to_string());
    state.set_node_editor_tool_label("ignored".to_string());
    state.set_node_editor_tool_description("ignored".to_string());
    state.set_node_editor_tool_credential_id("ignored".to_string());
    state.set_node_editor_tool_plugin_unique_identifier("ignored".to_string());
    state.node_editor_tool_parameters_action(scroll_action());
    state.node_editor_tool_configurations_action(scroll_action());
    state.set_node_editor_agent_strategy_provider("ignored".to_string());
    state.set_node_editor_agent_strategy_name("ignored".to_string());
    state.set_node_editor_agent_strategy_label("ignored".to_string());
    state.set_node_editor_agent_plugin_unique_identifier("ignored".to_string());
    state.node_editor_agent_output_schema_action(scroll_action());
    state.node_editor_agent_parameters_action(scroll_action());
    state.set_node_editor_agent_memory_enabled(true);
    state.set_node_editor_agent_memory_window_size("9".to_string());
    state.node_editor_agent_memory_prompt_action(scroll_action());
    state.set_node_editor_llm_provider("ignored".to_string());
    state.set_node_editor_llm_model_name("ignored".to_string());
    state.set_node_editor_llm_model_mode("ignored".to_string());
    state.set_node_editor_llm_enable_thinking(true);
    state.set_node_editor_llm_context_enabled(true);
    state.set_node_editor_llm_context_selector_input("ignored".to_string());
    state.node_editor_llm_system_prompt_action(scroll_action());
    state.node_editor_llm_user_prompt_action(scroll_action());
    state.set_node_editor_llm_vision_enabled(true);
    state.set_node_editor_code_language("javascript".to_string());
    state.add_node_editor_code_input_variable();
    state.remove_node_editor_code_input_variable(0);
    state.set_node_editor_code_input_variable_name(0, "ignored".to_string());
    state.set_node_editor_code_input_variable_selector(
        0,
        "ignored".to_string(),
        "string".to_string(),
    );
    state.add_node_editor_code_output_variable();
    state.remove_node_editor_code_output_variable(0);
    state.set_node_editor_code_output_name(0, "ignored".to_string());
    state.set_node_editor_code_output_type(0, "string".to_string());
    state.set_node_editor_code_retry_enabled(true);
    state.set_node_editor_code_retry_max_retries(10);
    state.set_node_editor_code_retry_interval(5000);
    state.set_node_editor_code_error_strategy("default-value".to_string());
    state.node_editor_code_action(scroll_action());
    state.node_editor_code_default_value_action(scroll_action());

    assert_eq!(editor(&state).raw_data_editor.text(), before);
    assert!(matches!(visual_draft(&state), WorkflowNodeVisualDraft::Answer { .. }));
}

#[test]
fn integration_setters_are_safe_noops_without_open_editor() {
    let mut state = WorkflowState::default();

    state.set_node_editor_tool_provider_id("ignored".to_string());
    state.set_node_editor_tool_provider_type("ignored".to_string());
    state.set_node_editor_tool_provider_name("ignored".to_string());
    state.set_node_editor_tool_name("ignored".to_string());
    state.set_node_editor_tool_label("ignored".to_string());
    state.set_node_editor_tool_description("ignored".to_string());
    state.set_node_editor_tool_credential_id("ignored".to_string());
    state.set_node_editor_tool_plugin_unique_identifier("ignored".to_string());
    state.node_editor_tool_parameters_action(scroll_action());
    state.node_editor_tool_configurations_action(scroll_action());
    state.set_node_editor_agent_strategy_provider("ignored".to_string());
    state.set_node_editor_agent_strategy_name("ignored".to_string());
    state.set_node_editor_agent_strategy_label("ignored".to_string());
    state.set_node_editor_agent_plugin_unique_identifier("ignored".to_string());
    state.node_editor_agent_output_schema_action(scroll_action());
    state.node_editor_agent_parameters_action(scroll_action());
    state.set_node_editor_agent_memory_enabled(true);
    state.set_node_editor_agent_memory_window_size("9".to_string());
    state.node_editor_agent_memory_prompt_action(scroll_action());
    state.set_node_editor_llm_provider("ignored".to_string());
    state.set_node_editor_llm_model_name("ignored".to_string());
    state.set_node_editor_llm_model_mode("ignored".to_string());
    state.set_node_editor_llm_enable_thinking(true);
    state.set_node_editor_llm_context_enabled(true);
    state.set_node_editor_llm_context_selector_input("ignored".to_string());
    state.node_editor_llm_system_prompt_action(scroll_action());
    state.node_editor_llm_user_prompt_action(scroll_action());
    state.set_node_editor_llm_vision_enabled(true);
    state.node_editor_answer_action(scroll_action());
    state.set_node_editor_code_language("javascript".to_string());
    state.add_node_editor_code_input_variable();
    state.remove_node_editor_code_input_variable(0);
    state.set_node_editor_code_input_variable_name(0, "ignored".to_string());
    state.set_node_editor_code_input_variable_selector(
        0,
        "ignored".to_string(),
        "string".to_string(),
    );
    state.add_node_editor_code_output_variable();
    state.remove_node_editor_code_output_variable(0);
    state.set_node_editor_code_output_name(0, "ignored".to_string());
    state.set_node_editor_code_output_type(0, "string".to_string());
    state.set_node_editor_code_retry_enabled(true);
    state.set_node_editor_code_retry_max_retries(10);
    state.set_node_editor_code_retry_interval(5000);
    state.set_node_editor_code_error_strategy("default-value".to_string());
    state.node_editor_code_action(scroll_action());
    state.node_editor_code_default_value_action(scroll_action());

    assert!(state.node_editor.is_none());
    assert!(state.document.nodes.is_empty());
}

#[test]
fn answer_action_ignores_non_answer_visual_draft() {
    let mut state = WorkflowState::default();
    open_editor(&mut state, "tool");
    let before = editor(&state).raw_data_editor.text();

    state.node_editor_answer_action(scroll_action());

    assert_eq!(editor(&state).raw_data_editor.text(), before);
    assert!(matches!(visual_draft(&state), WorkflowNodeVisualDraft::Tool { .. }));
}

#[test]
fn submit_node_editor_handles_empty_editor_and_missing_edit_target() {
    let mut state = WorkflowState::default();
    state.submit_node_editor().expect("empty editor submit should be a no-op");

    open_editor(&mut state, "custom");
    state.node_editor.as_mut().expect("node editor should exist").mode =
        WorkflowNodeEditorMode::Edit("missing-node".to_string());

    let error = state.submit_node_editor().expect_err("missing edit target should fail");
    assert_eq!(error, "目标节点不存在");
}

#[test]
fn submit_node_editor_rejects_invalid_visual_draft_and_keeps_editor_open() {
    let mut state = WorkflowState::default();
    open_editor(&mut state, "code");
    let WorkflowNodeVisualDraft::Code { language, code_editor, outputs, .. } = state
        .node_editor
        .as_mut()
        .expect("node editor")
        .visual_draft
        .as_mut()
        .expect("visual draft")
    else {
        panic!("expected code visual draft");
    };
    language.clear();
    *code_editor = text_editor::Content::with_text("");
    outputs.clear();

    let error = state.submit_node_editor().expect_err("invalid code fields should block submit");

    assert_eq!(error, "请先修正节点表单中的错误字段，再保存。");
    assert!(state.node_editor.is_some());
}

#[test]
fn code_language_accepts_trimmed_case_and_replaces_blank_code() {
    let mut state = WorkflowState::default();
    open_editor(&mut state, "code");

    if let WorkflowNodeVisualDraft::Code { code_editor, .. } = state
        .node_editor
        .as_mut()
        .expect("node editor")
        .visual_draft
        .as_mut()
        .expect("visual draft")
    {
        *code_editor = text_editor::Content::with_text("  \n");
    } else {
        panic!("expected code visual draft");
    }

    state.set_node_editor_code_language(" JavaScript ".to_string());

    let WorkflowNodeVisualDraft::Code { language, code_editor, .. } = visual_draft(&state) else {
        panic!("expected code visual draft");
    };
    assert_eq!(language, "javascript");
    assert_eq!(code_editor.text(), "function main() {\n  return {}\n}\n");
}

#[test]
fn code_input_empty_selector_and_missing_indices_are_handled_safely() {
    let mut state = WorkflowState::default();
    open_editor(&mut state, "code");
    state.add_node_editor_code_input_variable();

    state.set_node_editor_code_input_variable_selector(
        0,
        " .  . ".to_string(),
        "boolean".to_string(),
    );
    state.set_node_editor_code_input_variable_name(9, "ignored".to_string());
    state.set_node_editor_code_input_variable_selector(
        9,
        "start.value".to_string(),
        "number".to_string(),
    );
    state.remove_node_editor_code_input_variable(9);

    let WorkflowNodeVisualDraft::Code { inputs, .. } = visual_draft(&state) else {
        panic!("expected code visual draft");
    };
    assert_eq!(inputs.len(), 1);
    assert!(inputs[0].selector.is_empty());
    assert!(inputs[0].variable.is_empty());
    assert_eq!(inputs[0].value_type, "boolean");
}

#[test]
fn code_outputs_fill_first_name_gap_and_ignore_missing_updates() {
    let mut state = WorkflowState::default();
    open_editor(&mut state, "code");

    state.add_node_editor_code_output_variable();
    state.add_node_editor_code_output_variable();
    state.set_node_editor_code_output_name(0, "custom".to_string());
    state.add_node_editor_code_output_variable();
    state.set_node_editor_code_output_name(9, "ignored".to_string());
    state.set_node_editor_code_output_type(9, "number".to_string());

    let WorkflowNodeVisualDraft::Code { outputs, .. } = visual_draft(&state) else {
        panic!("expected code visual draft");
    };
    assert_eq!(outputs.len(), 3);
    assert_eq!(outputs[0].key, "custom");
    assert_eq!(outputs[1].key, "result_2");
    assert_eq!(outputs[2].key, "result");
    assert!(outputs.iter().all(|output| output.value_type == "string"));
}

#[test]
fn code_default_value_syncs_empty_content_and_forced_type_changes() {
    let mut state = WorkflowState::default();
    open_editor(&mut state, "code");
    state.set_node_editor_code_error_strategy("default-value".to_string());

    if let WorkflowNodeVisualDraft::Code { default_value_editor, .. } = state
        .node_editor
        .as_mut()
        .expect("node editor")
        .visual_draft
        .as_mut()
        .expect("visual draft")
    {
        *default_value_editor = text_editor::Content::with_text("  \n");
    } else {
        panic!("expected code visual draft");
    }

    state.add_node_editor_code_output_variable();
    let WorkflowNodeVisualDraft::Code { default_value_editor, .. } = visual_draft(&state) else {
        panic!("expected code visual draft");
    };
    assert!(default_value_editor.text().contains("result"));

    if let WorkflowNodeVisualDraft::Code { default_value_editor, .. } = state
        .node_editor
        .as_mut()
        .expect("node editor")
        .visual_draft
        .as_mut()
        .expect("visual draft")
    {
        *default_value_editor = text_editor::Content::with_text("manual: true\n");
    }
    state.set_node_editor_code_output_type(0, "number".to_string());

    let WorkflowNodeVisualDraft::Code { default_value_editor, .. } = visual_draft(&state) else {
        panic!("expected code visual draft");
    };
    assert_ne!(default_value_editor.text(), "manual: true\n");
    assert!(default_value_editor.text().contains("key: result"));
    assert!(default_value_editor.text().contains("value: 0"));
}

#[test]
fn code_default_value_editor_is_not_synced_for_non_default_value_strategy() {
    let mut state = WorkflowState::default();
    open_editor(&mut state, "code");

    let initial_default = match visual_draft(&state) {
        WorkflowNodeVisualDraft::Code { default_value_editor, .. } => default_value_editor.text(),
        _ => panic!("expected code visual draft"),
    };

    state.add_node_editor_code_output_variable();
    state.set_node_editor_code_output_name(0, "result".to_string());
    state.set_node_editor_code_output_type(0, "number".to_string());

    let WorkflowNodeVisualDraft::Code { error_strategy, default_value_editor, .. } =
        visual_draft(&state)
    else {
        panic!("expected code visual draft");
    };
    assert_eq!(error_strategy, "none");
    assert_eq!(default_value_editor.text(), initial_default);
}

#[test]
fn code_error_strategy_accepts_trimmed_fail_branch() {
    let mut state = WorkflowState::default();
    open_editor(&mut state, "code");

    state.set_node_editor_code_error_strategy(" fail-branch ".to_string());

    let WorkflowNodeVisualDraft::Code { error_strategy, .. } = visual_draft(&state) else {
        panic!("expected code visual draft");
    };
    assert_eq!(error_strategy, "fail-branch");
}

#[test]
fn submit_non_visual_node_uses_default_title_trimmed_description_and_next_z_index() {
    let mut state = WorkflowState::default();
    open_editor(&mut state, "answer");
    state.submit_node_editor().expect("answer editor should submit");
    let first_z_index = state.document.nodes[0].z_index;

    open_editor(&mut state, "http-request");
    assert!(editor(&state).visual_draft.is_none());
    state.set_node_editor_title("   ".to_string());
    state.set_node_editor_description("  invokes an endpoint  ".to_string());

    state.submit_node_editor().expect("HTTP request editor should submit");

    assert_eq!(state.document.nodes.len(), 2);
    let node = &state.document.nodes[1];
    assert_eq!(node.block_type, "http-request");
    assert_eq!(node.title, "HTTP 请求");
    assert_eq!(node.description, "invokes an endpoint");
    assert_eq!(node.z_index, first_z_index + 1.0);
    assert_eq!(state.selected_node_id.as_deref(), Some(node.id.as_str()));
    assert_eq!(state.status_message.as_deref(), Some("已新增 HTTP 请求 节点"));
}

#[test]
fn submit_edit_updates_node_and_reports_pruned_invalid_edges() {
    let mut state = WorkflowState::default();
    open_editor(&mut state, "answer");
    state.submit_node_editor().expect("answer editor should submit");
    let node_id = state.document.nodes[0].id.clone();

    state.open_edit_node_editor(Some(&node_id)).expect("answer editor should reopen");
    state.set_node_editor_title("First update".to_string());
    state.submit_node_editor().expect("answer edit should submit");
    assert_eq!(state.status_message.as_deref(), Some("已更新 回复 节点"));

    state.document.edges.push(WorkflowEdge {
        id: "stale-edge".to_string(),
        source: node_id.clone(),
        target: "missing-target".to_string(),
        source_handle: Some("removed-handle".to_string()),
        target_handle: Some("target".to_string()),
        source_type: "answer".to_string(),
        target_type: "answer".to_string(),
        selected: true,
        z_index: 1.0,
        raw_edge: Value::Null,
    });
    state.selected_edge_id = Some("stale-edge".to_string());

    state.open_edit_node_editor(Some(&node_id)).expect("answer editor should reopen");
    state.set_node_editor_title("  Updated answer  ".to_string());
    state.set_node_editor_description("  revised  ".to_string());
    state.submit_node_editor().expect("answer edit should submit");

    assert_eq!(state.document.nodes.len(), 1);
    assert_eq!(state.document.nodes[0].id, node_id);
    assert_eq!(state.document.nodes[0].title, "Updated answer");
    assert_eq!(state.document.nodes[0].description, "revised");
    assert!(state.document.edges.is_empty());
    assert!(state.selected_edge_id.is_none());
    assert_eq!(state.selected_node_id.as_deref(), Some(node_id.as_str()));
    assert_eq!(state.status_message.as_deref(), Some("已更新 回复 节点，并移除 1 条失效连线"));
    assert!(state.node_editor.is_none());
}
