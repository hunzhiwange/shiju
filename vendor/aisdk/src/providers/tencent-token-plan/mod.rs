//! This module provides the TencentTokenPlan provider, wrapping OpenAI Chat Completions for TencentTokenPlan requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    TencentTokenPlanProviderSettings,
    TencentTokenPlanProviderSettingsBuilder,
    "TencentTokenPlan",
    "https://api.lkeap.cloud.tencent.com/plan/v3",
    "TENCENT_TOKEN_PLAN_API_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(
    TencentTokenPlan,
    TencentTokenPlanBuilder,
    TencentTokenPlanProviderSettings,
    "tencent-token-plan"
);

// Generate the language model implementation
crate::openai_compatible_language_model!(TencentTokenPlan);
