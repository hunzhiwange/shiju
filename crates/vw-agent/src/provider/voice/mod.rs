//! Gateway adapters for public voice services.
pub mod realtime;
#[cfg(test)]
mod realtime_tests;
pub mod transcription;

use transcription::{TranscriptionError, TranscriptionProvider};
use vw_api_types::voice::VoiceErrorCode;
use vw_config_types::voice::VoiceConfig;

impl TranscriptionProvider {
    /// Resolves only gateway-owned API credentials in the caller's tenant/project scope.
    /// ChatGPT OAuth credentials are deliberately not accepted for the public audio API.
    pub async fn from_config(config: &VoiceConfig) -> Result<Self, TranscriptionError> {
        config
            .validate()
            .map_err(|_| TranscriptionError::Service(VoiceErrorCode::InvalidRequest))?;
        if !config.dictation_enabled {
            return Err(TranscriptionError::Service(VoiceErrorCode::UnsupportedCapability));
        }
        let (base, key) = voice_credentials(config).await?;
        let builder = crate::model_relay::upstream::upstream_http_client_builder();
        let builder = crate::config::apply_runtime_proxy_to_builder(builder, "provider.openai");
        Self::connect(&base, &key, builder, crate::model_relay::upstream::REQUEST_TIMEOUT)
    }
}

#[cfg(test)]
mod transcription_tests;

impl realtime::RealtimeSession {
    /// Caller establishes tenant/project scope before resolving the provider reference.
    pub async fn from_config(
        config: &VoiceConfig,
        voice_session_id: vw_api_types::voice::VoiceId,
        mode: vw_api_types::voice::TurnMode,
    ) -> Result<Self, VoiceErrorCode> {
        config.validate().map_err(|_| VoiceErrorCode::InvalidRequest)?;
        if !config.realtime_enabled {
            return Err(VoiceErrorCode::UnsupportedCapability);
        }
        let (base, key) = voice_credentials(config).await.map_err(|e| match e {
            TranscriptionError::Service(code) => code,
            TranscriptionError::EmptyTranscript => VoiceErrorCode::ProtocolError,
        })?;
        let provider_id = config
            .provider_id
            .clone()
            .ok_or(VoiceErrorCode::ProviderAuth)?
            .try_into()
            .map_err(|_| VoiceErrorCode::InvalidRequest)?;
        let builder = crate::model_relay::upstream::upstream_http_client_builder();
        let builder = crate::config::apply_runtime_proxy_to_builder(builder, "provider.openai");
        Self::connect(&base, &key, builder, config, provider_id, voice_session_id, mode).await
    }
}
async fn voice_credentials(config: &VoiceConfig) -> Result<(String, String), TranscriptionError> {
    let id = config
        .provider_id
        .as_deref()
        .ok_or_else(|| TranscriptionError::Service(VoiceErrorCode::ProviderAuth))?;
    let provider = super::provider::get_provider(id)
        .await
        .map_err(|_| TranscriptionError::Service(VoiceErrorCode::ProviderUnavailable))?
        .ok_or_else(|| TranscriptionError::Service(VoiceErrorCode::ProviderAuth))?;
    let auth = crate::auth::get(id);
    let key = provider
        .key
        .as_deref()
        .filter(|key| vw_provider_resolver::remote_models::is_real_bearer(key))
        .map(str::to_owned)
        .or_else(|| {
            provider
                .env
                .iter()
                .filter_map(|name| crate::env::get(name))
                .find(|key| vw_provider_resolver::remote_models::is_real_bearer(key))
        })
        .or_else(|| match auth {
            Some(crate::auth::Info::Api(api)) => Some(api.key),
            _ => None,
        })
        .filter(|key| vw_provider_resolver::remote_models::is_real_bearer(key))
        .ok_or_else(|| TranscriptionError::Service(VoiceErrorCode::ProviderAuth))?;
    let base = match provider.options.get("baseURL") {
        Some(value) => value
            .as_str()
            .ok_or_else(|| TranscriptionError::Service(VoiceErrorCode::InvalidRequest))?,
        None if id == "openai" => "https://api.openai.com/v1",
        None => return Err(TranscriptionError::Service(VoiceErrorCode::InvalidRequest)),
    };
    Ok((base.to_owned(), key))
}
