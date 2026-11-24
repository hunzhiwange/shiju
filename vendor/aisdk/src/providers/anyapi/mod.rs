//! This module provides the Anyapi provider, wrapping OpenAI Chat Completions for Anyapi requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    AnyapiProviderSettings,
    AnyapiProviderSettingsBuilder,
    "Anyapi",
    "https://api.anyapi.ai/v1",
    "ANYAPI_API_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(Anyapi, AnyapiBuilder, AnyapiProviderSettings, "anyapi");

// Generate the language model implementation
crate::openai_compatible_language_model!(Anyapi);
