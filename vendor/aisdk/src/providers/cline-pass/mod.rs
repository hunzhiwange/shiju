//! This module provides the ClinePass provider, wrapping OpenAI Chat Completions for ClinePass requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    ClinePassProviderSettings,
    ClinePassProviderSettingsBuilder,
    "ClinePass",
    "https://api.cline.bot/api/v1",
    "CLINE_API_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(
    ClinePass,
    ClinePassBuilder,
    ClinePassProviderSettings,
    "cline-pass"
);

// Generate the language model implementation
crate::openai_compatible_language_model!(ClinePass);
