//! This module provides the Tokengo provider, wrapping OpenAI Chat Completions for Tokengo requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    TokengoProviderSettings,
    TokengoProviderSettingsBuilder,
    "Tokengo",
    "https://api.tokengo.com/v1",
    "TOKENGO_API_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(Tokengo, TokengoBuilder, TokengoProviderSettings, "tokengo");

// Generate the language model implementation
crate::openai_compatible_language_model!(Tokengo);
