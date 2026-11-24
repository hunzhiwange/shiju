//! This module provides the Agentrouter provider, wrapping OpenAI Chat Completions for Agentrouter requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    AgentrouterProviderSettings,
    AgentrouterProviderSettingsBuilder,
    "Agentrouter",
    "https://agentrouter.org/v1",
    "AGENTROUTER_API_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(
    Agentrouter,
    AgentrouterBuilder,
    AgentrouterProviderSettings,
    "agentrouter"
);

// Generate the language model implementation
crate::openai_compatible_language_model!(Agentrouter);
