//! This module provides the Melious provider, wrapping OpenAI Chat Completions for Melious requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    MeliousProviderSettings,
    MeliousProviderSettingsBuilder,
    "Melious",
    "https://api.melious.ai/v1",
    "MELIOUS_API_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(Melious, MeliousBuilder, MeliousProviderSettings, "melious");

// Generate the language model implementation
crate::openai_compatible_language_model!(Melious);
