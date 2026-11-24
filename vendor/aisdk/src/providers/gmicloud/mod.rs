//! This module provides the Gmicloud provider, wrapping OpenAI Chat Completions for Gmicloud requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    GmicloudProviderSettings,
    GmicloudProviderSettingsBuilder,
    "Gmicloud",
    "https://api.gmi-serving.com/v1",
    "GMICLOUD_API_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(
    Gmicloud,
    GmicloudBuilder,
    GmicloudProviderSettings,
    "gmicloud"
);

// Generate the language model implementation
crate::openai_compatible_language_model!(Gmicloud);
