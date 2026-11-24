//! This module provides the AbliterationAi provider, wrapping OpenAI Chat Completions for AbliterationAi requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    AbliterationAiProviderSettings,
    AbliterationAiProviderSettingsBuilder,
    "AbliterationAi",
    "https://api.abliteration.ai/v1",
    "ABLIT_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(
    AbliterationAi,
    AbliterationAiBuilder,
    AbliterationAiProviderSettings,
    "abliteration-ai"
);

// Generate the language model implementation
crate::openai_compatible_language_model!(AbliterationAi);
