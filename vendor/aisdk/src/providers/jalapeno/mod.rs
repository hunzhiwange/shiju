//! This module provides the Jalapeno provider, wrapping OpenAI Chat Completions for Jalapeno requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    JalapenoProviderSettings,
    JalapenoProviderSettingsBuilder,
    "Jalapeno",
    "https://api.jalapeno-cloud.ai/v1",
    "JALAPENO_API_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(
    Jalapeno,
    JalapenoBuilder,
    JalapenoProviderSettings,
    "jalapeno"
);

// Generate the language model implementation
crate::openai_compatible_language_model!(Jalapeno);
