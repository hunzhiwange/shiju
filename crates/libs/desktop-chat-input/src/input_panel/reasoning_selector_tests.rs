use super::reasoning_selector::*;
use super::reasoning_message::Message;
use vw_api_chat_identity::ReasoningEffortDto;
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
fn reasoning_items_preserve_selection_and_messages() {
    let codex =
        reasoning_effort_items(reasoning_effort_options("codex"), Some(ReasoningEffortDto::High));
    assert_eq!(
        codex.iter().map(|item| reasoning_effort_label(item.value)).collect::<Vec<_>>(),
        ["默认", "None", "Low", "Medium", "High", "Xhigh"]
    );
    assert_eq!(codex.iter().filter(|item| item.selected).count(), 1);
    assert!(codex[4].selected);

    let claude = reasoning_effort_items(reasoning_effort_options("claude"), None);
    assert_eq!(
        claude.iter().map(|item| reasoning_effort_label(item.value)).collect::<Vec<_>>(),
        ["默认", "High", "Max"]
    );
    assert!(claude[0].selected);

    let other =
        reasoning_effort_items(reasoning_effort_options("gemini"), Some(ReasoningEffortDto::High));
    assert_eq!(
        other.iter().map(|item| reasoning_effort_label(item.value)).collect::<Vec<_>>(),
        ["默认"]
    );
    assert_eq!(other.iter().filter(|item| item.selected).count(), 0);

    assert!(matches!(
        reasoning_effort_message(None),
        Message::Selected(None)
    ));
    assert!(matches!(
        reasoning_effort_message(Some(ReasoningEffortDto::None)),
        Message::Selected(Some(
            ReasoningEffortDto::None
        ))
    ));

}

#[test]
fn custom_provider_options_use_api_model_real_levels() {

    use desktop_settings_providers::state::{ModelSummary, ProviderModelsSummary};
    for (provider, id, expected) in [
        ("shiju", "deepseek-v4-flash", vec!["默认", "None", "High", "Max"]),
        ("token-plan", "deepseek-v4-flash-0731", vec!["默认", "None", "Low", "High", "Max"]),
        ("token-plan", "glm-5.2", vec!["默认", "None", "High", "Max"]),
        ("token-plan", "qwen3.8-max", vec!["默认", "None", "Low", "Medium", "Xhigh"]),
        ("token-plan", "qwen3.7-plus", vec!["默认"]),
        ("token-plan", "qwen3.6-flash", vec!["默认"]),
        ("shiju", "unknown", vec!["默认"]),
    ] {
        let providers = vec![ProviderModelsSummary {
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
        let options = model_reasoning_effort_options(&providers, &format!("{provider}/alias"));
        assert_eq!(
            options.into_iter().map(reasoning_effort_label).collect::<Vec<_>>(),
            expected,
            "{provider}/{id}"
        );
    }
}
#[test]
fn bundled_shiju_definition_controls_visible_effort_choices() {

    use desktop_settings_providers::state::{ModelSummary, ProviderModelsSummary};
    let data: serde_json::Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../provider-model-catalog/assets/model/shiju.json"
    )))
    .unwrap();
    let raw: model_catalog_types::Provider =
        serde_json::from_value(data["shiju"].clone()).unwrap();
    let providers = vec![ProviderModelsSummary {
        id: "shiju".into(),
        name: raw.name.clone(),
        models: raw
            .models
            .values()
            .map(|raw_model| {
                let model = model_state::from_models_dev_model(&raw, raw_model);
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
        let options = model_reasoning_effort_options(&providers, &format!("shiju/{id}"));
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
