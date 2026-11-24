//! This module provides the Hyper provider, wrapping OpenAI Chat Completions for Hyper requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    HyperProviderSettings,
    HyperProviderSettingsBuilder,
    "Hyper",
    "https://hyper.charm.land/v1",
    "HYPER_API_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(Hyper, HyperBuilder, HyperProviderSettings, "hyper");

// Generate the language model implementation
crate::openai_compatible_language_model!(Hyper);
