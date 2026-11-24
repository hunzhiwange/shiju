//! This module provides the Boundless provider, wrapping OpenAI Chat Completions for Boundless requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    BoundlessProviderSettings,
    BoundlessProviderSettingsBuilder,
    "Boundless",
    "https://api.inference.boundless.network/v1",
    "BOUNDLESS_API_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(
    Boundless,
    BoundlessBuilder,
    BoundlessProviderSettings,
    "boundless"
);

// Generate the language model implementation
crate::openai_compatible_language_model!(Boundless);
