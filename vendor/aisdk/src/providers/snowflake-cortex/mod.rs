//! This module provides the SnowflakeCortex provider, wrapping OpenAI Chat Completions for SnowflakeCortex requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    SnowflakeCortexProviderSettings,
    SnowflakeCortexProviderSettingsBuilder,
    "SnowflakeCortex",
    "https://${SNOWFLAKE_ACCOUNT}.snowflakecomputing.com/api/v2/cortex/v1",
    "SNOWFLAKE_ACCOUNT"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(
    SnowflakeCortex,
    SnowflakeCortexBuilder,
    SnowflakeCortexProviderSettings,
    "snowflake-cortex"
);

// Generate the language model implementation
crate::openai_compatible_language_model!(SnowflakeCortex);
