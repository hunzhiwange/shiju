//! This module provides the Empiriolabs provider, wrapping OpenAI Chat Completions for Empiriolabs requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    EmpiriolabsProviderSettings,
    EmpiriolabsProviderSettingsBuilder,
    "Empiriolabs",
    "https://api.empiriolabs.ai/v1",
    "EMPIRIOLABS_API_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(
    Empiriolabs,
    EmpiriolabsBuilder,
    EmpiriolabsProviderSettings,
    "empiriolabs"
);

// Generate the language model implementation
crate::openai_compatible_language_model!(Empiriolabs);
