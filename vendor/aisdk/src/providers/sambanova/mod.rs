//! This module provides the Sambanova provider, wrapping OpenAI Chat Completions for Sambanova requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    SambanovaProviderSettings,
    SambanovaProviderSettingsBuilder,
    "Sambanova",
    "https://api.sambanova.ai/v1",
    "SAMBANOVA_API_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(
    Sambanova,
    SambanovaBuilder,
    SambanovaProviderSettings,
    "sambanova"
);

// Generate the language model implementation
crate::openai_compatible_language_model!(Sambanova);
