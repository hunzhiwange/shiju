//! This module provides the Kilo provider, wrapping OpenAI Chat Completions for Kilo requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    KiloProviderSettings,
    KiloProviderSettingsBuilder,
    "Kilo",
    "https://api.kilo.ai/api/gateway",
    "KILO_API_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(Kilo, KiloBuilder, KiloProviderSettings, "kilo");

// Generate the language model implementation
crate::openai_compatible_language_model!(Kilo);
