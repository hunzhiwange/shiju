//! This module provides the Daoxe provider, wrapping OpenAI Chat Completions for Daoxe requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    DaoxeProviderSettings,
    DaoxeProviderSettingsBuilder,
    "Daoxe",
    "https://daoxe.com/v1",
    "DAOXE_API_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(Daoxe, DaoxeBuilder, DaoxeProviderSettings, "daoxe");

// Generate the language model implementation
crate::openai_compatible_language_model!(Daoxe);
