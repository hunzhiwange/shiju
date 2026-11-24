//! This module provides the Arcee provider, wrapping OpenAI Chat Completions for Arcee requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    ArceeProviderSettings,
    ArceeProviderSettingsBuilder,
    "Arcee",
    "https://api.arcee.ai/api/v1",
    "ARCEE_API_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(Arcee, ArceeBuilder, ArceeProviderSettings, "arcee");

// Generate the language model implementation
crate::openai_compatible_language_model!(Arcee);
