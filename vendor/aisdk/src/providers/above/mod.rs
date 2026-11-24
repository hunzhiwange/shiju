//! This module provides the Above provider, wrapping OpenAI Chat Completions for Above requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    AboveProviderSettings,
    AboveProviderSettingsBuilder,
    "Above",
    "https://api.above.dev/v1",
    "ABOVE_API_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(Above, AboveBuilder, AboveProviderSettings, "above");

// Generate the language model implementation
crate::openai_compatible_language_model!(Above);
