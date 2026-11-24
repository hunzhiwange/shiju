//! This module provides the Evroc provider, wrapping OpenAI Chat Completions for Evroc requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    EvrocProviderSettings,
    EvrocProviderSettingsBuilder,
    "Evroc",
    "https://models.think.evroc.com/v1",
    "EVROC_API_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(Evroc, EvrocBuilder, EvrocProviderSettings, "evroc");

// Generate the language model implementation
crate::openai_compatible_language_model!(Evroc);
