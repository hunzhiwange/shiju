//! This module provides the Klokintegration provider, wrapping OpenAI Chat Completions for Klokintegration requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    KlokintegrationProviderSettings,
    KlokintegrationProviderSettingsBuilder,
    "Klokintegration",
    "https://api-gw.klok.ipaas.se/proxy/kloker-key/v1",
    "KLOKINTEGRATION_API_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(
    Klokintegration,
    KlokintegrationBuilder,
    KlokintegrationProviderSettings,
    "klokintegration"
);

// Generate the language model implementation
crate::openai_compatible_language_model!(Klokintegration);
