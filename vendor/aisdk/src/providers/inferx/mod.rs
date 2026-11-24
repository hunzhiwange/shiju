//! This module provides the Inferx provider, wrapping OpenAI Chat Completions for Inferx requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    InferxProviderSettings,
    InferxProviderSettingsBuilder,
    "Inferx",
    "https://model.inferx.net/endpoints/v1",
    "INFERX_API_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(Inferx, InferxBuilder, InferxProviderSettings, "inferx");

// Generate the language model implementation
crate::openai_compatible_language_model!(Inferx);
