// 验证定时任务加载、选择、历史格式化与兼容字段推断。
// 测试夹具围绕目标分支构造可控状态；跨模块行为由对应集成测试负责。

use super::*;
use crate::app::App;
use crate::app::state::{CronSettingsTab};
use gateway_cron::{CronJobDto};

fn app() -> App {
    App::new().0
}

#[test]
fn cron_create_from_conversation_opens_butler_chat_draft() {
    use crate::app::{AppTab, Screen};
    use config_agent::SHIJU_BUTLER_AGENT_KEY;
    use desktop_settings_agents::state::DelegateAgentSettingsEntry;

    let mut app = app();
    let path = "/workspace/project";
    let (tab_id, title) = desktop_workspace::projects::project_tab(path);
    app.navigation_tabs.project_path = Some(path.into());
    app.navigation_tabs.open_tabs.push(AppTab {
        id: tab_id.clone(),
        title,
        screen: Screen::Project,
        project_path: Some(path.into()),
    });
    app.navigation_tabs.screen = Screen::Cron;
    app.navigation_tabs.active_tab_id = Some("cron".into());
    app.agents_settings.entries = vec![DelegateAgentSettingsEntry::from_config(
        SHIJU_BUTLER_AGENT_KEY,
        config_agent::builtin_agent_config(SHIJU_BUTLER_AGENT_KEY),
    )];
    let _ = update(&mut app, SettingsMessage::CronAddFromConversation);

    assert_eq!(app.navigation_tabs.screen, Screen::Project);
    assert_eq!(app.navigation_tabs.active_tab_id.as_deref(), Some(tab_id.as_str()));
    assert!(app.session_state.active_session_id.is_none());
    assert!(!app.cron_settings.show_add_modal);
    let runtime = app.current_session_runtime_ref().expect("chat draft");
    assert_eq!(runtime.agent.as_deref(), Some(SHIJU_BUTLER_AGENT_KEY));
    assert_eq!(runtime.draft.as_ref().expect("draft directory").directory, path);
    let prompt = desktop_i18n::I18n::new(app.general_settings.desktop_locale)
        .__("我想创建一个定时任务。请帮我确认任务内容、执行时间或频率，以及是否需要通知。");
    assert_eq!(runtime.input_editor.text(), prompt);
    assert_eq!(app.session_state.input_editor.text(), prompt);
    assert_eq!(app.session_state.input_text, prompt);
}

#[test]
fn cron_create_from_conversation_uses_workspace_and_appends_to_draft() {
    use crate::app::Screen;
    use config_agent::SHIJU_BUTLER_AGENT_KEY;
    use desktop_settings_agents::state::DelegateAgentSettingsEntry;

    let mut app = app();
    app.navigation_tabs.project_path = None;
    app.navigation_tabs.screen = Screen::Cron;
    app.navigation_tabs.active_tab_id = Some("cron".into());
    app.navigation_panels.show_system_settings = false;
    app.agents_settings.entries = vec![DelegateAgentSettingsEntry::from_config(
        SHIJU_BUTLER_AGENT_KEY,
        config_agent::builtin_agent_config(SHIJU_BUTLER_AGENT_KEY),
    )];
    app.current_session_runtime_mut().input_editor =
        iced::widget::text_editor::Content::with_text("已有草稿");
    app.session_state.files = vec!["example.txt".into()];

    let resolve = update(&mut app, SettingsMessage::CronAddFromConversation);
    assert!(!resolve.is_none());
    assert_eq!(app.navigation_tabs.screen, Screen::Cron);

    let path = "/gateway/workspace/project";
    let gateway_user_generation = app.session_state.gateway_user_generation;
    let navigation_generation = app.session_state.session_view_cache.navigation_generation;
    let _ = update(
        &mut app,
        SettingsMessage::CronWorkspaceResolved {
            gateway_user_generation,
            navigation_generation,
            result: Ok((path.into(), "workspace-id".into())),
        },
    );

    assert_eq!(app.navigation_tabs.screen, Screen::Project);
    assert_eq!(app.navigation_tabs.project_path.as_deref(), Some(path));
    assert_eq!(app.workspace_state.project_id.as_deref(), Some("workspace-id"));
    assert_eq!(app.session_state.files, ["example.txt"]);
    let runtime = app.current_session_runtime_ref().expect("chat draft");
    assert_eq!(runtime.agent.as_deref(), Some(SHIJU_BUTLER_AGENT_KEY));
    assert_eq!(runtime.draft.as_ref().expect("draft directory").directory, path);
    let prompt = desktop_i18n::I18n::new(app.general_settings.desktop_locale)
        .__("我想创建一个定时任务。请帮我确认任务内容、执行时间或频率，以及是否需要通知。");
    let expected = format!("已有草稿\n{prompt}");
    assert_eq!(runtime.input_editor.text(), expected);
    assert_eq!(app.session_state.input_text, expected);
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
    let toast = app.toast_state.active_toast.as_ref().expect("error toast");
    assert_eq!(toast.message, "permission denied");
    assert_eq!(toast.kind, crate::app::state::ToastKind::Error);
    assert_eq!(app.cron_settings.add_draft.prompt, "report");
    assert!(!app.cron_settings.add_draft.submitting);
}

fn history_app() -> App {
    let mut app = app();
    app.navigation_tabs.screen = crate::app::Screen::Cron;
    let _ = update(&mut app, SettingsMessage::CronTabSelected(CronSettingsTab::History));
    app
}







#[test]
fn cron_history_close_and_workspace_change_invalidate_responses() {
    use super::super::cron_history;
    let mut app = history_app();
    let first = app.cron_settings.history.generation;
    app.navigation_tabs.screen = crate::app::Screen::Project;
    let _ = app.update(Message::None);
    assert!(app.cron_settings.history.generation > first);
    history_loaded(&mut app, first, Ok((history_response(), vec![job("old")])));
    assert!(app.cron_settings.history.runs.is_empty());
    app.navigation_tabs.screen = crate::app::Screen::Cron;
    let _ = app.update(Message::None);
    let second = app.cron_settings.history.generation;
    app.navigation_tabs.project_path = Some("/other".into());
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
    let toast_id = app.toast_state.next_toast_id;
    let _ = cron_history::update(&mut app, CronHistoryMessage::Refresh);
    let generation = app.cron_settings.history.generation;
    history_loaded(&mut app, generation, Err("offline".into()));
    assert_eq!(app.toast_state.next_toast_id, toast_id);
    let _ = cron_history::update(&mut app, CronHistoryMessage::Tick(generation));
    let current = app.cron_settings.history.generation;
    assert!(current > generation);
    let _ = cron_history::update(&mut app, CronHistoryMessage::Tick(generation));
    assert_eq!(app.cron_settings.history.generation, current);
}



#[test]
fn cron_wecom_form_configuration_cancel_preserves_both_drafts() {
    use crate::app::message::settings::ChannelsMessage;
    let mut app = app();
    app.cron_settings.add_draft.name = "新增草稿".into();
    app.cron_settings.edit_draft.name = "编辑草稿".into();
    for mode in [
        crate::app::state::WeComDeliveryMode::Manual,
        crate::app::state::WeComDeliveryMode::Qr,
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
