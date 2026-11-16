// 验证定时任务加载、选择、历史格式化与兼容字段推断。
// 测试夹具围绕目标分支构造可控状态；跨模块行为由对应集成测试负责。

use super::*;
use crate::app::App;
use crate::app::state::{CronSettingsTab};
use gateway_cron::{CronJobDto};

fn app() -> App {
    App::new().0
}

fn job(id: &str) -> CronJobDto {
    CronJobDto {
        timezone: None,
        anchor_date: None,
        weekdays: None,
        hour: None,
        minute: None,
        starts_at: None,
        ends_at: None,

        id: id.to_string(),
        name: Some(format!("job-{id}")),
        job_type: "shell".to_string(),
        schedule_kind: "cron".to_string(),
        expression: "* * * * *".to_string(),
        at: None,
        every_ms: None,
        command: "echo hi".to_string(),
        prompt: None,
        model: Some("openai/gpt-5".to_string()),
        agent: Some("main".to_string()),
        acp_agent: Some("codex".to_string()),
        project_path: Some("/tmp/project".to_string()),
        wake: false,
        fallbacks: vec!["slack".to_string()],
        full_access: false,
        task_pool: false,
        delivery_mode: "channel".to_string(),
        delivery_channel: Some("slack".to_string()),
        delivery_to: Some("#ops".to_string()),
        delivery_best_effort: true,
        delete_after_run: false,
        next_run: "2026-06-11T00:00:00Z".to_string(),
        last_run: None,
        last_status: None,
        last_output: None,
        enabled: true,
    }
}

// 数据边界：覆盖规范化、默认值以及相关状态字段的既定规则。
// 这里固定可观察结果，避免依赖处理过程中的临时实现细节。

// 行为契约：使用代表性场景固定该分支的可观察结果。
// 断言聚焦输入、状态与返回任务，不复述内部控制流程。

// 状态转换：覆盖交互前后的字段同步、选择关系与清理规则。
// 连续操作仍需保持模块不变量，并清除已经失效的瞬态反馈。

// 失败与边界：覆盖非法、缺失、过期或冲突场景的既定处理规则。
// 重点确认现有状态与错误反馈不会被意外的部分更新污染。

#[test]
fn wb_schedule_feedback_unconfigured_channel_is_explicit_error() {
    let mut app = app();
    app.channels_settings.slack = None;
    let draft = &mut app.cron_settings.add_draft;
    draft.prompt = "report".into();
    draft.delivery_enabled = true;
    draft.delivery_channel = "slack".into();
    draft.delivery_to = "target".into();
    assert!(draft.delivery_best_effort);
    let _ = update(&mut app, SettingsMessage::CronAddSubmit);
    assert!(!app.cron_settings.add_draft.submitting);
    assert!(app.cron_settings.save_error.as_deref().unwrap().contains("未配置"));
}

#[test]
fn wb_schedule_feedback_save_error_preserves_draft_and_shows_toast() {
    let mut app = app();
    app.cron_settings.add_draft.prompt = "report".into();
    app.cron_settings.add_draft.submitting = true;
    app.cron_settings.add_draft.submitting = true;
    let _ = update(
        &mut app,
        SettingsMessage::CronEditorSaveCompleted {
            editing: false,
            result: Err("permission denied".into()),
        },
    );
    assert_eq!(app.cron_settings.save_error.as_deref(), Some("permission denied"));
    let toast = app.active_toast.as_ref().expect("error toast");
    assert_eq!(toast.message, "permission denied");
    assert_eq!(toast.kind, crate::app::state::ToastKind::Error);
    assert_eq!(app.cron_settings.add_draft.prompt, "report");
    assert!(!app.cron_settings.add_draft.submitting);
}

fn history_app() -> App {
    let mut app = app();
    app.show_system_settings = true;
    app.system_settings_tab = crate::app::components::system_settings::SystemTab::Cron;
    let _ = update(&mut app, SettingsMessage::CronTabSelected(CronSettingsTab::History));
    app
}

fn history_loaded(
    app: &mut App,
    generation: u64,
    result: Result<(gateway_cron::CronHistoryResponse, Vec<CronJobDto>), String>,
) {
    let _ = super::super::cron_history::update(
        app,
        super::super::cron_history::CronHistoryMessage::Loaded { generation, result },
    );
}

fn history_response() -> gateway_cron::CronHistoryResponse {
    gateway_cron::CronHistoryResponse {
        runs: vec![gateway_cron::CronHistoryRunDto {
            id: Some(1),
            execution_id: None,
            job_id: "a".into(),
            name: Some("备份".into()),
            started_at: "start".into(),
            finished_at: Some("end".into()),
            duration_ms: Some(1),
            status: "ok".into(),
            output: None,
        }],
        finished_limit: 200,
        truncated: false,
    }
}

#[test]
fn cron_history_refresh_error_preserves_rows() {
    use super::super::cron_history::{self, CronHistoryMessage};
    let mut app = history_app();
    let generation = app.cron_settings.history.generation;
    history_loaded(&mut app, generation, Ok((history_response(), vec![job("a")])));
    app.cron_settings.history.search = "备份".into();
    let _ = cron_history::update(&mut app, CronHistoryMessage::Refresh);
    let generation = app.cron_settings.history.generation;
    let _ = cron_history::update(&mut app, CronHistoryMessage::Refresh);
    assert_eq!(generation, app.cron_settings.history.generation);
    history_loaded(&mut app, generation, Err("offline".into()));
    assert_eq!(app.cron_settings.history.runs.len(), 1);
    assert_eq!(app.cron_settings.history.search, "备份");
    assert_eq!(app.cron_settings.history.error.as_deref(), Some("offline"));
    assert!(!app.cron_settings.history.loading);
    assert_eq!(app.cron_settings.jobs[0].id, "a");
}

#[test]
fn cron_history_close_and_workspace_change_invalidate_responses() {
    use super::super::cron_history;
    let mut app = history_app();
    let first = app.cron_settings.history.generation;
    app.show_system_settings = false;
    let _ = app.update(Message::None);
    assert!(app.cron_settings.history.generation > first);
    history_loaded(&mut app, first, Ok((history_response(), vec![job("old")])));
    assert!(app.cron_settings.history.runs.is_empty());
    app.show_system_settings = true;
    let _ = app.update(Message::None);
    let second = app.cron_settings.history.generation;
    app.project_path = Some("/other".into());
    let _ = app.update(Message::None);
    assert!(app.cron_settings.history.generation > second);
    history_loaded(&mut app, second, Ok((history_response(), vec![job("old")])));
    assert!(app.cron_settings.history.runs.is_empty());
    assert!(cron_history::visible(&app));
}

#[test]
fn cron_history_repeated_polling_errors_do_not_repeat_toast() {
    use super::super::cron_history::{self, CronHistoryMessage};
    let mut app = history_app();
    let generation = app.cron_settings.history.generation;
    history_loaded(&mut app, generation, Err("offline".into()));
    let toast_id = app.next_toast_id;
    let _ = cron_history::update(&mut app, CronHistoryMessage::Refresh);
    let generation = app.cron_settings.history.generation;
    history_loaded(&mut app, generation, Err("offline".into()));
    assert_eq!(app.next_toast_id, toast_id);
    let _ = cron_history::update(&mut app, CronHistoryMessage::Tick(generation));
    let current = app.cron_settings.history.generation;
    assert!(current > generation);
    let _ = cron_history::update(&mut app, CronHistoryMessage::Tick(generation));
    assert_eq!(app.cron_settings.history.generation, current);
}

#[test]
fn cron_wecom_form_requires_credentials_and_explicit_target() {
    let mut app = app();
    let mut draft = crate::app::state::CronJobDraft::default();
    draft.delivery_enabled = true;
    draft.delivery_channel = "wecom".into();
    draft.delivery_to = "user".into();
    assert!(validate_delivery(&app, &draft).is_err());
    app.channels_settings.wecom = Some(config_channel_services::WeComConfig {
        bot_id: "bot".into(),
        secret: "secret".into(),
    });
    assert!(validate_delivery(&app, &draft).is_ok());
    draft.delivery_to.clear();
    assert!(validate_delivery(&app, &draft).is_err());
    draft.delivery_enabled = false;
    assert!(validate_delivery(&app, &draft).is_ok());
    draft.delivery_enabled = true;
    draft.delivery_channel = "wecom_webhook".into();
    draft.delivery_to = "default".into();
    // 对话机器人凭据不能替代群通知 Webhook。
    assert!(validate_delivery(&app, &draft).unwrap_err().contains("定时任务通知配置"));
    app.channels_settings.wecom = None;
    app.channels_settings.wecom_webhook = Some(config_channel_services::WeComWebhookConfig {
        webhook_url: "https://qyapi.weixin.qq.com/cgi-bin/webhook/send?key=fixture".into(),
    });
    assert!(validate_delivery(&app, &draft).is_ok());
    draft.delivery_to = "other".into();
    assert!(validate_delivery(&app, &draft).is_err());
}

#[test]
fn cron_wecom_form_configuration_cancel_preserves_both_drafts() {
    use crate::app::message::settings::ChannelsMessage;
    let mut app = app();
    app.cron_settings.add_draft.name = "新增草稿".into();
    app.cron_settings.edit_draft.name = "编辑草稿".into();
    for mode in [
        crate::app::state::WeComDeliveryMode::Manual,
        crate::app::state::WeComDeliveryMode::Webhook,
    ] {
        drop(crate::app::message::settings::update(
            &mut app,
            SettingsMessage::Channels(ChannelsMessage::WeComDeliveryOpened(mode)),
        ));
        drop(crate::app::message::settings::update(
            &mut app,
            SettingsMessage::Channels(ChannelsMessage::WeComDeliveryClosed),
        ));
        assert_eq!(app.cron_settings.add_draft.name, "新增草稿");
        assert_eq!(app.cron_settings.edit_draft.name, "编辑草稿");
        assert!(!app.cron_settings.add_draft.delivery_enabled);
        assert!(!app.cron_settings.edit_draft.delivery_enabled);
    }
}

#[test]
fn cron_history_stale_response_is_ignored() {
    let mut app = history_app();
    let generation = app.cron_settings.history.generation;
    let _ = update(&mut app, SettingsMessage::CronTabSelected(CronSettingsTab::Jobs));
    let _ = update(&mut app, SettingsMessage::CronTabSelected(CronSettingsTab::History));
    let current = app.cron_settings.history.generation;
    assert!(current > generation);
    history_loaded(&mut app, generation, Ok((history_response(), vec![job("old")])));
    assert!(app.cron_settings.history.runs.is_empty());
    assert!(app.cron_settings.history.loading);
    history_loaded(&mut app, current, Ok((history_response(), vec![job("a")])));
    assert_eq!(app.cron_settings.history.runs.len(), 1);
    assert_eq!(app.cron_settings.jobs[0].id, "a");
    // Duplicate completion for the same request cannot overwrite accepted data.
    history_loaded(&mut app, current, Err("late".into()));
    assert!(app.cron_settings.history.error.is_none());
}
