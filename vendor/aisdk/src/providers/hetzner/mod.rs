//! This module provides the Hetzner provider, wrapping OpenAI Chat Completions for Hetzner requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    HetznerProviderSettings,
    HetznerProviderSettingsBuilder,
    "Hetzner",
    "https://inference.hetzner.com/api/v1",
    "HETZNER_API_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(Hetzner, HetznerBuilder, HetznerProviderSettings, "hetzner");

// Generate the language model implementation
crate::openai_compatible_language_model!(Hetzner);
