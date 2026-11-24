//! This module provides the Impossibl provider, wrapping OpenAI Chat Completions for Impossibl requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    ImpossiblProviderSettings,
    ImpossiblProviderSettingsBuilder,
    "Impossibl",
    "https://api.impossibl.com/v1",
    "IMPOSSIBL_API_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(
    Impossibl,
    ImpossiblBuilder,
    ImpossiblProviderSettings,
    "impossibl"
);

// Generate the language model implementation
crate::openai_compatible_language_model!(Impossibl);
