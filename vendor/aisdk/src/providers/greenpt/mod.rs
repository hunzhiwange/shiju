//! This module provides the Greenpt provider, wrapping OpenAI Chat Completions for Greenpt requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    GreenptProviderSettings,
    GreenptProviderSettingsBuilder,
    "Greenpt",
    "https://api.greenpt.ai/v1",
    "GREENPT_API_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(Greenpt, GreenptBuilder, GreenptProviderSettings, "greenpt");

// Generate the language model implementation
crate::openai_compatible_language_model!(Greenpt);
