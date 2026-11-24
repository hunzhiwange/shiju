//! This module provides the Ambient provider, wrapping OpenAI Chat Completions for Ambient requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    AmbientProviderSettings,
    AmbientProviderSettingsBuilder,
    "Ambient",
    "https://api.ambient.xyz/v1",
    "AMBIENT_API_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(Ambient, AmbientBuilder, AmbientProviderSettings, "ambient");

// Generate the language model implementation
crate::openai_compatible_language_model!(Ambient);
