use std::{collections::HashMap, path::PathBuf};

use app_models::media_capabilities::{
    MediaCapability as Cap, MediaDeployment, MediaDeploymentEndpoints, MediaEndpointGrant,
    MediaModelBinding, project_media_catalog,
};
use llm_runtime::aisdk::media::{MediaEndpoint, MediaProtocol, credential, deployment_url};
use sha2::{Digest, Sha256};

use super::{Code, MediaModel, MediaOperation};

pub(super) const SHIJU_PROVIDER_ID: &str = "shiju";

pub(super) struct Route {
    pub provider_id: String,
    pub model: MediaModel,
    pub capability: Cap,
    pub endpoint: MediaEndpoint,
}

pub(super) async fn routes(
    tenant: vw_api_transport::TenantScope,
    directory: String,
    selected: Option<(String, String)>,
) -> Result<Vec<Route>, Code> {
    crate::session::ui_store::with_tenant_scope(
        tenant,
        crate::gateway::instance::with_instance(directory, move || {
            Box::pin(async move { Ok(project_routes(selected).await) })
        }),
    )
    .await
    .map_err(|_| Code::Unavailable)?
}

async fn project_routes(selected: Option<(String, String)>) -> Result<Vec<Route>, Code> {
    let providers = crate::provider::provider::list().await.map_err(|_| Code::Unavailable)?;
    let catalog = provider_model_cache::get().await;
    let mut routes = Vec::new();
    for (provider_id, provider) in providers {
        let Some(bearer) = credential(&provider) else { continue };
        let managed = provider_id == SHIJU_PROVIDER_ID;
        let mut account_grants = HashMap::new();
        for (model_id, model) in &provider.models {
            if !compatible(model) || model.provider_id != provider.id || model.api.id.is_empty() {
                continue;
            }
            let Some(base) = deployment_url(&model.api.url) else { continue };
            let deployment_id = deployment_id(&provider_id, &base);
            if selected.as_ref().is_some_and(|(id, _)| *id != deployment_id) {
                continue;
            }

            // The exact account and configured API base authorize managed media endpoints.
            let advertised = if managed {
                if !account_grants.contains_key(&base) {
                    let found =
                        provider_model_discovery::discover_media_endpoint_types(&base, &bearer)
                            .await
                            .map_err(|_| Code::Unavailable);
                    account_grants.insert(base.clone(), found);
                }
                match &account_grants[&base] {
                    Ok(grants) => grants.get(&model.api.id).map(Vec::as_slice),
                    Err(code) if selected.is_some() => return Err(*code),
                    Err(_) => continue,
                }
            } else {
                None
            };
            let configured =
                model.options.get("media_endpoint_types").and_then(serde_json::Value::as_array);
            let endpoint_types = if managed {
                advertised.unwrap_or_default().iter().map(String::as_str).collect()
            } else if let Some(configured) = configured {
                configured.iter().filter_map(serde_json::Value::as_str).collect()
            } else if provider_id == "openai"
                && base.host_str() == Some("api.openai.com")
                && base.path().trim_end_matches('/') == "/v1"
                && catalog
                    .get(&provider_id)
                    .and_then(|provider| provider.models.get(model_id))
                    .is_some_and(|metadata| metadata.modalities.is_some())
            {
                vec![
                    "image-generation",
                    "image-edit",
                    "openai-video",
                    "audio-speech",
                    "audio-transcription",
                    "chat-completions",
                ]
            } else {
                direct_chat_evidence(model, &base).into_iter().collect()
            };

            let binding = MediaModelBinding {
                provider: &provider_id,
                deployment: &deployment_id,
                model: model_id,
                credential_binding: &deployment_id,
            };
            let endpoints = if managed {
                MediaDeploymentEndpoints::Managed { supported_endpoint_types: advertised }
            } else {
                MediaDeploymentEndpoints::Direct
            };
            for endpoint_type in endpoint_types {
                let Some(protocol) = MediaProtocol::from_endpoint_type(endpoint_type) else {
                    continue;
                };
                let Some(endpoint) =
                    MediaEndpoint::new(protocol, &provider, model, base.clone(), bearer.clone())
                else {
                    continue;
                };
                let mut capabilities = match protocol {
                    MediaProtocol::Image => vec![Cap::ImageGeneration],
                    MediaProtocol::ImageEdit => vec![Cap::ImageEdit],
                    MediaProtocol::GeminiChatImage | MediaProtocol::DashScopeImage => {
                        vec![Cap::ImageGeneration, Cap::ImageEdit]
                    }
                    MediaProtocol::Video => {
                        let mut capabilities = vec![Cap::VideoGeneration];
                        if endpoint.supports_image_to_video() {
                            capabilities.push(Cap::ImageToVideo);
                        }
                        capabilities
                    }
                    MediaProtocol::Speech | MediaProtocol::CosyVoice => vec![Cap::TextToSpeech],
                    MediaProtocol::Transcription => vec![Cap::FileAsr],
                    MediaProtocol::ChatVision => {
                        let mut capabilities = Vec::new();
                        if model.capabilities.input.image {
                            capabilities
                                .extend([Cap::ImageUnderstanding, Cap::VideoUnderstandingFrames]);
                        }
                        if protocol.supports_native_video(&provider_id, &model.api.id) {
                            capabilities.push(Cap::VideoUnderstandingNative);
                        }
                        capabilities
                    }
                };
                // A selected deployment can also supply the file ASR needed by frame fallback.
                if selected.as_ref().is_some_and(|(_, id)| id != model_id) {
                    capabilities.retain(|cap| *cap == Cap::FileAsr);
                }
                let grants = [MediaEndpointGrant {
                    binding,
                    endpoint_type,
                    endpoint_id: endpoint_type,
                    capabilities: &capabilities,
                }];
                let deployment = MediaDeployment { binding, endpoints, grants: &grants };
                for capability in &capabilities {
                    if project_media_catalog(&catalog, &[deployment], *capability).is_empty() {
                        continue;
                    }
                    routes.push(Route {
                        provider_id: provider_id.clone(),
                        model: MediaModel {
                            deployment_id: deployment_id.clone(),
                            model_id: model_id.clone(),
                            display_name: model.name.clone(),
                            operations: vec![operation(*capability)],
                        },
                        capability: *capability,
                        endpoint: endpoint.clone(),
                    });
                }
            }
        }
    }
    routes.sort_by(|a, b| {
        (&a.model.deployment_id, &a.model.model_id)
            .cmp(&(&b.model.deployment_id, &b.model.model_id))
    });
    let asr_deployments: std::collections::HashSet<String> = routes
        .iter()
        .filter(|route| route.capability == Cap::FileAsr)
        .map(|route| route.model.deployment_id.clone())
        .collect();
    routes.retain(|route| {
        route.capability != Cap::VideoUnderstandingFrames
            || asr_deployments.contains(&route.model.deployment_id)
    });
    Ok(routes)
}

fn compatible(model: &model_types::Model) -> bool {
    matches!(model.api.adapter.as_str(), "openai-compatible" | "openai" | "openai-responses")
        && !model.api.protocols.as_ref().is_some_and(Vec::is_empty)
}

fn direct_chat_evidence(model: &model_types::Model, base: &reqwest::Url) -> Option<&'static str> {
    use model_protocols::{ModelProtocol, ProtocolAuthentication};
    let endpoint = format!("{}/chat/completions", base.as_str().trim_end_matches('/'));
    model
        .api
        .protocols
        .as_ref()?
        .iter()
        .any(|route| {
            route.protocol == ModelProtocol::Chat
                && matches!(route.authentication, ProtocolAuthentication::Bearer {})
                && route.models.contains(&model.api.id)
                && route.url == endpoint
        })
        .then_some("chat-completions")
}

fn deployment_id(provider: &str, base: &reqwest::Url) -> String {
    let digest = Sha256::digest(base.as_str().trim_end_matches('/').as_bytes());
    format!("{provider}:{}", hex::encode(&digest[..8]))
}

pub(super) fn operation(cap: Cap) -> MediaOperation {
    match cap {
        Cap::ImageGeneration => MediaOperation::ImageGenerate,
        Cap::ImageEdit => MediaOperation::ImageEdit,
        Cap::VideoGeneration | Cap::ImageToVideo => MediaOperation::VideoGenerate,
        Cap::TextToSpeech => MediaOperation::SpeechSynthesize,
        Cap::FileAsr | Cap::RealtimeAsr => MediaOperation::AudioTranscribe,
        Cap::ImageUnderstanding => MediaOperation::ImageUnderstand,
        Cap::VideoUnderstandingNative | Cap::VideoUnderstandingFrames => {
            MediaOperation::VideoUnderstand
        }
    }
}

pub(super) fn select(
    routes: Vec<Route>,
    operation_requested: MediaOperation,
    has_inputs: bool,
    deployment_id: Option<&str>,
    model_id: Option<&str>,
) -> Result<(Route, Option<Route>), Code> {
    let matches = |route: &Route| {
        deployment_id.is_none_or(|id| route.model.deployment_id == id)
            && model_id.is_none_or(|id| route.model.model_id == id)
            && operation(route.capability) == operation_requested
    };
    let desired = if operation_requested == MediaOperation::VideoGenerate {
        Some(if has_inputs { Cap::ImageToVideo } else { Cap::VideoGeneration })
    } else if operation_requested == MediaOperation::VideoUnderstand {
        Some(
            if routes
                .iter()
                .any(|route| matches(route) && route.capability == Cap::VideoUnderstandingNative)
            {
                Cap::VideoUnderstandingNative
            } else {
                Cap::VideoUnderstandingFrames
            },
        )
    } else {
        None
    };
    let mut routes = routes;
    let index = routes
        .iter()
        .position(|route| matches(route) && desired.is_none_or(|cap| route.capability == cap))
        .ok_or(Code::Forbidden)?;
    let route = routes.remove(index);
    let asr = if route.capability == Cap::VideoUnderstandingFrames {
        let index = routes
            .iter()
            .position(|asr| {
                asr.capability == Cap::FileAsr
                    && asr.model.deployment_id == route.model.deployment_id
            })
            .ok_or(Code::Unavailable)?;
        Some(routes.remove(index))
    } else {
        None
    };
    Ok((route, asr))
}

pub(super) fn frame_extractor() -> Option<PathBuf> {
    let path = PathBuf::from(std::env::var_os("SHIJU_MEDIA_FFMPEG")?);
    if !path.is_absolute() {
        return None;
    }
    let metadata = path.metadata().ok()?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        if metadata.permissions().mode() & 0o111 == 0 {
            return None;
        }
    }
    metadata.is_file().then_some(path)
}
