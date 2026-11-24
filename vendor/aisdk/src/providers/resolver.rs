//! Resolve complete model references into OpenAI-compatible providers.

use crate::core::{DynamicModel, ModelRef};
use crate::{Error, Result};

use super::{OpenAICompatible, provider_descriptor};

/// Explicit overrides used while resolving an OpenAI-compatible model.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct OpenAICompatibleResolveOptions {
    /// API key to use instead of a registered provider environment variable.
    pub api_key: Option<String>,
    /// API base URL to use instead of the registered provider URL.
    pub base_url: Option<String>,
    /// Optional Chat Completions request path override.
    pub path: Option<String>,
}

/// Resolve a complete `provider/model` reference into a configured provider.
///
/// Registered providers inherit their base URL and API key environment
/// variables from the generated registry. Unknown providers require an
/// explicit base URL. Model identifiers are passed through unchanged and are
/// not checked against a static model catalog.
pub fn resolve_openai_compatible_model(
    model_ref: &str,
    options: OpenAICompatibleResolveOptions,
) -> Result<OpenAICompatible<DynamicModel>> {
    resolve_openai_compatible_model_with_env(model_ref, options, |name| std::env::var(name).ok())
}

pub(super) fn resolve_openai_compatible_model_with_env<F>(
    model_ref: &str,
    options: OpenAICompatibleResolveOptions,
    mut env_lookup: F,
) -> Result<OpenAICompatible<DynamicModel>>
where
    F: FnMut(&str) -> Option<String>,
{
    let model_ref = ModelRef::parse(model_ref)?;
    let descriptor = provider_descriptor(&model_ref.provider_id);

    let base_url = options
        .base_url
        .or_else(|| descriptor.map(|registered| registered.base_url.to_owned()))
        .ok_or_else(|| {
            Error::InvalidInput(format!(
                "unknown provider `{}` requires an explicit base_url",
                model_ref.provider_id
            ))
        })?;

    let api_key = options.api_key.unwrap_or_else(|| {
        descriptor
            .and_then(|registered| {
                registered
                    .env_names
                    .iter()
                    .find_map(|name| env_lookup(name).filter(|value| !value.is_empty()))
            })
            .unwrap_or_default()
    });

    let mut builder = OpenAICompatible::<DynamicModel>::builder()
        .provider_name(model_ref.provider_id)
        .base_url(base_url)
        .api_key(api_key)
        .model_name(model_ref.model_id);

    if let Some(path) = options.path {
        builder = builder.path(path);
    }

    builder.build()
}
