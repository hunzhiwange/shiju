//! This module provides the AlibabaTokenPlan provider, wrapping OpenAI Chat Completions for AlibabaTokenPlan requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    AlibabaTokenPlanProviderSettings,
    AlibabaTokenPlanProviderSettingsBuilder,
    "AlibabaTokenPlan",
    "https://token-plan.ap-southeast-1.maas.aliyuncs.com/compatible-mode/v1",
    "ALIBABA_TOKEN_PLAN_API_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(
    AlibabaTokenPlan,
    AlibabaTokenPlanBuilder,
    AlibabaTokenPlanProviderSettings,
    "alibaba-token-plan"
);

// Generate the language model implementation
crate::openai_compatible_language_model!(AlibabaTokenPlan);
