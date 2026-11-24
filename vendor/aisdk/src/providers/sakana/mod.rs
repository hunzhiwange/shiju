//! This module provides the Sakana provider, wrapping OpenAI Chat Completions for Sakana requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    SakanaProviderSettings,
    SakanaProviderSettingsBuilder,
    "Sakana",
    "https://api.sakana.ai/v1",
    "SAKANA_API_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(Sakana, SakanaBuilder, SakanaProviderSettings, "sakana");

// Generate the language model implementation
crate::openai_compatible_language_model!(Sakana);
