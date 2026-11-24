//! This module provides the Claudinio provider, wrapping OpenAI Chat Completions for Claudinio requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    ClaudinioProviderSettings,
    ClaudinioProviderSettingsBuilder,
    "Claudinio",
    "https://api.claudin.io/v1",
    "CLAUDINIO_API_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(
    Claudinio,
    ClaudinioBuilder,
    ClaudinioProviderSettings,
    "claudinio"
);

// Generate the language model implementation
crate::openai_compatible_language_model!(Claudinio);
