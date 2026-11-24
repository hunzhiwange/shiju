//! This module provides the Drun provider, wrapping OpenAI Chat Completions for Drun requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    DrunProviderSettings,
    DrunProviderSettingsBuilder,
    "Drun",
    "https://chat.d.run/v1",
    "DRUN_API_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(Drun, DrunBuilder, DrunProviderSettings, "drun");

// Generate the language model implementation
crate::openai_compatible_language_model!(Drun);
