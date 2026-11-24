//! This module provides the Modal provider, wrapping OpenAI Chat Completions for Modal requests.

pub mod capabilities;

// Generate the settings module
crate::openai_compatible_settings!(
    ModalProviderSettings,
    ModalProviderSettingsBuilder,
    "Modal",
    "https://inference.us-west.modal.direct/v1",
    "MODAL_PROXY_TOKEN"
);

// Generate the provider struct and builder
crate::openai_compatible_provider!(Modal, ModalBuilder, ModalProviderSettings, "modal");

// Generate the language model implementation
crate::openai_compatible_language_model!(Modal);
