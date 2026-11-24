//! This module provides the Lowrouter provider, wrapping OpenAI Chat Completions for Lowrouter requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    LowrouterProviderSettings,
    LowrouterProviderSettingsBuilder,
    "Lowrouter",
    "https://api.lowrouter.ai/v1",
    "LOWROUTER_API_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(
    Lowrouter,
    LowrouterBuilder,
    LowrouterProviderSettings,
    "lowrouter"
);

// Generate the language model implementation
crate::openai_compatible_language_model!(Lowrouter);
