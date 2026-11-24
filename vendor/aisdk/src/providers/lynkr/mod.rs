//! This module provides the Lynkr provider, wrapping OpenAI Chat Completions for Lynkr requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    LynkrProviderSettings,
    LynkrProviderSettingsBuilder,
    "Lynkr",
    "http://127.0.0.1:8081/v1",
    "LYNKR_API_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(Lynkr, LynkrBuilder, LynkrProviderSettings, "lynkr");

// Generate the language model implementation
crate::openai_compatible_language_model!(Lynkr);
