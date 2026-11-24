//! This module provides the Poolside provider, wrapping OpenAI Chat Completions for Poolside requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    PoolsideProviderSettings,
    PoolsideProviderSettingsBuilder,
    "Poolside",
    "https://inference.poolside.ai/v1",
    "POOLSIDE_API_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(
    Poolside,
    PoolsideBuilder,
    PoolsideProviderSettings,
    "poolside"
);

// Generate the language model implementation
crate::openai_compatible_language_model!(Poolside);
