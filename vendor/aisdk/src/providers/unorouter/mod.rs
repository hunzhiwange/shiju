//! This module provides the Unorouter provider, wrapping OpenAI Chat Completions for Unorouter requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    UnorouterProviderSettings,
    UnorouterProviderSettingsBuilder,
    "Unorouter",
    "https://api.unorouter.com/v1",
    "UNOROUTER_API_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(
    Unorouter,
    UnorouterBuilder,
    UnorouterProviderSettings,
    "unorouter"
);

// Generate the language model implementation
crate::openai_compatible_language_model!(Unorouter);
