//! This module provides the Iteracompute provider, wrapping OpenAI Chat Completions for Iteracompute requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    IteracomputeProviderSettings,
    IteracomputeProviderSettingsBuilder,
    "Iteracompute",
    "https://api.iteracompute.com/v1",
    "ITERACOMPUTE_API_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(
    Iteracompute,
    IteracomputeBuilder,
    IteracomputeProviderSettings,
    "iteracompute"
);

// Generate the language model implementation
crate::openai_compatible_language_model!(Iteracompute);
