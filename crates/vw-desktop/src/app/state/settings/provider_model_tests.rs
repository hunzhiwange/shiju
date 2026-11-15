//! 提供商与模型设置测试，覆盖连接草稿、模型详情、路由和调度器默认契约。
//! 公开摘要字段及端口、并发等安全限值应与设置界面的编辑语义一致。

use super::*;

#[test]
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("provider_model_tests"));
}

#[test]
fn model_detail_and_model_settings_defaults_are_stable() {
    let custom_modal = CustomProviderModelModalState {
        active_tab: CustomProviderModelTab::Capabilities,
        edit_index: Some(2),
        model_id: "claude-sonnet".to_string(),
        display_name: "Claude Sonnet".to_string(),
        family: "claude".to_string(),
        context_limit: "200000".to_string(),
        input_limit: "190000".to_string(),
        output_limit: "10000".to_string(),
        reasoning: Some(true),
        tool_call: Some(true),
        temperature: Some(false),
        attachment: Some(true),
        interleaved: Some(true),
        input_modalities: Some(vec!["text".to_string(), "image".to_string()]),
        output_modalities: Some(vec!["text".to_string()]),
        inherited_capabilities: None,
        original_model_id: Some("claude-sonnet".to_string()),
    };
    assert_eq!(custom_modal.active_tab, CustomProviderModelTab::Capabilities);
    assert_eq!(custom_modal.edit_index, Some(2));
    assert_eq!(custom_modal.model_id, "claude-sonnet");
    assert_eq!(custom_modal.display_name, "Claude Sonnet");
    assert_eq!(custom_modal.family, "claude");
    assert_eq!(custom_modal.context_limit, "200000");
    assert_eq!(custom_modal.input_limit, "190000");
    assert_eq!(custom_modal.output_limit, "10000");
    assert_eq!(custom_modal.reasoning, Some(true));
    assert_eq!(custom_modal.tool_call, Some(true));
    assert_eq!(custom_modal.temperature, Some(false));
    assert_eq!(custom_modal.attachment, Some(true));
    assert_eq!(custom_modal.interleaved, Some(true));
    assert_eq!(
        custom_modal.input_modalities.as_deref(),
        Some(["text".to_string(), "image".to_string()].as_slice())
    );
    assert_eq!(custom_modal.output_modalities.as_deref(), Some(["text".to_string()].as_slice()));

    let row =
        ModelDetailRow { label: "Context".to_string().into(), value: "128k".to_string().into() };
    let modal = ModelDetailModalState {
        provider_id: "openai".to_string(),
        provider_name: "OpenAI".to_string(),
        model_id: "gpt".to_string(),
        model_name: "GPT".to_string(),
        rows: vec![row],
        raw_json: "{}".to_string(),
        show_raw: true,
    };
    assert_eq!(modal.provider_id, "openai");
    assert_eq!(modal.provider_name, "OpenAI");
    assert_eq!(modal.model_id, "gpt");
    assert_eq!(modal.model_name, "GPT");
    assert_eq!(modal.rows[0].label, "Context");
    assert_eq!(modal.rows[0].value, "128k");
    assert_eq!(modal.raw_json, "{}");
    assert!(modal.show_raw);

    let state = ModelSettingsState::default();
    assert!(!state.loading);
    assert!(state.query.is_empty());
    assert!(state.providers.is_empty());
    assert!(state.save_error.is_none());
    assert!(state.detail_modal.is_none());
}

#[test]
fn route_classification_and_heartbeat_defaults_are_ready_to_edit() {
    let embedding = EmbeddingRouteDraft::default();
    assert!(embedding.pattern.is_empty());
    assert!(embedding.provider.is_empty());
    assert!(embedding.model.is_empty());
    assert!(embedding.dimensions.is_empty());
    assert!(embedding.api_key_input.is_empty());
    let embedding_routes = EmbeddingRoutesSettingsState::default();
    assert!(embedding_routes.routes.is_empty());
    assert!(embedding_routes.save_error.is_none());
    assert!(!embedding_routes.save_success);

    let route = ModelRoute::default();
    assert!(route.pattern.is_empty());
    assert!(route.provider.is_empty());
    assert!(route.model.is_empty());
    assert!(route.priority_input.is_empty());
    let model_routes = ModelRoutesSettingsState::default();
    assert!(model_routes.routes.is_empty());
    assert!(model_routes.save_error.is_none());

    let rule = QueryClassificationRuleInput::default();
    assert!(rule.pattern.is_empty());
    assert!(rule.category.is_empty());
    assert_eq!(rule.priority_input, "0");
    let classification = QueryClassificationSettingsState::default();
    assert!(!classification.enabled);
    assert!(classification.rules.is_empty());
    assert!(classification.save_error.is_none());

    let heartbeat = HeartbeatSettingsState::default();
    assert!(!heartbeat.enabled);
    assert_eq!(heartbeat.interval_minutes, 30);
    assert!(heartbeat.message_input.is_empty());
    assert!(heartbeat.target_input.is_empty());
    assert!(heartbeat.to_input.is_empty());
    assert!(!heartbeat.show_help_modal);
    assert!(heartbeat.save_error.is_none());
}

#[test]
fn cron_tabs_job_types_and_draft_defaults_match_api_contract() {
    assert_eq!(CronSettingsTab::default(), CronSettingsTab::Jobs);
    assert_eq!(CronSettingsTab::Scheduler, CronSettingsTab::Scheduler);
    assert_eq!(CronAddJobType::default(), CronAddJobType::Agent);
    assert_eq!(CronAddJobType::Shell.as_api_value(), "shell");
    assert_eq!(CronAddJobType::Agent.as_api_value(), "agent");
    assert_eq!(CronAddScheduleKind::default(), CronAddScheduleKind::Cron);
    assert_eq!(CronAddScheduleKind::Cron.as_api_value(), "cron");
    assert_eq!(CronAddScheduleKind::At.as_api_value(), "at");
    assert_eq!(CronAddScheduleKind::Every.as_api_value(), "every");

    let draft = CronJobDraft::default();
    assert!(draft.name.is_empty());
    assert_eq!(draft.job_type, CronAddJobType::Agent);
    assert_eq!(draft.schedule_kind, CronAddScheduleKind::Cron);
    assert_eq!(draft.session_target, "isolated");
    assert_eq!(draft.agent, "main");
    assert_eq!(draft.schedule, "0 9 * * *");
    assert_eq!(draft.frequency, CronFrequency::Daily);
    assert!(draft.at.is_empty());
    assert!(draft.every_ms.is_empty());
    assert!(draft.command.is_empty());
    assert!(draft.command_editor.text().is_empty());
    assert!(draft.prompt.is_empty());
    assert!(draft.prompt_editor.text().is_empty());
    assert!(draft.acp_agent.is_empty());
    assert!(draft.project_path.is_empty());
    assert!(draft.model_provider.is_empty());
    assert!(draft.model.is_empty());
    assert!(!draft.wake);
    assert!(draft.fallbacks.is_empty());
    assert!(!draft.full_access);
    assert!(!draft.task_pool);
    assert!(!draft.delivery_enabled);
    assert!(draft.delivery_channel.is_empty());
    assert!(draft.delivery_to.is_empty());
    assert!(draft.delivery_best_effort);
    assert!(!draft.delete_after_run);

    let state = CronSettingsState::default();
    assert!(state.enabled);
    assert_eq!(state.max_run_history, 50);
    assert_eq!(state.active_tab, CronSettingsTab::Jobs);
    assert!(!state.jobs_loading);
    assert!(state.jobs.is_empty());
    assert!(state.selected_job_ids.is_empty());
    assert!(state.editing_job_id.is_none());
    assert!(state.runs_modal_job_id.is_none());
    assert!(!state.runs_modal_loading);
    assert!(state.runs_modal_error.is_none());
    assert!(state.runs_modal.is_empty());
    assert!(state.runs_modal_editor.text().is_empty());
    assert!(!state.show_help_modal);
    assert!(state.save_error.is_none());
    assert!(state.action_status.is_none());
}

#[test]
fn scheduler_defaults_match_safe_runtime_limits() {
    let scheduler = SchedulerSettingsState::default();
    assert!(scheduler.enabled);
    assert_eq!(scheduler.max_tasks, 64);
    assert_eq!(scheduler.max_concurrent, 4);
    assert!(!scheduler.show_help_modal);
    assert!(scheduler.save_error.is_none());
}
