//! This module provides the Bee provider, wrapping OpenAI Chat Completions for Bee requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    BeeProviderSettings,
    BeeProviderSettingsBuilder,
    "Bee",
    "https://api.bee.heossi.com/bee",
    "BEE_API_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(Bee, BeeBuilder, BeeProviderSettings, "bee");

// Generate the language model implementation
crate::openai_compatible_language_model!(Bee);
