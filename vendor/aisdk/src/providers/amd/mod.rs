//! This module provides the Amd provider, wrapping OpenAI Chat Completions for Amd requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    AmdProviderSettings,
    AmdProviderSettingsBuilder,
    "Amd",
    "https://developer.amd.com.cn/radeon/api/v1",
    "AMD_API_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(Amd, AmdBuilder, AmdProviderSettings, "amd");

// Generate the language model implementation
crate::openai_compatible_language_model!(Amd);
