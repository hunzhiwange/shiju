//! This module provides the Orcarouter provider, wrapping OpenAI Chat Completions for Orcarouter requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    OrcarouterProviderSettings,
    OrcarouterProviderSettingsBuilder,
    "Orcarouter",
    "https://api.orcarouter.ai/v1",
    "ORCAROUTER_API_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(
    Orcarouter,
    OrcarouterBuilder,
    OrcarouterProviderSettings,
    "orcarouter"
);

// Generate the language model implementation
crate::openai_compatible_language_model!(Orcarouter);
