//! This module provides the Clarifai provider, wrapping OpenAI Chat Completions for Clarifai requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    ClarifaiProviderSettings,
    ClarifaiProviderSettingsBuilder,
    "Clarifai",
    "https://api.clarifai.com/v2/ext/openai/v1",
    "CLARIFAI_PAT"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(
    Clarifai,
    ClarifaiBuilder,
    ClarifaiProviderSettings,
    "clarifai"
);

// Generate the language model implementation
crate::openai_compatible_language_model!(Clarifai);
