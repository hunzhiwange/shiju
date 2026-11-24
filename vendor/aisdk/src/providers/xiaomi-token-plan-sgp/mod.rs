//! This module provides the XiaomiTokenPlanSgp provider, wrapping OpenAI Chat Completions for XiaomiTokenPlanSgp requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    XiaomiTokenPlanSgpProviderSettings,
    XiaomiTokenPlanSgpProviderSettingsBuilder,
    "XiaomiTokenPlanSgp",
    "https://token-plan-sgp.xiaomimimo.com/v1",
    "XIAOMI_API_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(
    XiaomiTokenPlanSgp,
    XiaomiTokenPlanSgpBuilder,
    XiaomiTokenPlanSgpProviderSettings,
    "xiaomi-token-plan-sgp"
);

// Generate the language model implementation
crate::openai_compatible_language_model!(XiaomiTokenPlanSgp);
