//! This module provides the Coralbricks provider, wrapping OpenAI Chat Completions for Coralbricks requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    CoralbricksProviderSettings,
    CoralbricksProviderSettingsBuilder,
    "Coralbricks",
    "https://inference.coralbricks.ai/v1",
    "CORAL_API_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(
    Coralbricks,
    CoralbricksBuilder,
    CoralbricksProviderSettings,
    "coralbricks"
);

// Generate the language model implementation
crate::openai_compatible_language_model!(Coralbricks);
