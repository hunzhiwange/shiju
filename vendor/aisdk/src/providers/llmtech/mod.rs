//! This module provides the Llmtech provider, wrapping OpenAI Chat Completions for Llmtech requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    LlmtechProviderSettings,
    LlmtechProviderSettingsBuilder,
    "Llmtech",
    "https://api.llmtech.eu/v1",
    "LLMTECH_API_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(Llmtech, LlmtechBuilder, LlmtechProviderSettings, "llmtech");

// Generate the language model implementation
crate::openai_compatible_language_model!(Llmtech);
