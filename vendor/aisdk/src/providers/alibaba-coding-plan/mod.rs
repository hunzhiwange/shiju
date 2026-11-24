//! This module provides the AlibabaCodingPlan provider, wrapping OpenAI Chat Completions for AlibabaCodingPlan requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    AlibabaCodingPlanProviderSettings,
    AlibabaCodingPlanProviderSettingsBuilder,
    "AlibabaCodingPlan",
    "https://coding-intl.dashscope.aliyuncs.com/v1",
    "ALIBABA_CODING_PLAN_API_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(
    AlibabaCodingPlan,
    AlibabaCodingPlanBuilder,
    AlibabaCodingPlanProviderSettings,
    "alibaba-coding-plan"
);

// Generate the language model implementation
crate::openai_compatible_language_model!(AlibabaCodingPlan);
