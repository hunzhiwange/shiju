//! This module provides the Inco provider, wrapping OpenAI Chat Completions for Inco requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    IncoProviderSettings,
    IncoProviderSettingsBuilder,
    "Inco",
    "https://api.inco.ai/v1",
    "INCO_API_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(Inco, IncoBuilder, IncoProviderSettings, "inco");

// Generate the language model implementation
crate::openai_compatible_language_model!(Inco);
