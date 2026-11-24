//! This module provides the Crusoe provider, wrapping OpenAI Chat Completions for Crusoe requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    CrusoeProviderSettings,
    CrusoeProviderSettingsBuilder,
    "Crusoe",
    "https://api.inference.crusoecloud.com/v1",
    "CRUSOE_API_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(Crusoe, CrusoeBuilder, CrusoeProviderSettings, "crusoe");

// Generate the language model implementation
crate::openai_compatible_language_model!(Crusoe);
