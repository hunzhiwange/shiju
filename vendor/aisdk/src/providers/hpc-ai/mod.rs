//! This module provides the HpcAi provider, wrapping OpenAI Chat Completions for HpcAi requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    HpcAiProviderSettings,
    HpcAiProviderSettingsBuilder,
    "HpcAi",
    "https://api.hpc-ai.com/inference/v1",
    "HPC_AI_API_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(HpcAi, HpcAiBuilder, HpcAiProviderSettings, "hpc-ai");

// Generate the language model implementation
crate::openai_compatible_language_model!(HpcAi);
