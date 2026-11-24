//! This module provides the Edenai provider, wrapping OpenAI Chat Completions for Edenai requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    EdenaiProviderSettings,
    EdenaiProviderSettingsBuilder,
    "Edenai",
    "https://api.edenai.run/v3",
    "EDENAI_API_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(Edenai, EdenaiBuilder, EdenaiProviderSettings, "edenai");

// Generate the language model implementation
crate::openai_compatible_language_model!(Edenai);
