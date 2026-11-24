//! This module provides the Runinfra provider, wrapping OpenAI Chat Completions for Runinfra requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    RuninfraProviderSettings,
    RuninfraProviderSettingsBuilder,
    "Runinfra",
    "https://api.runinfra.ai/v1",
    "RUNINFRA_GATEWAY_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(
    Runinfra,
    RuninfraBuilder,
    RuninfraProviderSettings,
    "runinfra"
);

// Generate the language model implementation
crate::openai_compatible_language_model!(Runinfra);
