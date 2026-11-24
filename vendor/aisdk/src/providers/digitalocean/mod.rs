//! This module provides the Digitalocean provider, wrapping OpenAI Chat Completions for Digitalocean requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    DigitaloceanProviderSettings,
    DigitaloceanProviderSettingsBuilder,
    "Digitalocean",
    "https://inference.do-ai.run/v1",
    "DIGITALOCEAN_ACCESS_TOKEN"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(
    Digitalocean,
    DigitaloceanBuilder,
    DigitaloceanProviderSettings,
    "digitalocean"
);

// Generate the language model implementation
crate::openai_compatible_language_model!(Digitalocean);
