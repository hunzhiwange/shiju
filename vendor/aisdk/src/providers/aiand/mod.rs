//! This module provides the Aiand provider, wrapping OpenAI Chat Completions for Aiand requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    AiandProviderSettings,
    AiandProviderSettingsBuilder,
    "Aiand",
    "https://api.aiand.com/v1",
    "AIAND_API_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(Aiand, AiandBuilder, AiandProviderSettings, "aiand");

// Generate the language model implementation
crate::openai_compatible_language_model!(Aiand);
