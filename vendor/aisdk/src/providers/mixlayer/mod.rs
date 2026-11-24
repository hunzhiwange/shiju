//! This module provides the Mixlayer provider, wrapping OpenAI Chat Completions for Mixlayer requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    MixlayerProviderSettings,
    MixlayerProviderSettingsBuilder,
    "Mixlayer",
    "https://models.mixlayer.ai/v1",
    "MIXLAYER_API_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(
    Mixlayer,
    MixlayerBuilder,
    MixlayerProviderSettings,
    "mixlayer"
);

// Generate the language model implementation
crate::openai_compatible_language_model!(Mixlayer);
