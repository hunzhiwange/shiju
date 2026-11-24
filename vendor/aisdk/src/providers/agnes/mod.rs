//! This module provides the Agnes provider, wrapping OpenAI Chat Completions for Agnes requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    AgnesProviderSettings,
    AgnesProviderSettingsBuilder,
    "Agnes",
    "https://apihub.agnes-ai.com/v1",
    "AGNES_API_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(Agnes, AgnesBuilder, AgnesProviderSettings, "agnes");

// Generate the language model implementation
crate::openai_compatible_language_model!(Agnes);
