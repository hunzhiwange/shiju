//! This module provides the Longcat provider, wrapping OpenAI Chat Completions for Longcat requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    LongcatProviderSettings,
    LongcatProviderSettingsBuilder,
    "Longcat",
    "https://api.longcat.chat/openai",
    "LONGCAT_API_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(Longcat, LongcatBuilder, LongcatProviderSettings, "longcat");

// Generate the language model implementation
crate::openai_compatible_language_model!(Longcat);
