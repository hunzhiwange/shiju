//! This module provides the XiaomiTokenPlanCn provider, wrapping OpenAI Chat Completions for XiaomiTokenPlanCn requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    XiaomiTokenPlanCnProviderSettings,
    XiaomiTokenPlanCnProviderSettingsBuilder,
    "XiaomiTokenPlanCn",
    "https://token-plan-cn.xiaomimimo.com/v1",
    "XIAOMI_API_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(
    XiaomiTokenPlanCn,
    XiaomiTokenPlanCnBuilder,
    XiaomiTokenPlanCnProviderSettings,
    "xiaomi-token-plan-cn"
);

// Generate the language model implementation
crate::openai_compatible_language_model!(XiaomiTokenPlanCn);
