//! This module provides the VolcengineCodingPlan provider, wrapping OpenAI Chat Completions for VolcengineCodingPlan requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    VolcengineCodingPlanProviderSettings,
    VolcengineCodingPlanProviderSettingsBuilder,
    "VolcengineCodingPlan",
    "https://ark.cn-beijing.volces.com/api/coding/v3",
    "ARK_CODING_PLAN_API_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(
    VolcengineCodingPlan,
    VolcengineCodingPlanBuilder,
    VolcengineCodingPlanProviderSettings,
    "volcengine-coding-plan"
);

// Generate the language model implementation
crate::openai_compatible_language_model!(VolcengineCodingPlan);
