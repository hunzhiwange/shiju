//! This module provides the Ainetcafe provider, wrapping OpenAI Chat Completions for Ainetcafe requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    AinetcafeProviderSettings,
    AinetcafeProviderSettingsBuilder,
    "Ainetcafe",
    "https://microquickjs.com/v1",
    "AINETCAFE_API_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(
    Ainetcafe,
    AinetcafeBuilder,
    AinetcafeProviderSettings,
    "ainetcafe"
);

// Generate the language model implementation
crate::openai_compatible_language_model!(Ainetcafe);
