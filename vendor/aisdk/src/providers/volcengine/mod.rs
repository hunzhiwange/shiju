//! This module provides the Volcengine provider, wrapping OpenAI Chat Completions for Volcengine requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    VolcengineProviderSettings,
    VolcengineProviderSettingsBuilder,
    "Volcengine",
    "https://ark.cn-beijing.volces.com/api/v3",
    "ARK_API_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(
    Volcengine,
    VolcengineBuilder,
    VolcengineProviderSettings,
    "volcengine"
);

// Generate the language model implementation
crate::openai_compatible_language_model!(Volcengine);
