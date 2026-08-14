//! Deterministic Auto model selection over already active provider models.
//!
//! This module deliberately has no provider-directory loading or network calls.
//! Callers load active models, then pass them to [`select_auto_model`].

use crate::app::agent::agent::classifier;
use crate::app::agent::config::schema::{ModelRouteConfig, QueryClassificationConfig};
use crate::app::agent::provider::provider;
use std::fmt;

/// Complete, unambiguous model reference.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ModelReference {
    pub provider: String,
    pub model: String,
}

impl ModelReference {
    pub fn new(provider: impl Into<String>, model: impl Into<String>) -> Self {
        Self { provider: provider.into(), model: model.into() }
    }
}

/// One active model available to the Auto selector.
#[derive(Debug, Clone, Copy)]
pub struct ModelCandidate<'a> {
    pub provider_id: &'a str,
    pub model: &'a provider::Model,
}

impl ModelCandidate<'_> {
    fn reference(&self) -> ModelReference {
        ModelReference::new(self.provider_id, &self.model.id)
    }
}

/// Deterministic request profile inferred without an additional model request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelTaskProfile {
    Lightweight,
    General,
    Reasoning,
    ToolOrCode,
    Visual,
    LongContext,
}

impl ModelTaskProfile {
    fn label(self) -> &'static str {
        match self {
            Self::Lightweight => "lightweight",
            Self::General => "general",
            Self::Reasoning => "reasoning",
            Self::ToolOrCode => "tool_or_code",
            Self::Visual => "visual",
            Self::LongContext => "long_context",
        }
    }
}

/// Input for a single Auto selection attempt.
pub struct AutoModelSelectionRequest<'a> {
    /// Message used only for local classification and profile recognition.
    pub message: &'a str,
    /// Minimum required context window. Zero means no additional requirement.
    pub required_context: u64,
    pub needs_tools: bool,
    pub needs_image_input: bool,
    pub needs_attachments: bool,
    /// A configured agent model. It breaks otherwise equal candidates only.
    pub soft_preference: Option<ModelReference>,
    pub query_classification: QueryClassificationConfig,
    pub model_routes: Vec<ModelRouteConfig>,
}

/// Why a model was selected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModelSelectionSource {
    ConfiguredRule { hint: String },
    Auto,
}

/// Safe selection details suitable for events and logs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelSelectionDecision {
    pub model: ModelReference,
    pub source: ModelSelectionSource,
    pub profile: ModelTaskProfile,
    /// Contains only capability and configuration labels, never the request text.
    pub reason: String,
}

/// A selection failure that distinguishes bad configuration from missing capability.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModelSelectionError {
    ConfiguredRouteUnavailable { hint: String, provider: String, model: String },
    NoCompatibleModel { requirements: String },
}

impl fmt::Display for ModelSelectionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ConfiguredRouteUnavailable { hint, provider, model } => write!(
                f,
                "模型路由配置无效：hint:{hint} 指向的 active 模型 {provider}/{model} 不存在"
            ),
            Self::NoCompatibleModel { requirements } => {
                write!(f, "没有满足 Auto 模型需求的 active 候选：{requirements}")
            }
        }
    }
}

impl std::error::Error for ModelSelectionError {}

/// Select one complete provider/model reference from already active candidates.
pub fn select_auto_model(
    request: &AutoModelSelectionRequest<'_>,
    candidates: &[ModelCandidate<'_>],
) -> Result<ModelSelectionDecision, ModelSelectionError> {
    let profile = infer_profile(request);

    if let Some(classification) =
        classifier::classify_with_decision(&request.query_classification, request.message)
    {
        let route = request.model_routes.iter().find(|route| route.hint == classification.hint);
        if let Some(route) = route {
            let reference = ModelReference::new(&route.provider, &route.model);
            if candidates.iter().any(|candidate| candidate.reference() == reference) {
                return Ok(ModelSelectionDecision {
                    model: reference,
                    source: ModelSelectionSource::ConfiguredRule { hint: classification.hint },
                    profile,
                    reason: "source=configured_rule; target=active".to_string(),
                });
            }

            return Err(ModelSelectionError::ConfiguredRouteUnavailable {
                hint: classification.hint,
                provider: route.provider.clone(),
                model: route.model.clone(),
            });
        }
    }

    let requirements = requirement_summary(request);
    let mut compatible = candidates
        .iter()
        .copied()
        .filter(|candidate| is_compatible(request, candidate))
        .map(|candidate| RankedCandidate::new(request, profile, candidate))
        .collect::<Vec<_>>();

    compatible.sort_by(|left, right| right.compare_rank(left));
    let Some(best) = compatible.first() else {
        return Err(ModelSelectionError::NoCompatibleModel { requirements });
    };

    Ok(ModelSelectionDecision {
        model: best.reference.clone(),
        source: ModelSelectionSource::Auto,
        profile,
        reason: format!("source=auto; profile={}; requirements={requirements}", profile.label()),
    })
}

fn infer_profile(request: &AutoModelSelectionRequest<'_>) -> ModelTaskProfile {
    if request.required_context >= 64_000 {
        return ModelTaskProfile::LongContext;
    }
    if request.needs_image_input || request.needs_attachments {
        return ModelTaskProfile::Visual;
    }

    let message = request.message.to_ascii_lowercase();
    if request.needs_tools
        || contains_any(&message, &["code", "implement", "debug", "test", "script", "rust"])
        || request.message.contains("代码")
        || request.message.contains("调试")
    {
        return ModelTaskProfile::ToolOrCode;
    }
    if contains_any(&message, &["analyze", "analysis", "reasoning", "tradeoff", "compare", "prove"])
        || request.message.contains("分析")
        || request.message.contains("推理")
    {
        return ModelTaskProfile::Reasoning;
    }
    if request.message.chars().count() <= 240 {
        ModelTaskProfile::Lightweight
    } else {
        ModelTaskProfile::General
    }
}

fn contains_any(message: &str, terms: &[&str]) -> bool {
    terms.iter().any(|term| message.contains(term))
}

fn is_compatible(request: &AutoModelSelectionRequest<'_>, candidate: &ModelCandidate<'_>) -> bool {
    let capabilities = &candidate.model.capabilities;
    candidate.model.status == "active"
        && !candidate.provider_id.trim().is_empty()
        && !candidate.model.id.trim().is_empty()
        && capabilities.input.text
        && capabilities.output.text
        && (!request.needs_tools || capabilities.toolcall)
        && (!request.needs_image_input || capabilities.input.image)
        && (!request.needs_attachments || capabilities.attachment)
        && (request.required_context == 0
            || candidate.model.limit.context >= request.required_context)
}

fn requirement_summary(request: &AutoModelSelectionRequest<'_>) -> String {
    let mut requirements = vec!["text".to_string()];
    if request.needs_tools {
        requirements.push("tools".to_string());
    }
    if request.needs_image_input {
        requirements.push("image_input".to_string());
    }
    if request.needs_attachments {
        requirements.push("attachments".to_string());
    }
    if request.required_context > 0 {
        requirements.push(format!("context>={}", request.required_context));
    }
    requirements.join(",")
}

#[derive(Debug)]
struct RankedCandidate {
    reference: ModelReference,
    profile_score: u8,
    preferred: bool,
    cost_score: u8,
    release_date: String,
}

impl RankedCandidate {
    fn new(
        request: &AutoModelSelectionRequest<'_>,
        profile: ModelTaskProfile,
        candidate: ModelCandidate<'_>,
    ) -> Self {
        let reference = candidate.reference();
        Self {
            preferred: request.soft_preference.as_ref() == Some(&reference),
            profile_score: profile_score(profile, candidate.model),
            cost_score: cost_score(candidate.model),
            release_date: candidate.model.release_date.clone(),
            reference,
        }
    }

    fn compare_rank(&self, other: &Self) -> std::cmp::Ordering {
        self.profile_score
            .cmp(&other.profile_score)
            .then_with(|| self.preferred.cmp(&other.preferred))
            .then_with(|| self.cost_score.cmp(&other.cost_score))
            .then_with(|| self.release_date.cmp(&other.release_date))
            .then_with(|| other.reference.cmp(&self.reference))
    }
}

fn profile_score(profile: ModelTaskProfile, model: &provider::Model) -> u8 {
    let capabilities = &model.capabilities;
    match profile {
        ModelTaskProfile::Lightweight => {
            if !capabilities.reasoning && !capabilities.toolcall {
                30
            } else {
                10
            }
        }
        ModelTaskProfile::General => 20,
        ModelTaskProfile::Reasoning => u8::from(capabilities.reasoning) * 40,
        ModelTaskProfile::ToolOrCode => {
            let code_name = model.id.to_ascii_lowercase().contains("code")
                || model.name.to_ascii_lowercase().contains("code");
            u8::from(capabilities.toolcall) * 30 + u8::from(code_name) * 10
        }
        ModelTaskProfile::Visual => u8::from(capabilities.input.image) * 40,
        ModelTaskProfile::LongContext => {
            if model.limit.context >= 128_000 {
                40
            } else {
                20
            }
        }
    }
}

fn cost_score(model: &provider::Model) -> u8 {
    let input = model.cost.input;
    let output = model.cost.output;
    if input <= 0.0 || output <= 0.0 {
        // Zero is missing metadata, not a claim that the model is free.
        return 15;
    }

    match input + output {
        total if total <= 1.0 => 30,
        total if total <= 4.0 => 20,
        total if total <= 20.0 => 10,
        _ => 0,
    }
}
