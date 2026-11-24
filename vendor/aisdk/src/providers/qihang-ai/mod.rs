//! This module provides the QihangAi provider, wrapping OpenAI Chat Completions for QihangAi requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    QihangAiProviderSettings,
    QihangAiProviderSettingsBuilder,
    "QihangAi",
    "https://api.qhaigc.net/v1",
    "QIHANG_API_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(
    QihangAi,
    QihangAiBuilder,
    QihangAiProviderSettings,
    "qihang-ai"
);

// Generate the language model implementation
crate::openai_compatible_language_model!(QihangAi);
