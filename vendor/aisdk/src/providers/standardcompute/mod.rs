//! This module provides the Standardcompute provider, wrapping OpenAI Chat Completions for Standardcompute requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    StandardcomputeProviderSettings,
    StandardcomputeProviderSettingsBuilder,
    "Standardcompute",
    "https://api.stdcmpt.com/v1",
    "STANDARDCOMPUTE_API_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(
    Standardcompute,
    StandardcomputeBuilder,
    StandardcomputeProviderSettings,
    "standardcompute"
);

// Generate the language model implementation
crate::openai_compatible_language_model!(Standardcompute);
