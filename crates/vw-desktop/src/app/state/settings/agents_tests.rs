#[test]
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("agents_tests"));
}

use super::{AgentsSettingsState, DelegateAgentSettingsEntry, ordered_agent_keys};
use crate::app::state::{
    AGENT_DETAIL_BASIC_TAB, AGENT_PROMPT_SYSTEM_TAB, AgentSettingsEntryKind, MAIN_AGENT_KEY,
    WORKSPACE_IDENTITY_FILES,
};
use std::collections::HashMap;
use vw_config_types::agent::{DelegateAgentConfig, builtin_agent_keys};

#[test]
fn delegate_agent_entry_uses_main_kind_and_forces_main_enabled() {
    let config = DelegateAgentConfig {
        enabled: false,
        label: Some("Primary".to_string()),
        ..Default::default()
    };

    let entry = DelegateAgentSettingsEntry::from_config(MAIN_AGENT_KEY, Some(config));

    assert_eq!(entry.kind, AgentSettingsEntryKind::Main);
    assert!(entry.enabled);
    assert_eq!(entry.label, "Primary");
}

#[test]
fn delegate_agent_entry_uses_builtin_worker_defaults() {
    let entry = DelegateAgentSettingsEntry::from_config("title", None);

    assert_eq!(entry.key, "title");
    assert_eq!(entry.label, "Title");
    assert_eq!(entry.kind, AgentSettingsEntryKind::BuiltinWorker);
    assert!(entry.enabled);
    assert_eq!(entry.temperature, 0.5);
    assert_eq!(entry.max_depth, 3);
    assert_eq!(entry.max_iterations, 10);
}

#[test]
fn delegate_agent_entry_uses_custom_defaults_when_config_is_missing() {
    let entry = DelegateAgentSettingsEntry::from_config("custom-agent", None);

    assert_eq!(entry.key, "custom-agent");
    assert_eq!(entry.label, "custom-agent");
    assert_eq!(entry.kind, AgentSettingsEntryKind::Custom);
    assert!(entry.enabled);
    assert!((entry.temperature - 0.7).abs() < f32::EPSILON);
    assert_eq!(entry.system_prompt_editor.text(), "");
    assert_eq!(entry.api_key_input, "");
    assert_eq!(entry.max_depth, 3);
    assert_eq!(entry.max_iterations, 10);
}

#[test]
fn delegate_agent_entry_clamps_and_copies_config_values() {
    let config = DelegateAgentConfig {
        temperature: Some(5.0),
        max_depth: 0,
        max_iterations: 500,
        enabled: false,
        provider: "openai".to_string(),
        model: "gpt-4.1".to_string(),
        system_prompt: Some("  prompt  ".to_string()),
        api_key: Some("test-api-key".to_string()),
        agentic: true,
        allowed_tools: vec!["read".to_string()],
        allowed_skills: vec!["skill-a".to_string()],
        ..Default::default()
    };

    let entry = DelegateAgentSettingsEntry::from_config("custom", Some(config));

    assert_eq!(entry.kind, AgentSettingsEntryKind::Custom);
    assert!(!entry.enabled);
    assert_eq!(entry.provider, "openai");
    assert_eq!(entry.model, "gpt-4.1");
    assert_eq!(entry.temperature, 2.0);
    assert_eq!(entry.max_depth, 1);
    assert_eq!(entry.max_iterations, 100);
    assert_eq!(entry.system_prompt_editor.text(), "  prompt  ");
    assert_eq!(entry.api_key_input, "test-api-key");
    assert!(entry.agentic);
    assert_eq!(entry.allowed_tools, vec!["read"]);
    assert_eq!(entry.allowed_skills, vec!["skill-a"]);
    assert!(!entry.compact_context);
    assert_eq!(entry.max_tool_iterations, 20);
    assert_eq!(entry.max_history_messages, 50);
    assert!(!entry.parallel_tools);
    assert_eq!(entry.tool_dispatcher, "auto");
}

#[test]
fn delegate_agent_entry_replaces_blank_text_and_clamps_numeric_lower_bounds() {
    let config = DelegateAgentConfig {
        label: Some("  ".to_string()),
        system_prompt: Some(" \n\t ".to_string()),
        temperature: Some(-1.0),
        max_depth: 99,
        max_iterations: 0,
        ..Default::default()
    };

    let entry = DelegateAgentSettingsEntry::from_config("custom-agent", Some(config));

    assert_eq!(entry.label, "custom-agent");
    assert_eq!(entry.system_prompt_editor.text(), "");
    assert_eq!(entry.temperature, 0.0);
    assert_eq!(entry.max_depth, 32);
    assert_eq!(entry.max_iterations, 1);
}

#[test]
fn delegate_agent_entry_blank_builtin_label_falls_back_to_builtin_label() {
    let config = DelegateAgentConfig { label: Some(" \n ".to_string()), ..Default::default() };

    let entry = DelegateAgentSettingsEntry::from_config("title", Some(config));

    assert_eq!(entry.kind, AgentSettingsEntryKind::BuiltinWorker);
    assert_eq!(entry.label, "Title");
}

#[test]
fn ordered_agent_keys_puts_builtin_keys_before_sorted_custom_keys() {
    let mut configured = HashMap::new();
    configured.insert("zeta".to_string(), DelegateAgentConfig::default());
    configured.insert("alpha".to_string(), DelegateAgentConfig::default());

    let keys = ordered_agent_keys(&configured);
    let alpha_index = keys.iter().position(|key| key == "alpha").expect("alpha key missing");
    let zeta_index = keys.iter().position(|key| key == "zeta").expect("zeta key missing");
    let main_index = keys.iter().position(|key| key == MAIN_AGENT_KEY).expect("main key missing");

    assert!(main_index < alpha_index);
    assert!(alpha_index < zeta_index);
}

#[test]
fn ordered_agent_keys_does_not_duplicate_configured_builtin_keys() {
    let mut configured = HashMap::new();
    configured.insert(MAIN_AGENT_KEY.to_string(), DelegateAgentConfig::default());
    configured.insert("reviewer".to_string(), DelegateAgentConfig::default());
    configured.insert("custom".to_string(), DelegateAgentConfig::default());

    let keys = ordered_agent_keys(&configured);
    let expected_builtin_keys = builtin_agent_keys();
    let actual_builtin_keys =
        keys.iter().take(expected_builtin_keys.len()).map(String::as_str).collect::<Vec<_>>();

    assert_eq!(actual_builtin_keys, expected_builtin_keys);
    assert_eq!(keys.iter().filter(|key| key.as_str() == MAIN_AGENT_KEY).count(), 1);
    assert_eq!(keys.iter().filter(|key| key.as_str() == "reviewer").count(), 1);
    assert_eq!(keys.last().map(String::as_str), Some("custom"));
}

#[test]
fn agents_settings_default_initializes_complete_idle_state() {
    let state = AgentsSettingsState::default();

    assert!(!state.loading);
    assert!(state.providers.is_empty());
    assert!(state.provider_models.is_empty());
    assert_eq!(state.selected_agent, MAIN_AGENT_KEY);
    assert_eq!(state.active_detail_tab, AGENT_DETAIL_BASIC_TAB);
    assert_eq!(state.active_prompt_tab, AGENT_PROMPT_SYSTEM_TAB);
    assert!(state.new_agent_key_input.is_empty());
    assert!(state.available_tools.is_empty());
    assert!(state.workspace_identity_root_path.is_none());
    assert!(state.save_error.is_none());

    let expected_keys = builtin_agent_keys();
    assert_eq!(state.entries.len(), expected_keys.len());
    assert_eq!(
        state.entries.iter().map(|entry| entry.key.as_str()).collect::<Vec<_>>(),
        expected_keys
    );
    assert_eq!(state.entries[0].kind, AgentSettingsEntryKind::Main);
    assert!(
        state
            .entries
            .iter()
            .skip(1)
            .all(|entry| entry.kind == AgentSettingsEntryKind::BuiltinWorker)
    );

    assert_eq!(state.workspace_identity_files.len(), WORKSPACE_IDENTITY_FILES.len());
    for (file, (expected_name, expected_label)) in
        state.workspace_identity_files.iter().zip(WORKSPACE_IDENTITY_FILES)
    {
        assert_eq!(file.file_name, expected_name);
        assert_eq!(file.label, expected_label);
        assert_eq!(file.editor.text(), "");
        assert!(file.size_bytes.is_none());
        assert!(file.modified_at_ms.is_none());
    }
}
