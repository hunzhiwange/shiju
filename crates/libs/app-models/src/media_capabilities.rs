//! Project executable media routes from catalog metadata and scoped deployment grants.

use std::collections::{HashMap, HashSet};

use provider_model_catalog::{ModelModalities, Provider};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaCapability {
    ImageGeneration,
    ImageEdit,
    VideoGeneration,
    ImageToVideo,
    TextToSpeech,
    FileAsr,
    RealtimeAsr,
    ImageUnderstanding,
    VideoUnderstandingNative,
    VideoUnderstandingFrames,
}

impl MediaCapability {
    fn modalities(self) -> (&'static str, &'static str) {
        match self {
            Self::ImageGeneration => ("text", "image"),
            Self::ImageEdit => ("image", "image"),
            Self::VideoGeneration => ("text", "video"),
            Self::ImageToVideo => ("image", "video"),
            Self::TextToSpeech => ("text", "audio"),
            Self::FileAsr | Self::RealtimeAsr => ("audio", "text"),
            Self::ImageUnderstanding | Self::VideoUnderstandingFrames => ("image", "text"),
            Self::VideoUnderstandingNative => ("video", "text"),
        }
    }
}

/// The exact deployment identity. The credential binding is an opaque ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MediaModelBinding<'a> {
    pub provider: &'a str,
    pub deployment: &'a str,
    pub model: &'a str,
    pub credential_binding: &'a str,
}

#[derive(Debug, Clone, Copy)]
pub enum MediaDeploymentEndpoints<'a> {
    /// The account's per-model endpoint types. A missing list supplies no endpoint evidence.
    Managed { supported_endpoint_types: Option<&'a [String]> },
    /// Endpoint evidence comes from explicitly validated grants on the configured deployment.
    Direct,
}

/// The caller supplies a grant after authorizing the exact endpoint and credential binding
/// and verifying the adapter supports these operations through that endpoint type.
/// Frame-based understanding also requires a usable frame extractor.
#[derive(Debug, Clone, Copy)]
pub struct MediaEndpointGrant<'a> {
    pub binding: MediaModelBinding<'a>,
    pub endpoint_type: &'a str,
    /// Opaque reference to the resolved endpoint and authentication configuration.
    pub endpoint_id: &'a str,
    pub capabilities: &'a [MediaCapability],
}

#[derive(Debug, Clone, Copy)]
pub struct MediaDeployment<'a> {
    pub binding: MediaModelBinding<'a>,
    pub endpoints: MediaDeploymentEndpoints<'a>,
    pub grants: &'a [MediaEndpointGrant<'a>],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MediaModelOption<'a> {
    pub binding: MediaModelBinding<'a>,
    pub endpoint_type: &'a str,
    pub endpoint_id: &'a str,
}

/// Keep catalog declarations separate from callability. An unknown declaration can
/// be covered by an explicit grant. A present declaration must support the requested pair.
/// Results follow deployment and grant order, with one result per bound endpoint.
pub fn project_media_catalog<'a>(
    catalog: &HashMap<String, Provider>,
    deployments: &[MediaDeployment<'a>],
    capability: MediaCapability,
) -> Vec<MediaModelOption<'a>> {
    let (input, output) = capability.modalities();
    let mut options = Vec::new();
    let mut seen = HashSet::new();
    for deployment in deployments {
        let binding = deployment.binding;
        if [binding.provider, binding.deployment, binding.model, binding.credential_binding]
            .iter()
            .any(|value| value.trim().is_empty())
            || declared_modalities(catalog, binding)
                .is_some_and(|modalities| !modalities.supports_pair(input, output))
        {
            continue;
        }
        for grant in deployment.grants {
            if grant.binding != binding
                || grant.endpoint_type.trim().is_empty()
                || grant.endpoint_id.trim().is_empty()
                || !grant.capabilities.contains(&capability)
            {
                continue;
            }
            if let MediaDeploymentEndpoints::Managed { supported_endpoint_types } =
                deployment.endpoints
                && !supported_endpoint_types.is_some_and(|types| {
                    types.iter().any(|endpoint_type| endpoint_type.as_str() == grant.endpoint_type)
                })
            {
                continue;
            }
            let option = MediaModelOption {
                binding,
                endpoint_type: grant.endpoint_type,
                endpoint_id: grant.endpoint_id,
            };
            if seen.insert(option) {
                options.push(option);
            }
        }
    }
    options
}

fn declared_modalities<'catalog>(
    catalog: &'catalog HashMap<String, Provider>,
    binding: MediaModelBinding<'_>,
) -> Option<&'catalog ModelModalities> {
    let model = catalog.get(binding.provider)?.models.get(binding.model)?;
    if let Some(modalities) = &model.modalities {
        return Some(modalities);
    }

    let (vendor, model_id) = model.canonical_model_id.as_deref()?.split_once('/')?;
    if vendor.is_empty() || model_id.is_empty() || vendor == "shiju" {
        return None;
    }
    catalog.get(vendor)?.models.get(model_id)?.modalities.as_ref()
}
