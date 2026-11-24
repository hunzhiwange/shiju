//! This module provides the Nearai provider, wrapping OpenAI Chat Completions for Nearai requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    NearaiProviderSettings,
    NearaiProviderSettingsBuilder,
    "Nearai",
    "https://cloud-api.near.ai/v1",
    "NEARAI_API_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(Nearai, NearaiBuilder, NearaiProviderSettings, "nearai");

// Generate the language model implementation
crate::openai_compatible_language_model!(Nearai);
