//! This module provides the LlmgatewayProviders provider, wrapping OpenAI Chat Completions for LlmgatewayProviders requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    LlmgatewayProvidersProviderSettings,
    LlmgatewayProvidersProviderSettingsBuilder,
    "LlmgatewayProviders",
    "https://api.llmgateway.io/v1",
    "LLMGATEWAY_API_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(
    LlmgatewayProviders,
    LlmgatewayProvidersBuilder,
    LlmgatewayProvidersProviderSettings,
    "llmgateway-providers"
);

// Generate the language model implementation
crate::openai_compatible_language_model!(LlmgatewayProviders);
