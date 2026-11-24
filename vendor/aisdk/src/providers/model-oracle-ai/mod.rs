//! This module provides the ModelOracleAi provider, wrapping OpenAI Chat Completions for ModelOracleAi requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    ModelOracleAiProviderSettings,
    ModelOracleAiProviderSettingsBuilder,
    "ModelOracleAi",
    "https://api.modeloracle.com/api/v1",
    "MODEL_ORACLE_API_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(
    ModelOracleAi,
    ModelOracleAiBuilder,
    ModelOracleAiProviderSettings,
    "model-oracle-ai"
);

// Generate the language model implementation
crate::openai_compatible_language_model!(ModelOracleAi);
