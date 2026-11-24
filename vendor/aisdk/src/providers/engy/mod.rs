//! This module provides the Engy provider, wrapping OpenAI Chat Completions for Engy requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    EngyProviderSettings,
    EngyProviderSettingsBuilder,
    "Engy",
    "https://api.engy.ai/v1",
    "ENGY_API_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(Engy, EngyBuilder, EngyProviderSettings, "engy");

// Generate the language model implementation
crate::openai_compatible_language_model!(Engy);
