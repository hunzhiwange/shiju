//! This module provides the Aixy provider, wrapping OpenAI Chat Completions for Aixy requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    AixyProviderSettings,
    AixyProviderSettingsBuilder,
    "Aixy",
    "https://api.aixy-gateway.com/v1",
    "AIXY_API_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(Aixy, AixyBuilder, AixyProviderSettings, "aixy");

// Generate the language model implementation
crate::openai_compatible_language_model!(Aixy);
