//! 验证自动模型选择的规则优先级、能力过滤与稳定排序。

use super::{
    AutoModelSelectionRequest, ModelCandidate, ModelReference, ModelSelectionError,
    ModelSelectionSource, ModelTaskProfile, select_auto_model,
};
use crate::app::agent::config::schema::{
    ClassificationRule, ModelRouteConfig, QueryClassificationConfig,
};
use crate::app::agent::provider::provider;
use std::collections::HashMap;

fn model(
    provider_id: &str,
    id: &str,
    reasoning: bool,
    toolcall: bool,
    image_input: bool,
    context: u64,
    input_cost: f64,
    release_date: &str,
) -> provider::Model {
    provider::Model {
        id: id.to_string(),
        provider_id: provider_id.to_string(),
        api: provider::ApiInfo {
            id: id.to_string(),
            url: String::new(),
            adapter: "openai-compatible".to_string(),
        },
        name: id.to_string(),
        family: None,
        capabilities: provider::Capabilities {
            temperature: true,
            reasoning,
            attachment: image_input,
            toolcall,
            input: provider::CapabilityIO {
                text: true,
                audio: false,
                image: image_input,
                video: false,
                pdf: false,
            },
            output: provider::CapabilityIO {
                text: true,
                audio: false,
                image: false,
                video: false,
                pdf: false,
            },
            interleaved: provider::InterleavedCapability::Bool(false),
        },
        cost: provider::ModelCost {
            input: input_cost,
            output: input_cost,
            cache: provider::ModelCostCache { read: input_cost, write: input_cost },
            experimental_over_200k: None,
        },
        limit: provider::ModelLimit { context, input: None, output: 4096 },
        status: "active".to_string(),
        options: HashMap::new(),
        headers: HashMap::new(),
        release_date: release_date.to_string(),
        variants: HashMap::new(),
    }
}

fn request<'a>(message: &'a str) -> AutoModelSelectionRequest<'a> {
    AutoModelSelectionRequest {
        message,
        required_context: 0,
        needs_tools: false,
        needs_image_input: false,
        needs_attachments: false,
        soft_preference: None,
        query_classification: QueryClassificationConfig::default(),
        model_routes: Vec::new(),
    }
}

fn candidates<'a>(models: &'a [provider::Model]) -> Vec<ModelCandidate<'a>> {
    models.iter().map(|model| ModelCandidate { provider_id: &model.provider_id, model }).collect()
}

#[test]
fn auto_selector_prioritizes_configured_rule_route() {
    let models = vec![
        model("slow", "reasoner", true, false, false, 32_000, 8.0, "2025-01-01"),
        model("fast", "summary", false, false, false, 32_000, 0.2, "2025-01-01"),
    ];
    let classification = QueryClassificationConfig {
        enabled: true,
        rules: vec![ClassificationRule {
            hint: "summary".to_string(),
            keywords: vec!["summarize".to_string()],
            patterns: Vec::new(),
            min_length: None,
            max_length: None,
            priority: 1,
        }],
    };
    let routes = vec![ModelRouteConfig {
        hint: "summary".to_string(),
        provider: "fast".to_string(),
        model: "summary".to_string(),
        max_tokens: None,
        api_key: None,
    }];
    let mut selection_request = request("Please summarize this report");
    selection_request.query_classification = classification;
    selection_request.model_routes = routes;

    let decision = select_auto_model(&selection_request, &candidates(&models)).unwrap();

    assert_eq!(decision.model, ModelReference::new("fast", "summary"));
    assert_eq!(decision.source, ModelSelectionSource::ConfiguredRule { hint: "summary".into() });
}

#[test]
fn auto_selector_selects_lightweight_model_for_short_simple_request() {
    let models = vec![
        model("provider", "premium", true, true, false, 128_000, 20.0, "2025-01-01"),
        model("provider", "fast", false, false, false, 32_000, 0.2, "2025-01-01"),
    ];

    let decision = select_auto_model(&request("Summarize this."), &candidates(&models)).unwrap();

    assert_eq!(decision.profile, ModelTaskProfile::Lightweight);
    assert_eq!(decision.model, ModelReference::new("provider", "fast"));
}

#[test]
fn auto_selector_selects_reasoning_model_for_analysis_request() {
    let models = vec![
        model("provider", "general", false, false, false, 32_000, 1.0, "2025-01-01"),
        model("provider", "reasoner", true, false, false, 32_000, 5.0, "2025-01-01"),
    ];

    let decision = select_auto_model(
        &request("Analyze the tradeoffs carefully and explain your reasoning."),
        &candidates(&models),
    )
    .unwrap();

    assert_eq!(decision.profile, ModelTaskProfile::Reasoning);
    assert_eq!(decision.model, ModelReference::new("provider", "reasoner"));
}

#[test]
fn auto_selector_selects_tool_capable_model_for_code_request() {
    let models = vec![
        model("provider", "chat", true, false, false, 32_000, 1.0, "2025-01-01"),
        model("provider", "coder", false, true, false, 32_000, 4.0, "2025-01-01"),
    ];
    let mut selection_request = request("Implement this Rust function and run the tests.");
    selection_request.needs_tools = true;

    let decision = select_auto_model(&selection_request, &candidates(&models)).unwrap();

    assert_eq!(decision.profile, ModelTaskProfile::ToolOrCode);
    assert_eq!(decision.model, ModelReference::new("provider", "coder"));
}

#[test]
fn auto_selector_selects_visual_model_and_filters_non_visual_candidates() {
    let models = vec![
        model("provider", "chat", true, true, false, 32_000, 1.0, "2025-01-01"),
        model("provider", "vision", false, false, true, 32_000, 5.0, "2025-01-01"),
    ];
    let mut selection_request = request("Describe the attached image.");
    selection_request.needs_image_input = true;

    let decision = select_auto_model(&selection_request, &candidates(&models)).unwrap();

    assert_eq!(decision.profile, ModelTaskProfile::Visual);
    assert_eq!(decision.model, ModelReference::new("provider", "vision"));
}

#[test]
fn auto_selector_selects_long_context_model_and_filters_short_context_candidates() {
    let models = vec![
        model("provider", "short", true, true, false, 16_000, 0.2, "2025-01-01"),
        model("provider", "long", false, false, false, 256_000, 8.0, "2025-01-01"),
    ];
    let mut selection_request = request("Review this long document.");
    selection_request.required_context = 100_000;

    let decision = select_auto_model(&selection_request, &candidates(&models)).unwrap();

    assert_eq!(decision.profile, ModelTaskProfile::LongContext);
    assert_eq!(decision.model, ModelReference::new("provider", "long"));
}

#[test]
fn auto_selector_treats_unknown_cost_as_neutral_not_free() {
    let models = vec![
        model("provider", "known-cheap", false, false, false, 32_000, 0.2, "2025-01-01"),
        model("provider", "unknown-cost", false, false, false, 32_000, 0.0, "2025-01-01"),
    ];

    let decision = select_auto_model(&request("Summarize this."), &candidates(&models)).unwrap();

    assert_eq!(decision.model, ModelReference::new("provider", "known-cheap"));
}

#[test]
fn auto_selector_uses_soft_preference_only_as_tiebreaker() {
    let models = vec![
        model("provider", "preferred-chat", false, false, false, 32_000, 0.2, "2025-01-01"),
        model("provider", "reasoner", true, false, false, 32_000, 5.0, "2025-01-01"),
    ];
    let mut selection_request = request("Analyze the risks and explain the reasoning.");
    selection_request.soft_preference = Some(ModelReference::new("provider", "preferred-chat"));

    let decision = select_auto_model(&selection_request, &candidates(&models)).unwrap();

    assert_eq!(decision.model, ModelReference::new("provider", "reasoner"));
}

#[test]
fn auto_selector_returns_configuration_error_for_missing_rule_target() {
    let models =
        vec![model("provider", "available", false, false, false, 32_000, 0.2, "2025-01-01")];
    let classification = QueryClassificationConfig {
        enabled: true,
        rules: vec![ClassificationRule {
            hint: "fast".to_string(),
            keywords: vec!["quick".to_string()],
            patterns: Vec::new(),
            min_length: None,
            max_length: None,
            priority: 1,
        }],
    };
    let routes = vec![ModelRouteConfig {
        hint: "fast".to_string(),
        provider: "missing".to_string(),
        model: "model".to_string(),
        max_tokens: None,
        api_key: None,
    }];
    let mut selection_request = request("A quick answer please.");
    selection_request.query_classification = classification;
    selection_request.model_routes = routes;

    let error = select_auto_model(&selection_request, &candidates(&models)).unwrap_err();

    assert!(matches!(error, ModelSelectionError::ConfiguredRouteUnavailable { .. }));
}

#[test]
fn auto_selector_returns_requirement_summary_when_no_candidate_is_compatible() {
    let models = vec![model("provider", "chat", false, false, false, 16_000, 0.2, "2025-01-01")];
    let mut selection_request = request("Use tools to implement this.");
    selection_request.needs_tools = true;
    selection_request.required_context = 32_000;

    let error = select_auto_model(&selection_request, &candidates(&models)).unwrap_err();

    assert!(
        matches!(error, ModelSelectionError::NoCompatibleModel { requirements } if requirements.contains("tools") && requirements.contains("context>=32000"))
    );
}

#[test]
fn auto_selector_uses_stable_full_reference_order_after_equal_scores() {
    let models = vec![
        model("zeta", "same", false, false, false, 32_000, 1.0, "2025-01-01"),
        model("alpha", "same", false, false, false, 32_000, 1.0, "2025-01-01"),
    ];

    let first =
        select_auto_model(&request("Explain this topic in detail."), &candidates(&models)).unwrap();
    let mut reversed = models.clone();
    reversed.reverse();
    let second =
        select_auto_model(&request("Explain this topic in detail."), &candidates(&reversed))
            .unwrap();

    assert_eq!(first.model, ModelReference::new("alpha", "same"));
    assert_eq!(first, second);
}
