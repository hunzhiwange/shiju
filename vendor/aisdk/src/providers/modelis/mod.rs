//! This module provides the Modelis provider, wrapping OpenAI Chat Completions for Modelis requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    ModelisProviderSettings,
    ModelisProviderSettingsBuilder,
    "Modelis",
    "https://modelishub.com/v1",
    "MODELIS_API_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(Modelis, ModelisBuilder, ModelisProviderSettings, "modelis");

// Generate the language model implementation
crate::openai_compatible_language_model!(Modelis);
