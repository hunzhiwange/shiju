//! This module provides the Zenifra provider, wrapping OpenAI Chat Completions for Zenifra requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    ZenifraProviderSettings,
    ZenifraProviderSettingsBuilder,
    "Zenifra",
    "https://ai.zenifra.com/v1",
    "ZENIFRA_AI_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(Zenifra, ZenifraBuilder, ZenifraProviderSettings, "zenifra");

// Generate the language model implementation
crate::openai_compatible_language_model!(Zenifra);
