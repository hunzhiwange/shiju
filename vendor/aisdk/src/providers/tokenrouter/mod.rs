//! This module provides the Tokenrouter provider, wrapping OpenAI Chat Completions for Tokenrouter requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    TokenrouterProviderSettings,
    TokenrouterProviderSettingsBuilder,
    "Tokenrouter",
    "https://api.tokenrouter.com/v1",
    "TOKENROUTER_API_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(
    Tokenrouter,
    TokenrouterBuilder,
    TokenrouterProviderSettings,
    "tokenrouter"
);

// Generate the language model implementation
crate::openai_compatible_language_model!(Tokenrouter);
