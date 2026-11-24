//! This module provides the Lilac provider, wrapping OpenAI Chat Completions for Lilac requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    LilacProviderSettings,
    LilacProviderSettingsBuilder,
    "Lilac",
    "https://api.getlilac.com/v1",
    "LILAC_API_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(Lilac, LilacBuilder, LilacProviderSettings, "lilac");

// Generate the language model implementation
crate::openai_compatible_language_model!(Lilac);
