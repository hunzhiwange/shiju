//! This module provides the Tensorx provider, wrapping OpenAI Chat Completions for Tensorx requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    TensorxProviderSettings,
    TensorxProviderSettingsBuilder,
    "Tensorx",
    "https://api.tensorx.ai/v1",
    "TENSORX_API_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(Tensorx, TensorxBuilder, TensorxProviderSettings, "tensorx");

// Generate the language model implementation
crate::openai_compatible_language_model!(Tensorx);
