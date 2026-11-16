// 宿主契约：全局偏好/配置回写及真实 chat::update 的 ACP 回执隔离；纯运行态测试归 desktop-session::host_module_1_tests::runtime_tests。
//! 会话运行时测试，覆盖工具清单归一化、代理限制交集和请求能力选择。
//! 空限制、静态回退与内置工具锁定均应维持默认拒绝的安全边界。

use super::*;
use crate::app::state::{AcpHistoryReplayMode, SessionRuntimeState};

fn test_app() -> App {
    App::new().0
}

#[test]
fn sync_active_session_preferences_preserves_global_model_and_syncs_other_values() {
    let mut app = test_app();
    app.active_session_id = None;
    app.model = "unchanged".to_string();
    app.auto_model = false;
    app.sync_active_session_preferences();
    assert_eq!(app.model, "unchanged");
    assert!(!app.auto_model);

    let mut runtime = SessionRuntimeState::with_defaults("session-model".to_string(), true);
    runtime.reasoning_effort = Some(vw_api_chat_identity::ReasoningEffortDto::High);
    runtime.acp_agent = Some("session-acp".to_string());
    runtime.acp_history_mode = AcpHistoryReplayMode::Recent;
    runtime.acp_recent_count = 12;
    app.session_runtime_states.insert("active".to_string(), runtime);
    app.active_session_id = Some("active".to_string());

    app.sync_active_session_preferences();

    assert_eq!(app.model, "unchanged");
    assert!(!app.auto_model);
    assert_eq!(app.reasoning_effort, Some(vw_api_chat_identity::ReasoningEffortDto::High));
    assert_eq!(app.acp_agent.as_deref(), Some("session-acp"));
    assert_eq!(app.acp_history_mode, AcpHistoryReplayMode::Recent);
    assert_eq!(app.acp_recent_count, 12);
}

#[test]
fn historical_session_restores_persisted_reasoning_effort_without_affecting_model_restore() {
    use vw_api_chat_identity::ReasoningEffortDto;

    let mut app = test_app();
    let cfg = serde_json::json!({
        "session_reasoning_efforts": {
            "history": "high",
            "default": null,
            "disabled": "none",
            "unknown": "invalid"
        }
    });
    app.session_reasoning_efforts = crate::app::config::session_reasoning_efforts_from_config(&cfg);
    app.session_runtime_states.clear();
    assert!(!app.session_reasoning_efforts.contains_key("unknown"));
    assert_eq!(app.session_reasoning_efforts.get("default"), Some(&None));

    app.prepare_historical_session_model("history");
    app.active_session_id = Some("history".to_string());
    app.sync_active_session_preferences();
    assert_eq!(app.reasoning_effort, Some(ReasoningEffortDto::High));
    assert!(app.restore_session_model_from_history("history", "provider/history-model"));
    assert_eq!(app.current_session_runtime().reasoning_effort, Some(ReasoningEffortDto::High));
    assert_eq!(app.get_session_runtime("default").reasoning_effort, None);
    assert_eq!(
        app.get_session_runtime("disabled").reasoning_effort,
        Some(ReasoningEffortDto::None)
    );
    assert_eq!(app.get_session_runtime("new").reasoning_effort, None);

    // 模拟退出后从序列化配置重新进入历史会话。
    let saved = serde_json::json!({"session_reasoning_efforts": app.session_reasoning_efforts});
    app.session_runtime_states.clear();
    app.session_reasoning_efforts =
        crate::app::config::session_reasoning_efforts_from_config(&saved);
    assert_eq!(
        app.get_session_runtime_mut("history").reasoning_effort,
        Some(ReasoningEffortDto::High)
    );
}

#[test]
fn acp_model_source_ui_selection_waits_for_backend_confirmation() {
    use crate::app::message::{ChatMessage, chat};
    use vw_api_acp_model::acp_model_source::AcpModelSource;
    let (mut app, _) = crate::app::App::new();
    app.current_session_runtime_mut().acp_agent = Some("claude".into());
    let _ = chat::update(&mut app, ChatMessage::AcpModelSourceSelected(AcpModelSource::CliLocal));
    assert_eq!(app.current_session_runtime().acp_model_source, AcpModelSource::SessionModel);
    assert!(app.current_session_runtime().acp_options_state.source_pending.is_some());
}

#[test]
fn acp_model_source_ui_round_trip_preserves_model_and_open_popover() {
    use crate::app::message::{ChatMessage, chat};
    use vw_api_acp_model::{acp_model_source::AcpModelSource, acp_session::AcpSessionOptions};
    let (mut app, _) = crate::app::App::new();
    app.popover_visibility.model = true;
    let runtime = app.current_session_runtime_mut();
    runtime.acp_agent = Some("claude".into());
    runtime.model = "provider/selected-model".into();
    runtime.auto_model = false;
    assert_eq!(runtime.acp_model_source, AcpModelSource::SessionModel);
    for source in [AcpModelSource::CliLocal, AcpModelSource::SessionModel] {
        let _ = chat::update(&mut app, ChatMessage::AcpModelSourceSelected(source));
        let runtime = app.current_session_runtime();
        let update = runtime.acp_options_state.source_pending.clone().unwrap();
        let request_id = runtime.acp_options_state.source_request_id.unwrap();
        let _ = chat::update(
            &mut app,
            ChatMessage::AcpModelSourceSaved {
                session_id: "__empty__".into(),
                request_id,
                project_path: None,
                update: update.clone(),
                result: Ok(AcpSessionOptions {
                    acp_model_source: source,
                    selected_model: Some(update.selection),
                    ..Default::default()
                }),
            },
        );
        assert_eq!(app.current_session_runtime().acp_model_source, source);
        assert_eq!(app.current_session_runtime().model, "provider/selected-model");
        assert!(app.popover_visibility.model);
    }
}

#[test]
fn acp_model_source_ui_unsupported_cli_uses_native_and_rejects_client_switch() {
    use crate::app::message::{ChatMessage, chat};
    use vw_api_acp_model::{
        acp_model_source::{
            AcpModelSource, AcpModelSourceCapability, AcpModelSourceRejectionReason,
        },
        acp_session::AcpSessionOptions,
    };
    let (mut app, _) = crate::app::App::new();
    app.current_session_runtime_mut().acp_agent = Some("cursor".into());
    app.current_session_runtime_mut().acp_options_state.source_request_id = Some(0);
    let _ = chat::update(
        &mut app,
        ChatMessage::AcpSourceLoaded {
            session_id: "__empty__".into(),
            request_id: 0,
            agent: "cursor".into(),
            result: Ok(AcpSessionOptions {
                acp_model_source: AcpModelSource::CliLocal,
                model_source_capabilities: vec![
                    AcpModelSourceCapability::Unsupported {
                        source: AcpModelSource::SessionModel,
                        reason: AcpModelSourceRejectionReason::UnsupportedCli,
                    },
                    AcpModelSourceCapability::Supported { source: AcpModelSource::CliLocal },
                ],
                ..Default::default()
            }),
        },
    );
    let _ =
        chat::update(&mut app, ChatMessage::AcpModelSourceSelected(AcpModelSource::SessionModel));
    assert_eq!(app.current_session_runtime().acp_model_source, AcpModelSource::CliLocal);
    assert!(app.current_session_runtime().acp_options_state.source_pending.is_none());
}

#[test]
fn acp_model_source_ui_late_options_do_not_recreate_deleted_session() {
    use crate::app::message::{ChatMessage, chat};
    let (mut app, _) = crate::app::App::new();
    let _ = chat::update(
        &mut app,
        ChatMessage::AcpOptionsLoaded {
            session_id: "deleted-source-session".into(),
            result: Ok(Default::default()),
        },
    );
    assert!(!app.session_runtime_states.contains_key("deleted-source-session"));
}

fn source_ui_update() -> vw_api_acp_model::acp_model_source::AcpModelSourceUpdate {
    use vw_api_acp_model::acp_model_source::*;
    AcpModelSourceUpdate {
        agent: "claude".into(),
        selection: AcpSessionModelSelection {
            source: AcpModelSource::CliLocal,
            provider_id: None,
            model: None,
        },
    }
}

#[test]
fn acp_model_source_ui_failure_preserves_confirmed_model_and_other_session() {
    use crate::app::message::{ChatMessage, chat};
    use vw_api_acp_model::acp_model_source::AcpModelSource;
    let (mut app, _) = crate::app::App::new();
    app.active_session_id = Some("source-a".into());
    let runtime = app.current_session_runtime_mut();
    runtime.acp_agent = Some("claude".into());
    runtime.model = "provider/nested/model".into();
    runtime.acp_options_state.source_pending = Some(source_ui_update());
    runtime.acp_options_state.source_request_id = Some(12);
    app.active_session_id = Some("source-b".into());
    let _ = chat::update(
        &mut app,
        ChatMessage::AcpModelSourceSaved {
            session_id: "source-a".into(),
            request_id: 12,
            project_path: None,
            update: source_ui_update(),
            result: Err("credential revoked".into()),
        },
    );
    let runtime = app.get_session_runtime("source-a");
    assert_eq!(runtime.acp_model_source, AcpModelSource::SessionModel);
    assert_eq!(runtime.model, "provider/nested/model");
    assert!(runtime.acp_options_state.source_pending.is_none());
    assert!(app.current_session_runtime().acp_options_state.source_pending.is_none());
    assert_eq!(app.active_session_id.as_deref(), Some("source-b"));
}

#[test]
fn acp_model_source_ui_confirmation_updates_only_originating_session() {
    use crate::app::message::{ChatMessage, chat};
    use vw_api_acp_model::acp_model_source::AcpModelSource;
    let (mut app, _) = crate::app::App::new();
    app.active_session_id = Some("source-a".into());
    let runtime = app.current_session_runtime_mut();
    runtime.acp_agent = Some("claude".into());
    runtime.acp_options_state.source_pending = Some(source_ui_update());
    runtime.acp_options_state.source_request_id = Some(12);
    app.active_session_id = Some("source-b".into());
    let options = vw_api_acp_model::acp_session::AcpSessionOptions {
        acp_model_source: AcpModelSource::CliLocal,
        selected_model: Some(source_ui_update().selection),
        ..Default::default()
    };
    let _ = chat::update(
        &mut app,
        ChatMessage::AcpModelSourceSaved {
            session_id: "source-a".into(),
            request_id: 11,
            project_path: None,
            update: source_ui_update(),
            result: Ok(options.clone()),
        },
    );
    assert_eq!(app.get_session_runtime("source-a").acp_model_source, AcpModelSource::SessionModel);
    let _ = chat::update(
        &mut app,
        ChatMessage::AcpModelSourceSaved {
            session_id: "source-a".into(),
            request_id: 12,
            project_path: None,
            update: source_ui_update(),
            result: Ok(options),
        },
    );
    assert_eq!(app.get_session_runtime("source-a").acp_model_source, AcpModelSource::CliLocal);
    assert_eq!(app.current_session_runtime().acp_model_source, AcpModelSource::SessionModel);
    assert!(app.get_session_runtime("source-a").acp_rebuild_required);
}

#[test]
fn acp_model_source_ui_busy_session_rejects_model_and_source_changes() {
    use crate::app::message::{ChatMessage, chat};
    use vw_api_acp_model::acp_model_source::AcpModelSource;
    let (mut app, _) = crate::app::App::new();
    let runtime = app.current_session_runtime_mut();
    runtime.acp_agent = Some("claude".into());
    runtime.model = "provider/original".into();
    runtime.request_lifecycle.is_requesting = true;
    let _ = chat::update(&mut app, ChatMessage::ModelSelected("provider/other".into()));
    let _ = chat::update(&mut app, ChatMessage::AcpModelSourceSelected(AcpModelSource::CliLocal));
    assert_eq!(app.current_session_runtime().model, "provider/original");
    assert_eq!(app.current_session_runtime().acp_model_source, AcpModelSource::SessionModel);
    assert!(app.current_session_runtime().acp_options_state.source_pending.is_none());
}

#[test]
fn acp_model_source_ui_manual_model_draft_does_not_change_confirmed_model() {
    use crate::app::message::{ChatMessage, chat};
    let (mut app, _) = crate::app::App::new();
    let runtime = app.current_session_runtime_mut();
    runtime.acp_agent = Some("claude".into());
    runtime.model = "provider/original".into();
    let _ = chat::update(&mut app, ChatMessage::AcpModelInputDraftChanged("provider/oth".into()));
    let runtime = app.current_session_runtime();
    assert_eq!(runtime.model, "provider/original");
    assert_eq!(runtime.acp_options_state.model_input_draft.as_deref(), Some("provider/oth"));
    assert!(runtime.acp_options_state.source_pending.is_none());
}

#[test]
fn acp_model_source_ui_new_project_ignores_previous_project_confirmation() {
    use crate::app::message::{ChatMessage, chat};
    use vw_api_acp_model::{acp_model_source::AcpModelSource, acp_session::AcpSessionOptions};
    let (mut app, _) = crate::app::App::new();
    app.navigation_tabs.project_path = Some("/new-project".into());
    let runtime = app.current_session_runtime_mut();
    runtime.acp_agent = Some("claude".into());
    runtime.acp_options_state.source_pending = Some(source_ui_update());
    runtime.acp_options_state.source_request_id = Some(12);
    let _ = chat::update(
        &mut app,
        ChatMessage::AcpModelSourceSaved {
            session_id: "__empty__".into(),
            request_id: 12,
            project_path: Some("/old-project".into()),
            update: source_ui_update(),
            result: Ok(AcpSessionOptions {
                acp_model_source: AcpModelSource::CliLocal,
                ..Default::default()
            }),
        },
    );
    assert_eq!(app.current_session_runtime().acp_model_source, AcpModelSource::SessionModel);
    assert!(app.current_session_runtime().acp_options_state.source_pending.is_none());
    assert!(app.active_session_id.is_none());
}

#[test]
fn historical_session_model_restore_uses_latest_recorded_model() {
    let mut app = test_app();
    app.session_runtime_states.clear();
    app.model = "previous-session-model".to_string();
    app.auto_model = false;

    app.prepare_historical_session_model("history");
    app.active_session_id = Some("history".to_string());
    app.sync_active_session_preferences();

    assert_eq!(app.model, "previous-session-model");
    assert!(!app.auto_model);
    assert_eq!(app.get_session_runtime("history").model, "previous-session-model");

    let steps = vec![
        crate::app::models::ChatSessionStep {
            index: 1,
            turn_id: None,
            started_ms: 1,
            finished_ms: Some(2),
            start_snapshot_path: None,
            finish_snapshot_path: None,
            usage: crate::app::models::TokenUsage::default(),
            cost_usd: None,
            finish_reason: None,
            model: Some("provider/old-model".to_string()),
            tool_timings: Vec::new(),
        },
        crate::app::models::ChatSessionStep {
            index: 2,
            turn_id: None,
            started_ms: 3,
            finished_ms: Some(4),
            start_snapshot_path: None,
            finish_snapshot_path: None,
            usage: crate::app::models::TokenUsage::default(),
            cost_usd: None,
            finish_reason: None,
            model: Some(" provider/history-model ".to_string()),
            tool_timings: Vec::new(),
        },
    ];

    assert!(app.restore_session_model_from_steps("history", &steps));
    app.sync_active_session_preferences();

    assert_eq!(app.model, "previous-session-model");
    assert!(!app.auto_model);
    let runtime = app.session_runtime_states.get("history").expect("history runtime");
    assert_eq!(runtime.model, "provider/history-model");
    assert_eq!(runtime.task_mode_model, "provider/history-model");
    assert!(!runtime.restore_model_from_history);
}
