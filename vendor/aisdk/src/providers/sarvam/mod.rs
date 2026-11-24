//! This module provides the Sarvam provider, wrapping OpenAI Chat Completions for Sarvam requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    SarvamProviderSettings,
    SarvamProviderSettingsBuilder,
    "Sarvam",
    "https://api.sarvam.ai/v1",
    "SARVAM_API_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(Sarvam, SarvamBuilder, SarvamProviderSettings, "sarvam");

// Generate the language model implementation
crate::openai_compatible_language_model!(Sarvam);
