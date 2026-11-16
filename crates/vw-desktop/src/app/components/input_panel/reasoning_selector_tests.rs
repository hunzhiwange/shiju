//! 验证推理等级选择器的选项映射、当前值与弹窗交互。
//! 未指定等级和不支持推理的模型用于确认选择器的隐藏与回退规则。

use super::reasoning_selector::{
    reasoning_effort_selector,
};
use crate::app::message::chat;
use crate::app::{Message, message};
use vw_api_chat_identity::ReasoningEffortDto;

// 宿主保留：宿主模型能力投影、ACP 确认及聊天更新契约。
#[test]
fn acp_session_options_follow_capabilities_ignore_legacy_reasoning_selection() {
    let (mut app, _task) = crate::app::App::new();
    app.session_state.active_session_id = Some("session-a".to_string());
    app.get_session_runtime_mut("session-a").acp_agent = Some("codex".to_string());
    let _ = crate::app::message::view::update(
        &mut app,
        message::ViewMessage::ToggleReasoningEffortPopover,
    );
    assert!(app.popover_visibility.reasoning_effort);
    let _ = chat::update(
        &mut app,
        message::ChatMessage::ReasoningEffortSelected(Some(ReasoningEffortDto::Xhigh)),
    );
    assert_eq!(app.get_session_runtime("session-a").reasoning_effort, None);

    app.session_state.active_session_id = Some("session-b".to_string());
    app.get_session_runtime_mut("session-b").acp_agent = Some("claude".to_string());
    let _ = chat::update(
        &mut app,
        message::ChatMessage::ReasoningEffortSelected(Some(ReasoningEffortDto::Max)),
    );

    assert_eq!(app.get_session_runtime("session-a").reasoning_effort, None);
    assert_eq!(app.get_session_runtime("session-b").reasoning_effort, None);

    assert!(reasoning_effort_selector(&app, Some("claude"), None).is_none());
    assert!(reasoning_effort_selector(&app, None, None).is_none());
}



// 宿主保留：宿主模型能力投影、ACP 确认及聊天更新契约。
#[test]
fn acp_session_options_follow_capabilities_hide_unconnected_reasoning() {
    let (mut app, _task) = crate::app::App::new();
    app.popover_visibility.reasoning_effort = true;

    assert!(reasoning_effort_selector(&app, Some("codex cli"), None).is_none());
    assert!(reasoning_effort_selector(&app, Some("unknown"), None).is_none());
}

// 宿主保留：宿主模型能力投影、ACP 确认及聊天更新契约。
#[test]
fn model_reasoning_effort_options_follow_configured_capability_and_provider() {
    use super::reasoning_selector::model_reasoning_effort_options;
    use crate::app::state::{ModelSummary, ProviderModelsSummary};
    let (mut app, _) = crate::app::App::new();
    app.model_settings.providers = vec![ProviderModelsSummary {
        id: "provider".into(),
        name: "Provider".into(),
        models: vec![ModelSummary {
            id: "model".into(),
            name: "Model".into(),
            enabled: true,
            toolcall: true,
            attachment: false,
            context_limit: 8192,
            detail: serde_json::json!({"api": {"id": "o3", "adapter": "openai-compatible"}, "capabilities": {"reasoning": true}}),
        }],
    }];
    let expected = [
        None,
        Some(ReasoningEffortDto::Low),
        Some(ReasoningEffortDto::Medium),
        Some(ReasoningEffortDto::High),
    ];
    assert_eq!(model_reasoning_effort_options(&app, "provider/model"), expected);
    assert_eq!(model_reasoning_effort_options(&app, "model"), expected);
    assert_eq!(model_reasoning_effort_options(&app, "other/model"), [None]);
    assert_eq!(model_reasoning_effort_options(&app, "auto"), [None]);
    app.session_state.model = "provider/model".into();
    app.session_state.active_session_id = None;
    app.current_session_runtime_mut().model = "provider/model".into();
    assert!(reasoning_effort_selector(&app, None, None).is_some());
    app.model_settings.providers[0].models[0].detail["capabilities"]["reasoning"] = false.into();
    assert_eq!(model_reasoning_effort_options(&app, "provider/model"), [None]);
    assert!(reasoning_effort_selector(&app, None, None).is_none());
}


// 宿主保留：宿主模型能力投影、ACP 确认及聊天更新契约。
#[test]
fn model_change_clears_stale_reasoning_selection() {
    let (mut app, _) = crate::app::App::new();
    app.current_session_runtime_mut().acp_agent = None;
    app.current_session_runtime_mut().reasoning_effort = Some(ReasoningEffortDto::High);
    app.current_session_runtime_mut().task_mode_reasoning_effort = Some(ReasoningEffortDto::High);
    let _ =
        chat::update(&mut app, message::ChatMessage::ModelSelected("custom/unsupported".into()));
    assert_eq!(app.current_session_runtime().reasoning_effort, None);
    assert_eq!(app.current_session_runtime().task_mode_reasoning_effort, None);
}


// 宿主保留：宿主模型能力投影、ACP 确认及聊天更新契约。
#[test]
fn acp_session_options_follow_capabilities_before_connection() {
    let (app, _) = crate::app::App::new();
    assert!(
        reasoning_effort_selector(&app, Some("codex"), None).is_none(),
        "unconnected ACP sessions must not invent capabilities"
    );
}

// 宿主保留：宿主模型能力投影、ACP 确认及聊天更新契约。
#[test]
fn acp_session_options_follow_capabilities_commit_only_acknowledged_values() {
    use vw_api_acp_model::acp_session::*;
    let (mut app, _) = crate::app::App::new();
    app.session_state.active_session_id = Some("options-a".into());
    let options = AcpSessionOptions {
        acp_model_source:
            vw_api_acp_model::acp_model_source::AcpModelSource::SessionModel,
        model_source_capabilities: Vec::new(),
        selected_model: None,
        verified_cli_release: None,
        verified_platform: None,
        access_status: None,
        connected: true,
        options: vec![AcpSessionOption {
            kind: AcpOptionKind::Mode,
            name: "模式".into(),
            current: "ask".into(),
            choices: vec![
                AcpChoice { id: "ask".into(), name: "询问".into() },
                AcpChoice { id: "code".into(), name: "编码".into() },
            ],
        }],
    };
    let runtime = app.get_session_runtime_mut("options-a");
    runtime.acp_agent = Some("fixture".into());
    runtime.acp_options_state.options = options.clone();
    runtime.full_access_enabled = false;
    let _ = chat::update(
        &mut app,
        message::ChatMessage::AcpOptionSelected(AcpSessionSelection {
            kind: AcpOptionKind::Mode,
            value: "code".into(),
        }),
    );
    assert_eq!(app.get_session_runtime("options-a").acp_options_state.options, options);
    assert!(app.get_session_runtime("options-a").acp_options_state.pending);
    let _ = chat::update(
        &mut app,
        message::ChatMessage::AcpOptionsLoaded {
            session_id: "options-a".into(),
            result: Err("backend rejected selection".into()),
        },
    );
    assert_eq!(app.get_session_runtime("options-a").acp_options_state.options, options);
    assert!(!app.get_session_runtime("options-a").acp_options_state.pending);
    let mut confirmed = options.clone();
    confirmed.options[0].current = "code".into();
    app.session_state.active_session_id = Some("options-b".into());
    let _ = chat::update(
        &mut app,
        message::ChatMessage::AcpOptionsLoaded {
            session_id: "options-a".into(),
            result: Ok(confirmed.clone()),
        },
    );
    assert_eq!(app.get_session_runtime("options-a").acp_options_state.options, confirmed);
    assert!(!app.get_session_runtime("options-a").full_access_enabled);
    assert_eq!(app.get_session_runtime("options-b").acp_options_state.options, AcpSessionOptions::default());
}

// 宿主保留：宿主模型能力投影、ACP 确认及聊天更新契约。
#[test]
fn acp_session_options_follow_capabilities_reject_unadvertised_selection() {
    use vw_api_acp_model::acp_session::*;
    let (mut app, _) = crate::app::App::new();
    app.session_state.active_session_id = Some("options-unknown".into());
    app.current_session_runtime_mut().acp_agent = Some("codex".into());
    let _ = chat::update(
        &mut app,
        message::ChatMessage::AcpOptionSelected(AcpSessionSelection {
            kind: AcpOptionKind::Model,
            value: "invented".into(),
        }),
    );
    assert!(!app.current_session_runtime().acp_options_state.pending);
    assert!(app.current_session_runtime().acp_options_state.options.options.is_empty());
}



// 宿主保留：宿主模型能力投影、ACP 确认及聊天更新契约。
#[test]
fn reasoning_slider_keeps_popover_open_and_resets_to_default() {
    let (mut app, _) = crate::app::App::new();
    app.session_state.active_session_id = Some("reasoning-slider".into());
    app.popover_visibility.reasoning_effort = true;
    for value in [Some(ReasoningEffortDto::Low), Some(ReasoningEffortDto::High), None] {
        let _ = chat::update(&mut app, message::ChatMessage::ReasoningEffortSelected(value));
        assert!(app.popover_visibility.reasoning_effort);
        assert_eq!(app.current_session_runtime().reasoning_effort, value);
        assert_eq!(app.session_state.reasoning_effort, value);
    }
    let _ = crate::app::message::view::update(
        &mut app,
        message::ViewMessage::CloseReasoningEffortPopover,
    );
    assert!(!app.popover_visibility.reasoning_effort);
}
