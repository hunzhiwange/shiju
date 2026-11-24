//! This module provides the Nan provider, wrapping OpenAI Chat Completions for Nan requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    NanProviderSettings,
    NanProviderSettingsBuilder,
    "Nan",
    "https://api.nan.builders/v1",
    "NAN_API_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(Nan, NanBuilder, NanProviderSettings, "nan");

// Generate the language model implementation
crate::openai_compatible_language_model!(Nan);
