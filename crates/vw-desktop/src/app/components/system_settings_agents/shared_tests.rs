//! 验证代理设置共享控件、标签与状态样式的构建契约。
//! 帮助文本、禁用状态和明暗主题用例共同约束共享组件的一致性。

use super::shared::selected_entry;
use crate::app::state::{AgentSettingsEntryKind, DelegateAgentSettingsEntry};
use iced::widget::text_editor;

fn entry(key: &str, kind: AgentSettingsEntryKind) -> DelegateAgentSettingsEntry {
    DelegateAgentSettingsEntry {
        defaults: Default::default(),
        key: key.to_string(),
        builtin: false,
        label: key.to_string(),
        runtime: Default::default(),
        recommended_prompts: Vec::new(),
        description: String::new(),
        description_editor: text_editor::Content::new(),
        kind,
        role_controls_locked: false,
        can_disable: true,
        enabled: true,
        allow_as_subagent: false,
        max_parallel_subagents: 4,
        provider: String::new(),
        model: String::new(),
        force_model_selection: false,
        system_prompt_editor: text_editor::Content::new(),
        api_key_input: String::new(),
        temperature: 0.7,
        compact_context: false,
        max_history_messages: 100,
        parallel_tools: false,
        max_depth: 1,
        agentic: false,
        allowed_tools: Vec::new(),
        skill_policy: config_agent::DelegateCapabilityPolicy::default(),
        allowed_skills: Vec::new(),
        enabled_identity_files: Vec::new(),
        max_iterations: 50_000,
    }
}

#[test]
fn selected_entry_returns_exact_selected_agent() {
    let (mut app, _) = crate::app::App::new();
    app.agents_settings.entries = vec![
        entry("first", AgentSettingsEntryKind::Custom),
        entry("second", AgentSettingsEntryKind::BuiltinWorker),
    ];
    app.agents_settings.selected_agent = "second".to_string();

    assert_eq!(selected_entry(&app).map(|entry| entry.key.as_str()), Some("second"));
}

#[test]
fn selected_entry_falls_back_to_first_agent_when_selection_is_missing() {
    let (mut app, _) = crate::app::App::new();
    app.agents_settings.entries = vec![
        entry("first", AgentSettingsEntryKind::Custom),
        entry("second", AgentSettingsEntryKind::BuiltinWorker),
    ];
    app.agents_settings.selected_agent = "missing".to_string();

    assert_eq!(selected_entry(&app).map(|entry| entry.key.as_str()), Some("first"));
}

#[test]
fn selected_entry_returns_none_when_agent_list_is_empty() {
    let (mut app, _) = crate::app::App::new();
    app.agents_settings.entries.clear();

    assert!(selected_entry(&app).is_none());
}
