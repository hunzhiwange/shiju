//! This module provides the ScxAi provider, wrapping OpenAI Chat Completions for ScxAi requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    ScxAiProviderSettings,
    ScxAiProviderSettingsBuilder,
    "ScxAi",
    "https://api.scx.ai/v1",
    "SCX_API_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(ScxAi, ScxAiBuilder, ScxAiProviderSettings, "scx-ai");

// Generate the language model implementation
crate::openai_compatible_language_model!(ScxAi);
