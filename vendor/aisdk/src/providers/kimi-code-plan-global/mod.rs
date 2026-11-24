//! This module provides the KimiCodePlanGlobal provider, wrapping OpenAI Chat Completions for KimiCodePlanGlobal requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    KimiCodePlanGlobalProviderSettings,
    KimiCodePlanGlobalProviderSettingsBuilder,
    "KimiCodePlanGlobal",
    "https://api.kimi.ai/coding/v1",
    "KIMI_API_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(
    KimiCodePlanGlobal,
    KimiCodePlanGlobalBuilder,
    KimiCodePlanGlobalProviderSettings,
    "kimi-code-plan-global"
);

// Generate the language model implementation
crate::openai_compatible_language_model!(KimiCodePlanGlobal);
