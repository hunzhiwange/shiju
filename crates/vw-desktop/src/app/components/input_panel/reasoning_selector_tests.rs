//! 验证推理等级选择器的选项映射、当前值与弹窗交互。
//! 未指定等级和不支持推理的模型用于确认选择器的隐藏与回退规则。

use super::reasoning_selector::{
    reasoning_effort_items, reasoning_effort_label, reasoning_effort_message,
    reasoning_effort_options, reasoning_effort_selector,
};
use crate::app::message::chat;
use crate::app::{Message, message};
use iced::Length;
use vw_gateway_client::vw_api_types::ReasoningEffortDto;

#[test]
fn chat_reasoning_effort_selector_uses_agent_options_and_current_session() {
    let codex =
        reasoning_effort_items(reasoning_effort_options("codex"), Some(ReasoningEffortDto::High));
    assert_eq!(
        codex.iter().map(|item| item.label).collect::<Vec<_>>(),
        ["默认", "None", "Low", "Medium", "High", "Xhigh"]
    );
    assert_eq!(codex.iter().filter(|item| item.selected).count(), 1);
    assert!(codex[4].selected);

    let claude = reasoning_effort_items(reasoning_effort_options("claude"), None);
    assert_eq!(claude.iter().map(|item| item.label).collect::<Vec<_>>(), ["默认", "High", "Max"]);
    assert!(claude[0].selected);

    let other =
        reasoning_effort_items(reasoning_effort_options("gemini"), Some(ReasoningEffortDto::High));
    assert_eq!(other.iter().map(|item| item.label).collect::<Vec<_>>(), ["默认"]);
    assert_eq!(other.iter().filter(|item| item.selected).count(), 0);

    assert!(matches!(
        reasoning_effort_message(None),
        Message::Chat(message::ChatMessage::ReasoningEffortSelected(None))
    ));
    assert!(matches!(
        reasoning_effort_message(Some(ReasoningEffortDto::None)),
        Message::Chat(message::ChatMessage::ReasoningEffortSelected(Some(
            ReasoningEffortDto::None
        )))
    ));

    let (mut app, _task) = crate::app::App::new();
    app.active_session_id = Some("session-a".to_string());
    app.get_session_runtime_mut("session-a").acp_agent = Some("codex".to_string());
    let _ = crate::app::message::view::update(
        &mut app,
        message::ViewMessage::ToggleReasoningEffortPopover,
    );
    assert!(app.show_reasoning_effort_popover);
    let _ = chat::update(
        &mut app,
        message::ChatMessage::ReasoningEffortSelected(Some(ReasoningEffortDto::Xhigh)),
    );
    assert!(!app.show_reasoning_effort_popover);

    app.active_session_id = Some("session-b".to_string());
    app.get_session_runtime_mut("session-b").acp_agent = Some("claude".to_string());
    let _ = chat::update(
        &mut app,
        message::ChatMessage::ReasoningEffortSelected(Some(ReasoningEffortDto::Max)),
    );

    assert_eq!(
        app.get_session_runtime("session-a").reasoning_effort,
        Some(ReasoningEffortDto::Xhigh)
    );
    assert_eq!(
        app.get_session_runtime("session-b").reasoning_effort,
        Some(ReasoningEffortDto::Max)
    );

    let selector = reasoning_effort_selector(
        &app,
        Some("claude"),
        app.get_session_runtime("session-b").reasoning_effort,
    )
    .expect("ACP sessions show the reasoning selector");
    assert_eq!(selector.as_widget().size().width, Length::Shrink);
    assert!(reasoning_effort_selector(&app, None, None).is_none());
}

#[test]
fn reasoning_effort_options_accept_cli_aliases_case_insensitively() {
    for agent in [" CODEX ", "Codex CLI", "codex-cli"] {
        assert_eq!(
            reasoning_effort_options(agent),
            [
                None,
                Some(ReasoningEffortDto::None),
                Some(ReasoningEffortDto::Low),
                Some(ReasoningEffortDto::Medium),
                Some(ReasoningEffortDto::High),
                Some(ReasoningEffortDto::Xhigh),
            ]
        );
    }

    for agent in [" CLAUDE ", "Claude Code", "claude-code", "ClaudeCode"] {
        assert_eq!(
            reasoning_effort_options(agent),
            [None, Some(ReasoningEffortDto::High), Some(ReasoningEffortDto::Max),]
        );
    }
}

#[test]
fn reasoning_effort_label_displays_every_supported_value() {
    let cases = [
        (None, "默认"),
        (Some(ReasoningEffortDto::None), "None"),
        (Some(ReasoningEffortDto::Minimal), "Minimal"),
        (Some(ReasoningEffortDto::Low), "Low"),
        (Some(ReasoningEffortDto::Medium), "Medium"),
        (Some(ReasoningEffortDto::High), "High"),
        (Some(ReasoningEffortDto::Xhigh), "Xhigh"),
        (Some(ReasoningEffortDto::Max), "Max"),
    ];

    for (effort, expected) in cases {
        assert_eq!(reasoning_effort_label(effort), expected);
    }
}

#[test]
fn reasoning_effort_selector_builds_expanded_codex_and_default_agent_lists() {
    let (mut app, _task) = crate::app::App::new();
    app.show_reasoning_effort_popover = true;

    let codex_selector = reasoning_effort_selector(&app, Some("codex cli"), None)
        .expect("codex ACP sessions show the reasoning selector");
    assert_eq!(codex_selector.as_widget().size().width, Length::Shrink);

    let default_selector =
        reasoning_effort_selector(&app, Some("unknown"), Some(ReasoningEffortDto::Medium))
            .expect("unknown ACP sessions still show the default selector");
    assert_eq!(default_selector.as_widget().size().width, Length::Shrink);
}

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
    app.model = "provider/model".into();
    app.active_session_id = None;
    app.current_session_runtime_mut().model = "provider/model".into();
    assert!(reasoning_effort_selector(&app, None, None).is_some());
    app.model_settings.providers[0].models[0].detail["capabilities"]["reasoning"] = false.into();
    assert_eq!(model_reasoning_effort_options(&app, "provider/model"), [None]);
    assert!(reasoning_effort_selector(&app, None, None).is_none());
}

#[test]
fn custom_provider_options_use_api_model_real_levels() {
    use super::reasoning_selector::model_reasoning_effort_options;
    use crate::app::state::{ModelSummary, ProviderModelsSummary};
    let (mut app, _) = crate::app::App::new();
    for (provider, id, expected) in [
        ("shiju", "deepseek-v4-flash", vec!["默认", "None", "High", "Max"]),
        ("token-plan", "deepseek-v4-flash-0731", vec!["默认", "None", "Low", "High", "Max"]),
        ("token-plan", "glm-5.2", vec!["默认", "None", "High", "Max"]),
        ("token-plan", "qwen3.8-max", vec!["默认", "None", "Low", "Medium", "Xhigh"]),
        ("token-plan", "qwen3.7-plus", vec!["默认"]),
        ("token-plan", "qwen3.6-flash", vec!["默认"]),
        ("shiju", "unknown", vec!["默认"]),
    ] {
        app.model_settings.providers = vec![ProviderModelsSummary {
            id: provider.into(),
            name: provider.into(),
            models: vec![ModelSummary {
                id: "alias".into(),
                name: "Display name".into(),
                enabled: true,
                toolcall: true,
                attachment: false,
                context_limit: 8192,
                detail: serde_json::json!({"api": {"id": id, "adapter": "openai-compatible"}, "capabilities": {"reasoning": true}}),
            }],
        }];
        let options = model_reasoning_effort_options(&app, &format!("{provider}/alias"));
        assert_eq!(
            options.into_iter().map(reasoning_effort_label).collect::<Vec<_>>(),
            expected,
            "{provider}/{id}"
        );
    }
}

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

#[test]
fn bundled_shiju_definition_controls_visible_effort_choices() {
    use super::reasoning_selector::model_reasoning_effort_options;
    use crate::app::state::{ModelSummary, ProviderModelsSummary};
    let data: serde_json::Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../vw-provider-resolver/assets/model/shiju.json"
    )))
    .unwrap();
    let raw: vw_shared::provider::models::Provider =
        serde_json::from_value(data["shiju"].clone()).unwrap();
    let (mut app, _) = crate::app::App::new();
    app.model_settings.providers = vec![ProviderModelsSummary {
        id: "shiju".into(),
        name: raw.name.clone(),
        models: raw
            .models
            .values()
            .map(|raw_model| {
                let model = vw_shared::provider::state::from_models_dev_model(&raw, raw_model);
                ModelSummary {
                    id: model.id.clone(),
                    name: model.name.clone(),
                    enabled: true,
                    toolcall: model.capabilities.toolcall,
                    attachment: model.capabilities.attachment,
                    context_limit: model.limit.context,
                    detail: serde_json::to_value(model).unwrap(),
                }
            })
            .collect(),
    }];
    for id in raw.models.keys() {
        let options = model_reasoning_effort_options(&app, &format!("shiju/{id}"));
        use ReasoningEffortDto::{High, Low, Max, Medium, Minimal, None as Disabled, Xhigh};
        let levels = match id.as_str() {
            "deepseek-v4-flash" | "deepseek-v4-pro-0813" => vec![High, Max],
            "deepseek-v4.1-flash"
            | "deepseek-v4.1-flash-1017"
            | "glm-5.3"
            | "kimi-k3"
            | "kimi/kimi-k2.8-preview" => {
                vec![Low, High, Max]
            }
            "qwen3.8-27b" => vec![Disabled, Minimal, Low, Medium, High],
            "qwen3.8-2.4t-a95b" => vec![Minimal, Low, Medium, High],
            "qwen3.8-max" | "qwen3.8-flash" => {
                vec![Low, Medium, Xhigh]
            }
            "qwen3.8-omni-flash" => vec![Disabled, Low, Medium, Xhigh],
            _ => vec![],
        };
        let expected =
            std::iter::once(None).chain(levels.into_iter().map(Some)).collect::<Vec<_>>();
        assert_eq!(options, expected, "{id}");
    }
}
