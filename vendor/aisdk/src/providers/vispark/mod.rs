//! This module provides the Vispark provider, wrapping OpenAI Chat Completions for Vispark requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    VisparkProviderSettings,
    VisparkProviderSettingsBuilder,
    "Vispark",
    "https://api.lab.vispark.in/v1",
    "VISPARK_LAB_API_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(Vispark, VisparkBuilder, VisparkProviderSettings, "vispark");

// Generate the language model implementation
crate::openai_compatible_language_model!(Vispark);
