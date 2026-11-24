//! This module provides the Xpersona provider, wrapping OpenAI Chat Completions for Xpersona requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    XpersonaProviderSettings,
    XpersonaProviderSettingsBuilder,
    "Xpersona",
    "https://www.xpersona.co/v1",
    "XPERSONA_API_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(
    Xpersona,
    XpersonaBuilder,
    XpersonaProviderSettings,
    "xpersona"
);

// Generate the language model implementation
crate::openai_compatible_language_model!(Xpersona);
