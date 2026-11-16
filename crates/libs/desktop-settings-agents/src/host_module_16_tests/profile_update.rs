use crate::{
    agents_state::AgentsSettingsState,
    agents_update,
    agents_update_message::{AgentsMessage, SettingsMessage},
};
#[derive(Default)]
pub(super) struct ProfileFixture {
    pub app_theme: iced::Theme,
    pub capabilities:
        Vec<(Option<String>, Vec<vw_api_acp_model::acp_model_source::AcpModelSourceCapability>)>,
    pub agents_settings: AgentsSettingsState,
    pub acp_agents: Vec<String>,
    pub mcp_settings: McpFixture,
    pub acp_settings: crate::acp_state::AcpSettingsState,
    pub desktop_locale: config_ui::DesktopLocale,
    pub project_path: Option<String>,
    pub recent_projects: Vec<String>,
}
pub(super) fn dispatch(app: &mut ProfileFixture, message: AgentsMessage) {
    let validation_error =
        app.agents_settings.profile_modal.as_ref().and_then(|draft| validation_error(app, draft));
    let mut input = agents_update::Update {
        state: &mut app.agents_settings,
        access: desktop_config::ConfigAccess::new(
            gateway_endpoint::GatewayEndpoint::new("127.0.0.1", 1),
            None,
        ),
        client: Err("offline fixture".into()),
        acp_agents: &app.acp_agents,
        acp_loading: false,
        new_session_creating: false,
        project_path: None,
        validation_error,
        effects: Vec::new(),
    };
    let _ = agents_update::update(&mut input, SettingsMessage::Agents(message));
}
pub(super) fn runtime_creation_draft() -> ProfileFixture {
    let mut app = ProfileFixture::default();
    app.agents_settings.config_ready = true;
    app.acp_agents = vec!["codex".into(), "claude".into()];
    dispatch(&mut app, AgentsMessage::AddAgentKeyChanged("runtime-custom".into()));
    dispatch(&mut app, AgentsMessage::AddAgentRequested);
    app
}
#[test]
fn agent_profile_modal_cancel_preserves_saved_profile() {
    let mut app = ProfileFixture::default();
    app.agents_settings.config_ready = true;
    let original = app.agents_settings.entries[0].label.clone();
    let rules = app.agents_settings.entries[0].system_prompt_editor.text();
    let key = app.agents_settings.entries[0].key.clone();
    dispatch(&mut app, AgentsMessage::ProfileOpen(key));
    dispatch(&mut app, AgentsMessage::ProfileNameChanged("中文资料：长文本".repeat(30)));
    dispatch(
        &mut app,
        AgentsMessage::SystemPromptAction(
            "main".into(),
            iced::widget::text_editor::Action::Edit(iced::widget::text_editor::Edit::Paste(
                std::sync::Arc::new("取消的规则".to_string()),
            )),
        ),
    );
    dispatch(&mut app, AgentsMessage::ProfileCancel);
    assert_eq!(app.agents_settings.entries[0].label, original);
    assert_eq!(app.agents_settings.entries[0].system_prompt_editor.text(), rules);
    assert!(app.agents_settings.profile_modal.is_none());
}

#[test]
fn agent_profile_modal_failed_save_preserves_draft() {
    let mut app = ProfileFixture::default();
    app.agents_settings.config_ready = true;
    let original = app.agents_settings.entries[0].label.clone();
    let key = app.agents_settings.entries[0].key.clone();
    dispatch(&mut app, AgentsMessage::ProfileOpen(key));
    dispatch(&mut app, AgentsMessage::ProfileNameChanged("待重试的资料".to_string()));
    dispatch(&mut app, AgentsMessage::ProfileSave);
    dispatch(&mut app, AgentsMessage::ProfileSaved(Err("受控保存失败".to_string())));
    assert_eq!(app.agents_settings.entries[0].label, original);
    assert_eq!(app.agents_settings.profile_modal.as_ref().unwrap().label, "待重试的资料");
}

#[test]
fn agent_profile_modal_builtin_rejects_engine_switch_and_delete() {
    let mut app = ProfileFixture::default();
    app.agents_settings.config_ready = true;
    dispatch(&mut app, AgentsMessage::ProfileOpen("main".to_string()));
    dispatch(&mut app, AgentsMessage::ProfileRuntimeChanged("codex".to_string()));
    dispatch(&mut app, AgentsMessage::ProfileDelete);
    assert!(!app.agents_settings.profile_saving);
    assert_eq!(
        app.agents_settings.profile_modal.as_ref().unwrap().runtime,
        config_agent_profile::AgentRuntime::Native
    );
}

#[test]
fn agent_profile_modal_prompts_preserve_user_order() {
    let mut app = ProfileFixture::default();
    app.agents_settings.config_ready = true;
    dispatch(&mut app, AgentsMessage::ProfileOpen("main".to_string()));
    dispatch(&mut app, AgentsMessage::ProfilePromptAdd);
    dispatch(&mut app, AgentsMessage::ProfilePromptChanged(0, "第一项".to_string()));
    dispatch(&mut app, AgentsMessage::ProfilePromptAdd);
    dispatch(&mut app, AgentsMessage::ProfilePromptChanged(1, "第二项".to_string()));
    dispatch(&mut app, AgentsMessage::ProfilePromptMove(1, 0));
    dispatch(&mut app, AgentsMessage::ProfilePromptRemove(1));
    assert_eq!(
        app.agents_settings.profile_modal.as_ref().unwrap().recommended_prompts,
        vec!["第二项"]
    );
}

#[test]
fn agent_config_cancel_discards_parameters_and_profile_together() {
    let mut app = ProfileFixture::default();
    app.agents_settings.config_ready = true;
    let original = app.agents_settings.entries[0].clone();
    dispatch(&mut app, AgentsMessage::ProfileOpen(original.key.clone()));
    dispatch(&mut app, AgentsMessage::TemperatureChanged(original.key.clone(), 1.5));
    dispatch(&mut app, AgentsMessage::MaxIterationsChanged(original.key.clone(), 123));
    dispatch(&mut app, AgentsMessage::ProfileNameChanged("修改名称".into()));
    assert_eq!(app.agents_settings.profile_modal.as_ref().unwrap().max_iterations, 123);
    dispatch(&mut app, AgentsMessage::ProfileCancel);
    assert_eq!(app.agents_settings.entries[0].temperature, original.temperature);
    assert_eq!(app.agents_settings.entries[0].max_iterations, original.max_iterations);
    assert_eq!(app.agents_settings.entries[0].label, original.label);
    assert!(!app.agents_settings.dirty);
}

#[test]
fn agent_config_create_commits_only_after_successful_save() {
    let mut app = ProfileFixture::default();
    app.agents_settings.config_ready = true;
    dispatch(&mut app, AgentsMessage::AddAgentKeyChanged("custom-agent".into()));
    dispatch(&mut app, AgentsMessage::AddAgentRequested);
    assert!(!app.agents_settings.entries.iter().any(|e| e.key == "custom-agent"));
    dispatch(&mut app, AgentsMessage::MaxIterationsChanged("custom-agent".into(), 123));
    dispatch(&mut app, AgentsMessage::ProfileNameChanged("新智能体".into()));
    dispatch(&mut app, AgentsMessage::ProfileSave);
    assert!(app.agents_settings.profile_saving);
    dispatch(&mut app, AgentsMessage::ProfileSaved(Ok(())));
    let entry = app.agents_settings.entries.iter().find(|e| e.key == "custom-agent").unwrap();
    assert_eq!(entry.label, "新智能体");
    assert_eq!(entry.max_iterations, 123);
    assert!(app.agents_settings.profile_modal.is_none());
}

#[test]
fn agent_config_cleanup_basic_combines_profile_and_runtime() {
    let mut app = ProfileFixture::default();
    app.agents_settings.config_ready = true;
    dispatch(&mut app, AgentsMessage::ProfileOpen("main".into()));
    assert_eq!(app.agents_settings.active_detail_tab, crate::state::AGENT_DETAIL_BASIC_TAB);
    dispatch(&mut app, AgentsMessage::ProfileNameChanged("合并配置".into()));
    dispatch(&mut app, AgentsMessage::TemperatureChanged("main".into(), 1.3));
    dispatch(&mut app, AgentsMessage::ProfileSave);
    assert!(app.agents_settings.profile_saving);
    dispatch(&mut app, AgentsMessage::ProfileSaved(Ok(())));
    let entry = app.agents_settings.entries.iter().find(|e| e.key == "main").unwrap();
    assert_eq!(entry.label, "合并配置");
    assert_eq!(entry.temperature, 1.3);
}

#[test]
fn agent_config_cleanup_single_system_prompt_preserves_content() {
    let mut app = ProfileFixture::default();
    app.agents_settings.config_ready = true;
    let key = app.agents_settings.entries[0].key.clone();
    let before = app.agents_settings.entries[0].system_prompt_editor.text();
    dispatch(&mut app, AgentsMessage::ProfileOpen(key.clone()));
    assert_eq!(
        app.agents_settings.profile_modal.as_ref().unwrap().system_prompt_editor.text(),
        before
    );
    dispatch(
        &mut app,
        AgentsMessage::SystemPromptAction(
            key,
            iced::widget::text_editor::Action::Edit(iced::widget::text_editor::Edit::Paste(
                std::sync::Arc::new("身份规则\n中文".into()),
            )),
        ),
    );
    dispatch(&mut app, AgentsMessage::ProfileSave);
    dispatch(&mut app, AgentsMessage::ProfileSaved(Ok(())));
    assert!(app.agents_settings.entries[0].system_prompt_editor.text().contains("身份规则\n中文"));
}

#[test]
fn agent_config_cleanup_multiline_description_roundtrips_and_cancels() {
    use iced::widget::text_editor::{Action, Edit};
    let mut app = ProfileFixture::default();
    app.agents_settings.config_ready = true;
    dispatch(&mut app, AgentsMessage::AddAgentKeyChanged("multiline".into()));
    dispatch(&mut app, AgentsMessage::AddAgentRequested);
    assert_eq!(app.agents_settings.active_detail_tab, crate::state::AGENT_DETAIL_BASIC_TAB);
    let description = "中文第一行\n第二行 English\n第三行";
    dispatch(&mut app, AgentsMessage::ProfileDescriptionAction(Action::SelectAll));
    dispatch(
        &mut app,
        AgentsMessage::ProfileDescriptionAction(Action::Edit(Edit::Paste(std::sync::Arc::new(
            description.into(),
        )))),
    );
    dispatch(&mut app, AgentsMessage::ProfileSave);
    assert!(app.agents_settings.profile_saving);
    dispatch(&mut app, AgentsMessage::ProfileSaved(Err("隔离失败".into())));
    assert_eq!(app.agents_settings.profile_modal.as_ref().unwrap().description, description);
    dispatch(&mut app, AgentsMessage::ProfileSave);
    assert!(app.agents_settings.profile_saving);
    dispatch(&mut app, AgentsMessage::ProfileSaved(Ok(())));
    dispatch(&mut app, AgentsMessage::ProfileOpen("multiline".into()));
    let draft = app.agents_settings.profile_modal.as_ref().unwrap();
    assert_eq!(draft.description, description);
    assert_eq!(draft.description_editor.text(), description);
    dispatch(
        &mut app,
        AgentsMessage::ProfileDescriptionAction(Action::Edit(Edit::Paste(std::sync::Arc::new(
            "取消内容".into(),
        )))),
    );
    dispatch(&mut app, AgentsMessage::ProfileCancel);
    dispatch(&mut app, AgentsMessage::ProfileOpen("multiline".into()));
    assert_eq!(app.agents_settings.profile_modal.as_ref().unwrap().description, description);
}

#[test]
fn agent_runtime_creation_success_locks_runtime_but_keeps_profile_editable() {
    use config_agent_profile::{AgentDefault, AgentRuntime};
    let mut app = runtime_creation_draft();
    for backend in ["codex", "claude", "native", "codex"] {
        dispatch(&mut app, AgentsMessage::ProfileRuntimeChanged(backend.into()));
        let expected = if backend == "native" {
            AgentRuntime::Native
        } else {
            AgentRuntime::Acp { backend: backend.into() }
        };
        assert_eq!(app.agents_settings.profile_modal.as_ref().unwrap().runtime, expected);
    }
    dispatch(&mut app, AgentsMessage::ProfileRuntimeChanged("unknown-custom-name".into()));
    let expected = AgentRuntime::Acp { backend: "codex".into() };
    assert_eq!(app.agents_settings.profile_modal.as_ref().unwrap().runtime, expected);
    dispatch(&mut app, AgentsMessage::ProfileSave);
    assert!(app.agents_settings.profile_saving);
    dispatch(&mut app, AgentsMessage::ProfileRuntimeChanged("claude".into()));
    dispatch(&mut app, AgentsMessage::ProfileCancel);
    assert_eq!(app.agents_settings.profile_modal.as_ref().unwrap().runtime, expected);
    dispatch(&mut app, AgentsMessage::ProfileSaved(Ok(())));
    dispatch(&mut app, AgentsMessage::ProfileOpen("runtime-custom".into()));
    for backend in ["native", "claude"] {
        dispatch(&mut app, AgentsMessage::ProfileRuntimeChanged(backend.into()));
        assert_eq!(app.agents_settings.profile_modal.as_ref().unwrap().runtime, expected);
    }
    dispatch(&mut app, AgentsMessage::ProfileNameChanged("保存后仍可改名".into()));
    dispatch(&mut app, AgentsMessage::ProfileModelChanged(AgentDefault::Inherit));
    let draft = app.agents_settings.profile_modal.as_ref().unwrap();
    assert_eq!(draft.label, "保存后仍可改名");
    assert_eq!(draft.defaults.model, AgentDefault::Inherit);
}

#[test]
fn agent_runtime_creation_failed_save_allows_retry_and_cancel_leaves_no_entry() {
    use config_agent_profile::AgentRuntime;
    let mut app = runtime_creation_draft();
    dispatch(&mut app, AgentsMessage::ProfileRuntimeChanged("codex".into()));
    dispatch(&mut app, AgentsMessage::ProfileSave);
    assert!(app.agents_settings.profile_saving);
    dispatch(&mut app, AgentsMessage::ProfileSaved(Err("受控保存失败".into())));
    dispatch(&mut app, AgentsMessage::ProfileRuntimeChanged("claude".into()));
    assert_eq!(
        app.agents_settings.profile_modal.as_ref().unwrap().runtime,
        AgentRuntime::Acp { backend: "claude".into() }
    );
    dispatch(&mut app, AgentsMessage::ProfileCancel);
    assert!(app.agents_settings.profile_modal.is_none());
    assert!(!app.agents_settings.entries.iter().any(|e| e.key == "runtime-custom"));
}

#[test]
fn agent_runtime_creation_reloaded_custom_and_builtin_reject_changes() {
    use config_agent::DelegateAgentConfig;
    use config_agent_profile::AgentRuntime;
    let mut app = ProfileFixture::default();
    app.agents_settings.config_ready = true;
    app.acp_agents = vec!["codex".into(), "claude".into()];
    let config = DelegateAgentConfig {
        runtime: AgentRuntime::Acp { backend: "claude".into() },
        ..Default::default()
    };
    app.agents_settings
        .entries
        .push(crate::state::DelegateAgentSettingsEntry::from_config("reloaded", Some(config)));
    for key in ["reloaded", "main"] {
        dispatch(&mut app, AgentsMessage::ProfileOpen(key.into()));
        let expected = app.agents_settings.profile_modal.as_ref().unwrap().runtime.clone();
        dispatch(&mut app, AgentsMessage::ProfileRuntimeChanged("codex".into()));
        assert_eq!(app.agents_settings.profile_modal.as_ref().unwrap().runtime, expected);
    }
}

#[test]
fn agent_runtime_visibility_switch_closes_and_rejects_tools_tab_without_clearing_values() {
    use crate::state::{AGENT_DETAIL_BASIC_TAB, AGENT_DETAIL_TOOLS_TAB};
    let mut app = runtime_creation_draft();
    let draft = app.agents_settings.profile_modal.as_mut().unwrap();
    draft.api_key_input = "retained-override".into();
    draft.temperature = 1.23;
    draft.allowed_tools = vec!["shell".into()];
    draft.max_iterations = 123;
    draft.max_depth = 7;
    draft.max_parallel_subagents = 3;
    draft.compact_context = true;
    draft.parallel_tools = true;
    draft.max_history_messages = 42;
    let original = draft.clone();
    dispatch(&mut app, AgentsMessage::DetailTabSelected(AGENT_DETAIL_TOOLS_TAB.into()));
    assert_eq!(app.agents_settings.active_detail_tab, AGENT_DETAIL_TOOLS_TAB);
    dispatch(&mut app, AgentsMessage::ProfileRuntimeChanged("codex".into()));
    assert_eq!(app.agents_settings.active_detail_tab, AGENT_DETAIL_BASIC_TAB);
    dispatch(&mut app, AgentsMessage::DetailTabSelected(AGENT_DETAIL_TOOLS_TAB.into()));
    assert_eq!(app.agents_settings.active_detail_tab, AGENT_DETAIL_BASIC_TAB);
    dispatch(&mut app, AgentsMessage::ProfileRuntimeChanged("native".into()));
    dispatch(&mut app, AgentsMessage::DetailTabSelected(AGENT_DETAIL_TOOLS_TAB.into()));
    assert_eq!(app.agents_settings.active_detail_tab, AGENT_DETAIL_TOOLS_TAB);
    dispatch(&mut app, AgentsMessage::ProfileRuntimeChanged("claude".into()));
    assert_eq!(app.agents_settings.active_detail_tab, AGENT_DETAIL_BASIC_TAB);
    dispatch(&mut app, AgentsMessage::ProfileSave);
    assert!(app.agents_settings.profile_saving);
    dispatch(&mut app, AgentsMessage::ProfileSaved(Ok(())));
    let saved = app.agents_settings.entries.iter().find(|e| e.key == original.key).unwrap();
    assert_eq!(saved.allowed_tools, original.allowed_tools);
    assert_eq!(saved.max_iterations, original.max_iterations);
    assert_eq!(saved.max_depth, original.max_depth);
    assert_eq!(saved.max_parallel_subagents, original.max_parallel_subagents);
    assert_eq!(saved.api_key_input, original.api_key_input);
    assert_eq!(saved.temperature, original.temperature);
    assert_eq!(saved.defaults.permission, original.defaults.permission);
    assert_eq!(saved.compact_context, original.compact_context);
    assert_eq!(saved.parallel_tools, original.parallel_tools);
    assert_eq!(saved.max_history_messages, original.max_history_messages);
}

#[derive(Default)]
pub(super) struct McpFixture {
    pub inspect: Option<vw_api_mcp_inspect::McpInspectResponse>,
    pub inspect_loading: bool,
    pub inspect_error: Option<String>,
}
pub(super) fn defaults_input<'a>(
    app: &'a ProfileFixture,
    draft: &crate::state::DelegateAgentSettingsEntry,
) -> crate::defaults::Input<'a> {
    crate::defaults::catalog_input(
        &app.agents_settings,
        app.desktop_locale,
        app.app_theme,
        super::selectable_help,
        crate::defaults::supports_fixed_model(
            &draft.runtime,
            app.capabilities.iter().map(|(agent, caps)| (agent.as_deref(), caps.as_slice())),
        ),
        app.mcp_settings.inspect.as_ref(),
        app.mcp_settings.inspect_loading,
        app.mcp_settings.inspect_error.as_ref(),
    )
}
pub(super) fn validation_error(
    app: &ProfileFixture,
    draft: &crate::state::DelegateAgentSettingsEntry,
) -> Option<String> {
    crate::defaults::validation_error(&defaults_input(app, draft), draft)
}

#[test]
fn agent_runtime_empty_directory_keeps_native() {
    let mut app = ProfileFixture::default();
    app.agents_settings.config_ready = true;
    dispatch(&mut app, AgentsMessage::AddAgentKeyChanged("empty-catalog".into()));
    dispatch(&mut app, AgentsMessage::AddAgentRequested);
    dispatch(&mut app, AgentsMessage::ProfileRuntimeChanged("codex".into()));
    assert_eq!(
        app.agents_settings.profile_modal.as_ref().unwrap().runtime,
        config_agent_profile::AgentRuntime::Native
    );
}
