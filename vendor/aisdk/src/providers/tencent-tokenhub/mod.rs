//! This module provides the TencentTokenhub provider, wrapping OpenAI Chat Completions for TencentTokenhub requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    TencentTokenhubProviderSettings,
    TencentTokenhubProviderSettingsBuilder,
    "TencentTokenhub",
    "https://tokenhub.tencentmaas.com/v1",
    "TENCENT_TOKENHUB_API_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(
    TencentTokenhub,
    TencentTokenhubBuilder,
    TencentTokenhubProviderSettings,
    "tencent-tokenhub"
);

// Generate the language model implementation
crate::openai_compatible_language_model!(TencentTokenhub);
