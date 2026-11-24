//! This module provides the Pareto provider, wrapping OpenAI Chat Completions for Pareto requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    ParetoProviderSettings,
    ParetoProviderSettingsBuilder,
    "Pareto",
    "https://api.paretoinference.com/v1",
    "PARETO_API_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(Pareto, ParetoBuilder, ParetoProviderSettings, "pareto");

// Generate the language model implementation
crate::openai_compatible_language_model!(Pareto);
