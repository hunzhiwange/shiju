//! This module provides the Tempr provider, wrapping OpenAI Chat Completions for Tempr requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    TemprProviderSettings,
    TemprProviderSettingsBuilder,
    "Tempr",
    "https://api.temprhq.io/v1",
    "TEMPR_API_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(Tempr, TemprBuilder, TemprProviderSettings, "tempr");

// Generate the language model implementation
crate::openai_compatible_language_model!(Tempr);
