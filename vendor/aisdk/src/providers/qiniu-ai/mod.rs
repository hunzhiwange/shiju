//! This module provides the QiniuAi provider, wrapping OpenAI Chat Completions for QiniuAi requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    QiniuAiProviderSettings,
    QiniuAiProviderSettingsBuilder,
    "QiniuAi",
    "https://api.qnaigc.com/v1",
    "QINIU_API_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(QiniuAi, QiniuAiBuilder, QiniuAiProviderSettings, "qiniu-ai");

// Generate the language model implementation
crate::openai_compatible_language_model!(QiniuAi);
