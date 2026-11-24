//! This module provides the StepfunAi provider, wrapping OpenAI Chat Completions for StepfunAi requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    StepfunAiProviderSettings,
    StepfunAiProviderSettingsBuilder,
    "StepfunAi",
    "https://api.stepfun.ai/v1",
    "STEPFUN_API_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(
    StepfunAi,
    StepfunAiBuilder,
    StepfunAiProviderSettings,
    "stepfun-ai"
);

// Generate the language model implementation
crate::openai_compatible_language_model!(StepfunAi);
