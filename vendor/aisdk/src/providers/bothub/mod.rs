//! This module provides the Bothub provider, wrapping OpenAI Chat Completions for Bothub requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    BothubProviderSettings,
    BothubProviderSettingsBuilder,
    "Bothub",
    "https://openai.bothub.ru/v1",
    "BOTHUB_API_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(Bothub, BothubBuilder, BothubProviderSettings, "bothub");

// Generate the language model implementation
crate::openai_compatible_language_model!(Bothub);
