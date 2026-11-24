//! This module provides the Tinfoil provider, wrapping OpenAI Chat Completions for Tinfoil requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    TinfoilProviderSettings,
    TinfoilProviderSettingsBuilder,
    "Tinfoil",
    "https://inference.tinfoil.sh/v1",
    "TINFOIL_API_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(Tinfoil, TinfoilBuilder, TinfoilProviderSettings, "tinfoil");

// Generate the language model implementation
crate::openai_compatible_language_model!(Tinfoil);
