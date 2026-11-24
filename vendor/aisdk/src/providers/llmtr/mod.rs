//! This module provides the Llmtr provider, wrapping OpenAI Chat Completions for Llmtr requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    LlmtrProviderSettings,
    LlmtrProviderSettingsBuilder,
    "Llmtr",
    "https://llmtr.com/v1",
    "LLMTR_API_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(Llmtr, LlmtrBuilder, LlmtrProviderSettings, "llmtr");

// Generate the language model implementation
crate::openai_compatible_language_model!(Llmtr);
