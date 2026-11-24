//! This module provides the KimiCodePlanCn provider, wrapping OpenAI Chat Completions for KimiCodePlanCn requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    KimiCodePlanCnProviderSettings,
    KimiCodePlanCnProviderSettingsBuilder,
    "KimiCodePlanCn",
    "https://api.kimi.com/coding/v1",
    "KIMI_API_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(
    KimiCodePlanCn,
    KimiCodePlanCnBuilder,
    KimiCodePlanCnProviderSettings,
    "kimi-code-plan-cn"
);

// Generate the language model implementation
crate::openai_compatible_language_model!(KimiCodePlanCn);
