//! This module provides the Openreason provider, wrapping OpenAI Chat Completions for Openreason requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    OpenreasonProviderSettings,
    OpenreasonProviderSettingsBuilder,
    "Openreason",
    "https://api.openreason.app/v1",
    "OPENREASON_API_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(
    Openreason,
    OpenreasonBuilder,
    OpenreasonProviderSettings,
    "openreason"
);

// Generate the language model implementation
crate::openai_compatible_language_model!(Openreason);
