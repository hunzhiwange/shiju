//! This module provides the XiaomiTokenPlanAms provider, wrapping OpenAI Chat Completions for XiaomiTokenPlanAms requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    XiaomiTokenPlanAmsProviderSettings,
    XiaomiTokenPlanAmsProviderSettingsBuilder,
    "XiaomiTokenPlanAms",
    "https://token-plan-ams.xiaomimimo.com/v1",
    "XIAOMI_API_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(
    XiaomiTokenPlanAms,
    XiaomiTokenPlanAmsBuilder,
    XiaomiTokenPlanAmsProviderSettings,
    "xiaomi-token-plan-ams"
);

// Generate the language model implementation
crate::openai_compatible_language_model!(XiaomiTokenPlanAms);
