//! This module provides the UmansAi provider, wrapping OpenAI Chat Completions for UmansAi requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    UmansAiProviderSettings,
    UmansAiProviderSettingsBuilder,
    "UmansAi",
    "https://api.code.umans.ai/v1",
    "UMANS_AI_API_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(UmansAi, UmansAiBuilder, UmansAiProviderSettings, "umans-ai");

// Generate the language model implementation
crate::openai_compatible_language_model!(UmansAi);
