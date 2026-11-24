//! This module provides the Dinference provider, wrapping OpenAI Chat Completions for Dinference requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    DinferenceProviderSettings,
    DinferenceProviderSettingsBuilder,
    "Dinference",
    "https://api.dinference.com/v1",
    "DINFERENCE_API_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(
    Dinference,
    DinferenceBuilder,
    DinferenceProviderSettings,
    "dinference"
);

// Generate the language model implementation
crate::openai_compatible_language_model!(Dinference);
