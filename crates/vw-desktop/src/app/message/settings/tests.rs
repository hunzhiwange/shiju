use super::*;
use crate::app::App;
use crate::app::state::StaffSettingsState;
use vw_config_types::staff::{StaffChannelBindingConfig, StaffConfig};

fn app() -> App {
    App::new().0
}

#[test]
fn settings_apply_agent_config_saved_routes_errors() {
    let mut app = app();

    apply_agent_config_saved(&mut app, "browser", Err("boom".to_string()));
    assert!(app.browser_settings.save_error.is_some());

    apply_agent_config_saved(&mut app, "memory", Ok(()));
    assert!(app.memory_settings.save_error.is_none());

    apply_agent_config_saved(&mut app, "staff", Err("boom".to_string()));
    assert!(app.staff_settings.save_error.is_some());
    assert!(!app.staff_settings.save_success);

    apply_agent_config_saved(&mut app, "staff", Ok(()));
    assert!(app.staff_settings.save_error.is_none());
    assert!(app.staff_settings.save_success);

    apply_agent_config_saved(&mut app, "unknown", Err("ignored".to_string()));
}

#[test]
fn every_non_staff_save_tag_routes_error_and_success_to_its_own_state() {
    let mut app = app();

    macro_rules! assert_route {
        ($tag:literal, $field:ident) => {{
            app.$field.save_error = Some("stale".to_string());
            apply_agent_config_saved(&mut app, $tag, Err("gateway offline".to_string()));
            assert_ne!(app.$field.save_error.as_deref(), Some("stale"), "{} error route", $tag);

            apply_agent_config_saved(&mut app, $tag, Ok(()));
            assert!(app.$field.save_error.is_none(), "{} success route", $tag);
        }};
    }

    assert_route!("heartbeat", heartbeat_settings);
    assert_route!("goal_loop", goal_loop_settings);
    assert_route!("cron", cron_settings);
    assert_route!("sop", sop_settings);
    assert_route!("scheduler", scheduler_settings);
    assert_route!("reliability", reliability_settings);
    assert_route!("memory", memory_settings);
    assert_route!("security", security_settings);
    assert_route!("channels_config", channels_settings);
    assert_route!("observability", observability_settings);
    assert_route!("storage", storage_settings);
    assert_route!("proxy", proxy_settings);
    assert_route!("browser", browser_settings);
    assert_route!("http_request", http_request_settings);
    assert_route!("multimodal", multimodal_settings);
    assert_route!("web_search", web_search_settings);
    assert_route!("tunnel", tunnel_settings);
    assert_route!("hooks", hooks_settings);
    assert_route!("composio", composio_settings);
    assert_route!("skills", skills_settings);
    assert_route!("research", research_settings);
    assert_route!("agent", agents_settings);
    assert_route!("runtime", runtime_settings);
    assert_route!("model_routes", model_routes_settings);
    assert_route!("embedding_routes", embedding_routes_settings);
    assert_route!("query_classification", query_classification_settings);
    assert_route!("autonomy", autonomy_settings);
    assert_route!("agents_ipc", agents_ipc_settings);
    assert_route!("coordination", coordination_settings);
    assert_route!("transcription", transcription_settings);
}

#[test]
fn unknown_save_tag_leaves_existing_feedback_unchanged() {
    let mut app = app();
    app.browser_settings.save_error = Some("keep".to_string());

    apply_agent_config_saved(&mut app, "unknown", Err("ignored".to_string()));

    assert_eq!(app.browser_settings.save_error.as_deref(), Some("keep"));
}

#[test]
fn staff_config_saved_refreshes_task_board_staff_projection() {
    let mut app = app();
    let mut config = StaffConfig::default();
    let executor = config
        .members
        .iter_mut()
        .find(|member| member.id == "staff-executor")
        .expect("default executor");
    executor.capacity = 5;
    executor.delegate_agent = Some("coder".to_string());
    executor.acp_agent = None;
    app.staff_settings.config = config;

    apply_agent_config_saved(&mut app, "staff", Ok(()));

    let member = app
        .task_board_staff_config
        .members
        .iter()
        .find(|member| member.id == "staff-executor")
        .expect("projected executor");
    assert_eq!(member.capacity, 5);
    assert_eq!(member.delegate_agent.as_deref(), Some("coder"));

    let load = app
        .task_board_staff_projection
        .member_loads
        .iter()
        .find(|member| member.member_id == "staff-executor")
        .expect("executor load");
    assert_eq!(load.capacity, 5);
    assert_eq!(load.delegate_agent.as_deref(), Some("coder"));
}

#[test]
fn staff_reset_draft_restores_saved_config() {
    let mut app = app();
    let mut config = StaffConfig::default();
    let executor = config
        .members
        .iter_mut()
        .find(|member| member.id == "staff-executor")
        .expect("default executor");
    executor.capacity = 5;
    app.staff_settings = StaffSettingsState::from_config(config);
    app.staff_settings
        .member_drafts
        .iter_mut()
        .find(|draft| draft.id == "staff-executor")
        .expect("executor draft")
        .capacity_input = "2".to_string();

    assert!(app.staff_settings.has_unsaved_changes());

    let _task = update(&mut app, SettingsMessage::Staff(StaffMessage::ResetDraftRequested));

    assert!(!app.staff_settings.has_unsaved_changes());
    let executor = app
        .staff_settings
        .member_drafts
        .iter()
        .find(|draft| draft.id == "staff-executor")
        .expect("executor draft");
    assert_eq!(executor.capacity_input, "5");
}

#[test]
fn staff_save_failure_keeps_unsaved_changes_retryable() {
    let mut app = app();
    let mut config = StaffConfig::default();
    let executor = config
        .members
        .iter_mut()
        .find(|member| member.id == "staff-executor")
        .expect("default executor");
    executor.capacity = 5;
    app.staff_settings = StaffSettingsState::from_config(config);
    app.staff_settings
        .member_drafts
        .iter_mut()
        .find(|draft| draft.id == "staff-executor")
        .expect("executor draft")
        .capacity_input = "6".to_string();
    app.staff_settings.pending_config = app.staff_settings.to_config().ok();

    apply_agent_config_saved(&mut app, "staff", Err("gateway down".to_string()));

    assert!(app.staff_settings.has_unsaved_changes());
    assert!(app.staff_settings.pending_config.is_none());
    assert!(app.staff_settings.save_error.is_some());
}

#[test]
fn staff_channel_binding_messages_update_member_binding_drafts() {
    let mut app = app();
    app.staff_settings = StaffSettingsState::from_config(StaffConfig::default());
    let member_id = "staff-planner".to_string();

    let _ = update(
        &mut app,
        SettingsMessage::Staff(StaffMessage::ChannelBindingAdded(member_id.clone())),
    );
    let draft = app
        .staff_settings
        .member_drafts
        .iter()
        .find(|draft| draft.id == member_id)
        .expect("planner draft");
    assert_eq!(draft.channel_bindings.len(), 1);
    assert_eq!(draft.channel_bindings[0].channel_type, "matrix");

    let _ = update(
        &mut app,
        SettingsMessage::Staff(StaffMessage::ChannelBindingChannelTypeChanged(
            member_id.clone(),
            0,
            "feishu".to_string(),
        )),
    );
    let _ = update(
        &mut app,
        SettingsMessage::Staff(StaffMessage::ChannelBindingInstanceIdChanged(
            member_id.clone(),
            0,
            " planner-app ".to_string(),
        )),
    );
    let config = app.staff_settings.to_config().expect("staff config");
    let planner =
        config.members.iter().find(|member| member.id == member_id).expect("planner config");
    assert_eq!(
        planner.channel_bindings,
        vec![StaffChannelBindingConfig {
            channel_type: "feishu".to_string(),
            instance_id: "planner-app".to_string(),
        }]
    );

    let _ = update(
        &mut app,
        SettingsMessage::Staff(StaffMessage::ChannelBindingRemoved(member_id.clone(), 0)),
    );
    let draft = app
        .staff_settings
        .member_drafts
        .iter()
        .find(|draft| draft.id == member_id)
        .expect("planner draft");
    assert!(draft.channel_bindings.is_empty());
}

#[test]
fn settings_update_dispatches_to_child_modules() {
    let mut app = app();

    let _ = update(&mut app, SettingsMessage::HeartbeatHelpOpen);
    assert!(app.heartbeat_settings.show_help_modal);

    let _ = update(
        &mut app,
        SettingsMessage::Channels(ChannelsMessage::PanelToggled("telegram".to_string())),
    );
    assert!(app.channels_settings.expanded_panels.contains("telegram"));

    let _ = update(
        &mut app,
        SettingsMessage::AgentConfigSaved { tag: "hooks", result: Err("x".to_string()) },
    );
    assert!(app.hooks_settings.save_error.is_some());
}

#[test]
fn settings_update_routes_nested_messages_to_their_owned_state() {
    let mut app = app();
    let embedding_route_count = app.embedding_routes_settings.routes.len();
    app.query_classification_settings.save_error = Some("stale".to_string());

    let _ = update(&mut app, SettingsMessage::EmbeddingRoutes(EmbeddingRoutesMessage::AddRoute));
    let _ = update(&mut app, SettingsMessage::Hooks(HooksMessage::EnabledToggled(true)));
    let _ = update(
        &mut app,
        SettingsMessage::Storage(StorageMessage::ProviderChanged("postgres".to_string())),
    );
    let _ = update(&mut app, SettingsMessage::ModelRoutes(ModelRoutesMessage::Refresh));
    let _ =
        update(&mut app, SettingsMessage::QueryClassification(QueryClassificationMessage::Refresh));
    let _ = update(
        &mut app,
        SettingsMessage::Runtime(RuntimeMessage::KindChanged("docker".to_string())),
    );
    let _ = update(&mut app, SettingsMessage::Browser(BrowserMessage::EnabledToggled(true)));
    let _ = update(&mut app, SettingsMessage::WebSearch(WebSearchMessage::HelpOpen));
    let _ = update(&mut app, SettingsMessage::GoalLoop(GoalLoopMessage::EnabledToggled(true)));
    let _ =
        update(&mut app, SettingsMessage::HttpRequest(HttpRequestMessage::EnabledToggled(true)));
    let _ = update(
        &mut app,
        SettingsMessage::Memory(MemoryMessage::BackendChanged("none".to_string())),
    );
    let _ = update(
        &mut app,
        SettingsMessage::Multimodal(MultimodalMessage::AllowRemoteFetchToggled(true)),
    );
    let _ = update(
        &mut app,
        SettingsMessage::Tunnel(TunnelMessage::ProviderChanged("ngrok".to_string())),
    );
    let _ = update(
        &mut app,
        SettingsMessage::Sop(SopMessage::SopsDirChanged("workspace/sops".to_string())),
    );

    assert_eq!(app.embedding_routes_settings.routes.len(), embedding_route_count + 1);
    assert!(app.hooks_settings.enabled);
    assert_eq!(app.storage_settings.provider, "postgres");
    assert!(app.query_classification_settings.save_error.is_none());
    assert_eq!(app.runtime_settings.kind, "docker");
    assert!(app.browser_settings.enabled);
    assert!(app.web_search_settings.show_help_modal);
    assert!(app.goal_loop_settings.enabled);
    assert!(app.http_request_settings.enabled);
    assert_eq!(app.memory_settings.backend, "none");
    assert!(app.multimodal_settings.allow_remote_fetch);
    assert_eq!(app.tunnel_settings.provider, "ngrok");
    assert_eq!(app.sop_settings.sops_dir_input, "workspace/sops");
}

#[test]
fn settings_update_routes_standalone_help_messages() {
    let mut app = app();

    let _ = update(&mut app, SettingsMessage::CronHelpOpen);
    let _ = update(&mut app, SettingsMessage::SchedulerHelpOpen);
    let _ = update(&mut app, SettingsMessage::AgentsIpcHelpOpen);
    let _ = update(&mut app, SettingsMessage::CoordinationHelpOpen);
    let _ = update(&mut app, SettingsMessage::ReliabilityHelpOpen);
    let _ = update(&mut app, SettingsMessage::SecurityHelpOpen);
    let _ = update(&mut app, SettingsMessage::AutonomyHelpOpen);
    let _ = update(&mut app, SettingsMessage::ObservabilityHelpOpen);
    let _ = update(&mut app, SettingsMessage::ProxyHelpOpen);
    let _ = update(&mut app, SettingsMessage::ResearchHelpOpen);
    let _ = update(&mut app, SettingsMessage::TranscriptionHelpOpen);
    let _ = update(&mut app, SettingsMessage::SkillsHelpOpen);

    assert!(app.cron_settings.show_help_modal);
    assert!(app.scheduler_settings.show_help_modal);
    assert!(app.agents_ipc_settings.show_help_modal);
    assert!(app.coordination_settings.show_help_modal);
    assert!(app.reliability_settings.show_help_modal);
    assert!(app.security_settings.show_help_modal);
    assert!(app.autonomy_settings.show_help_modal);
    assert!(app.observability_settings.show_help_modal);
    assert!(app.proxy_settings.show_help_modal);
    assert!(app.research_settings.show_help_modal);
    assert!(app.transcription_settings.show_help_modal);
    assert!(app.skills_settings.show_help_modal);
}

#[test]
fn settings_update_routes_additional_feature_groups() {
    let mut app = app();
    app.acp_settings.loading = false;
    app.gateway_client_settings.show_help_modal = false;
    app.gateway_settings.show_help_modal = false;
    app.provider_settings.loading = false;
    app.model_settings.loading = false;
    app.skills_settings.show_help_modal = false;

    let _ = update(&mut app, SettingsMessage::Acp(AcpMessage::Refresh));
    let _ = update(&mut app, SettingsMessage::GatewayClient(GatewayClientMessage::HelpOpen));
    let _ = update(&mut app, SettingsMessage::Gateway(GatewayMessage::HelpOpen));
    let _ = update(&mut app, SettingsMessage::ProvidersRefresh);
    let _ = update(&mut app, SettingsMessage::ModelsRefresh);
    let _ = update(&mut app, SettingsMessage::SkillsHelpOpen);

    assert!(app.acp_settings.loading);
    assert!(app.gateway_client_settings.show_help_modal);
    assert!(app.gateway_settings.show_help_modal);
    assert!(app.provider_settings.loading);
    assert!(app.model_settings.loading);
    assert!(app.skills_settings.show_help_modal);
}

#[test]
fn settings_update_routes_tabs_projects_sessions_dialogue_flow_and_composio_groups() {
    let mut app = app();
    let original_sidebar_state = app.settings_sidebar_collapsed;
    app.recent_projects = vec!["/workspace/project-a".to_string()];
    app.recent_project_delete_confirm_idx = None;
    app.dialogue_flow_show_reasoning_summary = false;
    app.composio_settings.enabled = false;

    let _ = update(&mut app, SettingsMessage::ToggleSettingsSidebar);
    let _ = update(&mut app, SettingsMessage::RecentProjectDeleteRequested(0));
    let _ = update(&mut app, SettingsMessage::SessionDelete("missing".to_string()));
    let _ = update(&mut app, SettingsMessage::DialogueFlowShowReasoningSummaryToggled(true));
    let _ = update(&mut app, SettingsMessage::ComposioEnabledToggled(true));

    assert_eq!(app.settings_sidebar_collapsed, !original_sidebar_state);
    assert_eq!(app.recent_project_delete_confirm_idx, Some(0));
    assert!(app.dialogue_flow_show_reasoning_summary);
    assert!(app.composio_settings.enabled);
}
