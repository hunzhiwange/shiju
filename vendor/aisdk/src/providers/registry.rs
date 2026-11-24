//! Runtime metadata for generated OpenAI-compatible providers.

use super::generated_registry::PROVIDERS;

/// The wire protocol implemented by a provider.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProviderProtocol {
    /// OpenAI Chat Completions compatible protocol.
    OpenAiCompatible,
}

/// Static metadata needed to construct a provider at runtime.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProviderDescriptor {
    /// Stable models.dev provider ID.
    pub id: &'static str,
    /// Provider API base URL.
    pub base_url: &'static str,
    /// Environment variables that may contain the provider API key.
    pub env_names: &'static [&'static str],
    /// Wire protocol implemented by the provider.
    pub protocol: ProviderProtocol,
}

/// Return the descriptor registered for `provider_id`.
pub fn provider_descriptor(provider_id: &str) -> Option<&'static ProviderDescriptor> {
    PROVIDERS
        .iter()
        .find(|descriptor| descriptor.id == provider_id)
}

/// Return all provider descriptors enabled by the current Cargo features.
pub fn provider_descriptors() -> &'static [ProviderDescriptor] {
    PROVIDERS
}
