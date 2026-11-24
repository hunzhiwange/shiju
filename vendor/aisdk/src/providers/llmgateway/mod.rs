//! This module provides the Llmgateway provider, wrapping OpenAI Chat Completions for Llmgateway requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    LlmgatewayProviderSettings,
    LlmgatewayProviderSettingsBuilder,
    "Llmgateway",
    "https://api.llmgateway.io/v1",
    "LLMGATEWAY_API_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(
    Llmgateway,
    LlmgatewayBuilder,
    LlmgatewayProviderSettings,
    "llmgateway"
);

// Generate the language model implementation
crate::openai_compatible_language_model!(Llmgateway);
