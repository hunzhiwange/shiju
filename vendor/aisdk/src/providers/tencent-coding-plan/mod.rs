//! This module provides the TencentCodingPlan provider, wrapping OpenAI Chat Completions for TencentCodingPlan requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    TencentCodingPlanProviderSettings,
    TencentCodingPlanProviderSettingsBuilder,
    "TencentCodingPlan",
    "https://api.lkeap.cloud.tencent.com/coding/v3",
    "TENCENT_CODING_PLAN_API_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(
    TencentCodingPlan,
    TencentCodingPlanBuilder,
    TencentCodingPlanProviderSettings,
    "tencent-coding-plan"
);

// Generate the language model implementation
crate::openai_compatible_language_model!(TencentCodingPlan);
