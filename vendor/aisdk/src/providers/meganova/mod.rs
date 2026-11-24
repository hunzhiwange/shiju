//! This module provides the Meganova provider, wrapping OpenAI Chat Completions for Meganova requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    MeganovaProviderSettings,
    MeganovaProviderSettingsBuilder,
    "Meganova",
    "https://api.meganova.ai/v1",
    "MEGANOVA_API_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(
    Meganova,
    MeganovaBuilder,
    MeganovaProviderSettings,
    "meganova"
);

// Generate the language model implementation
crate::openai_compatible_language_model!(Meganova);
