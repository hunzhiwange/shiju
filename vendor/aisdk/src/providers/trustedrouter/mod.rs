//! This module provides the Trustedrouter provider, wrapping OpenAI Chat Completions for Trustedrouter requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    TrustedrouterProviderSettings,
    TrustedrouterProviderSettingsBuilder,
    "Trustedrouter",
    "https://api.trustedrouter.com/v1",
    "TRUSTEDROUTER_API_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(
    Trustedrouter,
    TrustedrouterBuilder,
    TrustedrouterProviderSettings,
    "trustedrouter"
);

// Generate the language model implementation
crate::openai_compatible_language_model!(Trustedrouter);
