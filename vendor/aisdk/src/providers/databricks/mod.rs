//! This module provides the Databricks provider, wrapping OpenAI Chat Completions for Databricks requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    DatabricksProviderSettings,
    DatabricksProviderSettingsBuilder,
    "Databricks",
    "https://${DATABRICKS_HOST}/ai-gateway/mlflow/v1",
    "DATABRICKS_HOST"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(
    Databricks,
    DatabricksBuilder,
    DatabricksProviderSettings,
    "databricks"
);

// Generate the language model implementation
crate::openai_compatible_language_model!(Databricks);
