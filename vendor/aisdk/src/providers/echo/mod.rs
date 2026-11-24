//! This module provides the Echo provider, wrapping OpenAI Chat Completions for Echo requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    EchoProviderSettings,
    EchoProviderSettingsBuilder,
    "Echo",
    "https://echo.tracerml.ai/v1",
    "ECHO_API_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(Echo, EchoBuilder, EchoProviderSettings, "echo");

// Generate the language model implementation
crate::openai_compatible_language_model!(Echo);
