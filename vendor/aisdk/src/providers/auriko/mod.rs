//! This module provides the Auriko provider, wrapping OpenAI Chat Completions for Auriko requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    AurikoProviderSettings,
    AurikoProviderSettingsBuilder,
    "Auriko",
    "https://api.auriko.ai/v1",
    "AURIKO_API_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(Auriko, AurikoBuilder, AurikoProviderSettings, "auriko");

// Generate the language model implementation
crate::openai_compatible_language_model!(Auriko);
