//! This module provides the Commandcode provider, wrapping OpenAI Chat Completions for Commandcode requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    CommandcodeProviderSettings,
    CommandcodeProviderSettingsBuilder,
    "Commandcode",
    "https://api.commandcode.ai/provider/v1",
    "CMD_API_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(
    Commandcode,
    CommandcodeBuilder,
    CommandcodeProviderSettings,
    "commandcode"
);

// Generate the language model implementation
crate::openai_compatible_language_model!(Commandcode);
