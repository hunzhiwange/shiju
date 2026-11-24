//! This module provides the Ebcloud provider, wrapping OpenAI Chat Completions for Ebcloud requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    EbcloudProviderSettings,
    EbcloudProviderSettingsBuilder,
    "Ebcloud",
    "https://maas-api.ebcloud.com/v1",
    "EBCLOUD_API_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(Ebcloud, EbcloudBuilder, EbcloudProviderSettings, "ebcloud");

// Generate the language model implementation
crate::openai_compatible_language_model!(Ebcloud);
