//! This module provides the Oci provider, wrapping OpenAI Chat Completions for Oci requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    OciProviderSettings,
    OciProviderSettingsBuilder,
    "Oci",
    "https://inference.generativeai.us-chicago-1.oci.oraclecloud.com/openai/v1",
    "OCI_GENAI_API_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(Oci, OciBuilder, OciProviderSettings, "oci");

// Generate the language model implementation
crate::openai_compatible_language_model!(Oci);
