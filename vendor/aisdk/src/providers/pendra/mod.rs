//! This module provides the Pendra provider, wrapping OpenAI Chat Completions for Pendra requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    PendraProviderSettings,
    PendraProviderSettingsBuilder,
    "Pendra",
    "https://api.pendra.ai/api/v1",
    "PENDRA_API_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(Pendra, PendraBuilder, PendraProviderSettings, "pendra");

// Generate the language model implementation
crate::openai_compatible_language_model!(Pendra);
