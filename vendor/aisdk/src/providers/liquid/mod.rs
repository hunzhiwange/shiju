//! This module provides the Liquid provider, wrapping OpenAI Chat Completions for Liquid requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    LiquidProviderSettings,
    LiquidProviderSettingsBuilder,
    "Liquid",
    "https://router.inference.ai.exchange/v1",
    "LIQUID_API_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(Liquid, LiquidBuilder, LiquidProviderSettings, "liquid");

// Generate the language model implementation
crate::openai_compatible_language_model!(Liquid);
