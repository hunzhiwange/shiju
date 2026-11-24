//! This module provides the Ofox provider, wrapping OpenAI Chat Completions for Ofox requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    OfoxProviderSettings,
    OfoxProviderSettingsBuilder,
    "Ofox",
    "https://api.ofox.ai/v1",
    "OFOX_API_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(Ofox, OfoxBuilder, OfoxProviderSettings, "ofox");

// Generate the language model implementation
crate::openai_compatible_language_model!(Ofox);
