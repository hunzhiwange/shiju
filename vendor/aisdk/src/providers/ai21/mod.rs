//! This module provides the Ai21 provider, wrapping OpenAI Chat Completions for Ai21 requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    Ai21ProviderSettings,
    Ai21ProviderSettingsBuilder,
    "Ai21",
    "https://api.ai21.com/studio/v1",
    "AI21_API_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(Ai21, Ai21Builder, Ai21ProviderSettings, "ai21");

// Generate the language model implementation
crate::openai_compatible_language_model!(Ai21);
