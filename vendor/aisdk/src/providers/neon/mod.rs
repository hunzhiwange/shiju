//! This module provides the Neon provider, wrapping OpenAI Chat Completions for Neon requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    NeonProviderSettings,
    NeonProviderSettingsBuilder,
    "Neon",
    "${NEON_AI_GATEWAY_BASE_URL}/v1",
    "NEON_AI_GATEWAY_BASE_URL"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(Neon, NeonBuilder, NeonProviderSettings, "neon");

// Generate the language model implementation
crate::openai_compatible_language_model!(Neon);
