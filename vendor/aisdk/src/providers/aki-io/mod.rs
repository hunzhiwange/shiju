//! This module provides the AkiIo provider, wrapping OpenAI Chat Completions for AkiIo requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    AkiIoProviderSettings,
    AkiIoProviderSettingsBuilder,
    "AkiIo",
    "https://aki.io/v1",
    "AKI_IO_API_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(AkiIo, AkiIoBuilder, AkiIoProviderSettings, "aki-io");

// Generate the language model implementation
crate::openai_compatible_language_model!(AkiIo);
