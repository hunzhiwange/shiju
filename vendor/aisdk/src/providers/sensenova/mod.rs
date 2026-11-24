//! This module provides the Sensenova provider, wrapping OpenAI Chat Completions for Sensenova requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    SensenovaProviderSettings,
    SensenovaProviderSettingsBuilder,
    "Sensenova",
    "https://token.sensenova.cn/v1",
    "SENSENOVA_API_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(
    Sensenova,
    SensenovaBuilder,
    SensenovaProviderSettings,
    "sensenova"
);

// Generate the language model implementation
crate::openai_compatible_language_model!(Sensenova);
