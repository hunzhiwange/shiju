//! This module provides the Kenari provider, wrapping OpenAI Chat Completions for Kenari requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    KenariProviderSettings,
    KenariProviderSettingsBuilder,
    "Kenari",
    "https://kenari.id/v1",
    "KENARI_API_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(Kenari, KenariBuilder, KenariProviderSettings, "kenari");

// Generate the language model implementation
crate::openai_compatible_language_model!(Kenari);
