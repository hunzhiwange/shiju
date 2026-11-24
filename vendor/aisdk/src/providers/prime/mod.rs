//! This module provides the Prime provider, wrapping OpenAI Chat Completions for Prime requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    PrimeProviderSettings,
    PrimeProviderSettingsBuilder,
    "Prime",
    "https://api.pinference.ai/api/v1",
    "PRIME_API_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(Prime, PrimeBuilder, PrimeProviderSettings, "prime");

// Generate the language model implementation
crate::openai_compatible_language_model!(Prime);
