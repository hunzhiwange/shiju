//! This module provides the AtomicChat provider, wrapping OpenAI Chat Completions for AtomicChat requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    AtomicChatProviderSettings,
    AtomicChatProviderSettingsBuilder,
    "AtomicChat",
    "http://127.0.0.1:1337/v1",
    "ATOMIC_CHAT_API_KEY"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(
    AtomicChat,
    AtomicChatBuilder,
    AtomicChatProviderSettings,
    "atomic-chat"
);

// Generate the language model implementation
crate::openai_compatible_language_model!(AtomicChat);
