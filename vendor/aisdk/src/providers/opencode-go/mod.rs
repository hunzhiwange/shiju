//! This module provides the OpencodeGo provider, wrapping OpenAI Chat Completions for OpencodeGo requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    OpencodeGoProviderSettings,
    OpencodeGoProviderSettingsBuilder,
    "OpencodeGo",
    "https://opencode.ai/inference/go/openai/v1",
    "OPENCODE_API_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(
    OpencodeGo,
    OpencodeGoBuilder,
    OpencodeGoProviderSettings,
    "opencode-go"
);

// Generate the language model implementation
crate::openai_compatible_language_model!(OpencodeGo);
