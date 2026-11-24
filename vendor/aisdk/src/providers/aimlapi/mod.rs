//! This module provides the Aimlapi provider, wrapping OpenAI Chat Completions for Aimlapi requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    AimlapiProviderSettings,
    AimlapiProviderSettingsBuilder,
    "Aimlapi",
    "https://api.aimlapi.com/v1",
    "AIMLAPI_API_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(Aimlapi, AimlapiBuilder, AimlapiProviderSettings, "aimlapi");

// Generate the language model implementation
crate::openai_compatible_language_model!(Aimlapi);
