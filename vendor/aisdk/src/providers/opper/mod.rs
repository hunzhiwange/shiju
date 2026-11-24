//! This module provides the Opper provider, wrapping OpenAI Chat Completions for Opper requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    OpperProviderSettings,
    OpperProviderSettingsBuilder,
    "Opper",
    "https://api.opper.ai/v3/compat",
    "OPPER_API_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(Opper, OpperBuilder, OpperProviderSettings, "opper");

// Generate the language model implementation
crate::openai_compatible_language_model!(Opper);
