//! This module provides the Pioneer provider, wrapping OpenAI Chat Completions for Pioneer requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    PioneerProviderSettings,
    PioneerProviderSettingsBuilder,
    "Pioneer",
    "https://api.fastino.ai/v1",
    "PIONEER_API_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(Pioneer, PioneerBuilder, PioneerProviderSettings, "pioneer");

// Generate the language model implementation
crate::openai_compatible_language_model!(Pioneer);
