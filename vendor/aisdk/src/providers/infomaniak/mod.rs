//! This module provides the Infomaniak provider, wrapping OpenAI Chat Completions for Infomaniak requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    InfomaniakProviderSettings,
    InfomaniakProviderSettingsBuilder,
    "Infomaniak",
    "https://api.infomaniak.com/2/ai/${INFOMANIAK_PRODUCT_ID}/openai/v1",
    "INFOMANIAK_API_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(
    Infomaniak,
    InfomaniakBuilder,
    InfomaniakProviderSettings,
    "infomaniak"
);

// Generate the language model implementation
crate::openai_compatible_language_model!(Infomaniak);
