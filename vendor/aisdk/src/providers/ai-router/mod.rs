//! This module provides the AiRouter provider, wrapping OpenAI Chat Completions for AiRouter requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    AiRouterProviderSettings,
    AiRouterProviderSettingsBuilder,
    "AiRouter",
    "https://api.ai-router.dev/v1",
    "AI_ROUTER_API_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(
    AiRouter,
    AiRouterBuilder,
    AiRouterProviderSettings,
    "ai-router"
);

// Generate the language model implementation
crate::openai_compatible_language_model!(AiRouter);
