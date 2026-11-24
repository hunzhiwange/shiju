//! This module provides the Inceptron provider, wrapping OpenAI Chat Completions for Inceptron requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    InceptronProviderSettings,
    InceptronProviderSettingsBuilder,
    "Inceptron",
    "https://api.inceptron.io/v1",
    "INCEPTRON_API_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(
    Inceptron,
    InceptronBuilder,
    InceptronProviderSettings,
    "inceptron"
);

// Generate the language model implementation
crate::openai_compatible_language_model!(Inceptron);
