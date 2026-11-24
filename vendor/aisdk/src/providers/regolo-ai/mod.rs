//! This module provides the RegoloAi provider, wrapping OpenAI Chat Completions for RegoloAi requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    RegoloAiProviderSettings,
    RegoloAiProviderSettingsBuilder,
    "RegoloAi",
    "https://api.regolo.ai/v1",
    "REGOLO_API_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(
    RegoloAi,
    RegoloAiBuilder,
    RegoloAiProviderSettings,
    "regolo-ai"
);

// Generate the language model implementation
crate::openai_compatible_language_model!(RegoloAi);
