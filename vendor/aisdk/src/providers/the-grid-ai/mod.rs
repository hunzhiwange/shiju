//! This module provides the TheGridAi provider, wrapping OpenAI Chat Completions for TheGridAi requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    TheGridAiProviderSettings,
    TheGridAiProviderSettingsBuilder,
    "TheGridAi",
    "https://api.thegrid.ai/v1",
    "THEGRID_API_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(
    TheGridAi,
    TheGridAiBuilder,
    TheGridAiProviderSettings,
    "the-grid-ai"
);

// Generate the language model implementation
crate::openai_compatible_language_model!(TheGridAi);
