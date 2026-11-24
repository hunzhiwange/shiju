//! This module provides the Crossmodel provider, wrapping OpenAI Chat Completions for Crossmodel requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    CrossmodelProviderSettings,
    CrossmodelProviderSettingsBuilder,
    "Crossmodel",
    "https://api.crossmodel.ai/v1",
    "CROSSMODEL_API_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(
    Crossmodel,
    CrossmodelBuilder,
    CrossmodelProviderSettings,
    "crossmodel"
);

// Generate the language model implementation
crate::openai_compatible_language_model!(Crossmodel);
