//! This module provides the Frogbot provider, wrapping OpenAI Chat Completions for Frogbot requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    FrogbotProviderSettings,
    FrogbotProviderSettingsBuilder,
    "Frogbot",
    "https://app.frogbot.ai/api/v1",
    "FROGBOT_API_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(Frogbot, FrogbotBuilder, FrogbotProviderSettings, "frogbot");

// Generate the language model implementation
crate::openai_compatible_language_model!(Frogbot);
