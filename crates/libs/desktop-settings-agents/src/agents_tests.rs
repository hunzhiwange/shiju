//! 宿主组合状态与领域条目的接线检查；纯领域断言随实现迁移。
use super::AgentsSettingsState;
use crate::state::{
    AGENT_DETAIL_BASIC_TAB, AGENT_PROMPT_SYSTEM_TAB, AGENT_TOOLS_OPTIONAL_TAB, AgentConfigTab,
    AgentSettingsEntryKind, MAIN_AGENT_KEY, WORKSPACE_IDENTITY_FILES,
};
use config_agent::builtin_agent_keys;

#[test]
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("agents_tests"));
}

#[test]
fn agents_settings_default_waits_for_config_before_allowing_save() {
    let state = AgentsSettingsState::default();

    assert!(state.loading);
    assert!(!state.config_ready);
    assert!(state.providers.is_empty());
    assert!(state.provider_models.is_empty());
    assert_eq!(state.selected_agent, MAIN_AGENT_KEY);
    assert_eq!(state.active_config_tab, AgentConfigTab::Agents);
    let config_tabs = [
        AgentConfigTab::Agents,
        AgentConfigTab::Channels,
        AgentConfigTab::Runtime,
        AgentConfigTab::Acp,
        AgentConfigTab::Mcp,
        AgentConfigTab::Skills,
        AgentConfigTab::Plugins,
        AgentConfigTab::Reliability,
        AgentConfigTab::DialogueFlow,
        AgentConfigTab::QuickMessages,
        AgentConfigTab::Heartbeat,
    ];
    assert_eq!(config_tabs.len(), 11);
    assert_eq!(state.active_detail_tab, AGENT_DETAIL_BASIC_TAB);
    assert_eq!(state.active_tools_list_tab, AGENT_TOOLS_OPTIONAL_TAB);
    assert_eq!(state.active_prompt_tab, AGENT_PROMPT_SYSTEM_TAB);
    assert!(state.new_agent_key_input.is_empty());
    assert!(state.available_tools.is_empty());
    assert!(state.workspace_identity_root_path.is_none());
    assert!(state.save_error.is_none());
    assert!(!state.save_in_flight);
    assert!(!state.dirty);

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
