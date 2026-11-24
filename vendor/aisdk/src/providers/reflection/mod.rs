//! This module provides the Reflection provider, wrapping OpenAI Chat Completions for Reflection requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    ReflectionProviderSettings,
    ReflectionProviderSettingsBuilder,
    "Reflection",
    "https://api.reflection.ai/openai/v1",
    "REFLECTION_API_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(
    Reflection,
    ReflectionBuilder,
    ReflectionProviderSettings,
    "reflection"
);

// Generate the language model implementation
crate::openai_compatible_language_model!(Reflection);
