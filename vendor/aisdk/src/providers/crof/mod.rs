//! This module provides the Crof provider, wrapping OpenAI Chat Completions for Crof requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    CrofProviderSettings,
    CrofProviderSettingsBuilder,
    "Crof",
    "https://crof.ai/v1",
    "CROF_API_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(Crof, CrofBuilder, CrofProviderSettings, "crof");

// Generate the language model implementation
crate::openai_compatible_language_model!(Crof);
