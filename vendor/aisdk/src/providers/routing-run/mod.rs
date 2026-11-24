//! This module provides the RoutingRun provider, wrapping OpenAI Chat Completions for RoutingRun requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    RoutingRunProviderSettings,
    RoutingRunProviderSettingsBuilder,
    "RoutingRun",
    "https://api.routing.run/v1",
    "ROUTING_RUN_API_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(
    RoutingRun,
    RoutingRunBuilder,
    RoutingRunProviderSettings,
    "routing-run"
);

// Generate the language model implementation
crate::openai_compatible_language_model!(RoutingRun);
