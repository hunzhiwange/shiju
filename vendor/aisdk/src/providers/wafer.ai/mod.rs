//! This module provides the WaferAi provider, wrapping OpenAI Chat Completions for WaferAi requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    WaferAiProviderSettings,
    WaferAiProviderSettingsBuilder,
    "WaferAi",
    "https://pass.wafer.ai/v1",
    "WAFER_API_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(WaferAi, WaferAiBuilder, WaferAiProviderSettings, "wafer.ai");

// Generate the language model implementation
crate::openai_compatible_language_model!(WaferAi);
