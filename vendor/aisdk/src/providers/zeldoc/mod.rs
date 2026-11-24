//! This module provides the Zeldoc provider, wrapping OpenAI Chat Completions for Zeldoc requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    ZeldocProviderSettings,
    ZeldocProviderSettingsBuilder,
    "Zeldoc",
    "https://api.zeldoc.ai/v1",
    "ZELDOC_API_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(Zeldoc, ZeldocBuilder, ZeldocProviderSettings, "zeldoc");

// Generate the language model implementation
crate::openai_compatible_language_model!(Zeldoc);
