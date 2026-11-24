//! This module provides the ScnetTokenPlan provider, wrapping OpenAI Chat Completions for ScnetTokenPlan requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    ScnetTokenPlanProviderSettings,
    ScnetTokenPlanProviderSettingsBuilder,
    "ScnetTokenPlan",
    "https://api.scnet.cn/api/llm/v1",
    "SCNET_API_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(
    ScnetTokenPlan,
    ScnetTokenPlanBuilder,
    ScnetTokenPlanProviderSettings,
    "scnet-token-plan"
);

// Generate the language model implementation
crate::openai_compatible_language_model!(ScnetTokenPlan);
