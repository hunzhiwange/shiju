//! This module provides the CloudferroSherlock provider, wrapping OpenAI Chat Completions for CloudferroSherlock requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    CloudferroSherlockProviderSettings,
    CloudferroSherlockProviderSettingsBuilder,
    "CloudferroSherlock",
    "https://api-sherlock.cloudferro.com/openai/v1/",
    "CLOUDFERRO_SHERLOCK_API_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(
    CloudferroSherlock,
    CloudferroSherlockBuilder,
    CloudferroSherlockProviderSettings,
    "cloudferro-sherlock"
);

// Generate the language model implementation
crate::openai_compatible_language_model!(CloudferroSherlock);
