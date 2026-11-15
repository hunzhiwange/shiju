//! 语音 HTTP 兼容门面。
use super::GatewayClient;
pub use gateway_voice::VoiceApiError;
use gateway_voice::GatewayVoiceClient;

impl GatewayClient {
    pub async fn voice_settings_patch(&self, patch: &serde_json::Value) -> Result<(), String> {
        GatewayVoiceClient::new(self.transport.clone()).voice_settings_patch(patch).await
    }
    pub async fn voice_preview(&self, voice: &impl serde::Serialize) -> Result<futures_util::stream::BoxStream<'static, Result<Vec<u8>, VoiceApiError>>, VoiceApiError> {
        GatewayVoiceClient::new(self.transport.clone()).voice_preview(voice).await
    }
    pub async fn voice_config<T: serde::de::DeserializeOwned>(&self) -> Result<T, VoiceApiError> {
        GatewayVoiceClient::new(self.transport.clone()).voice_config().await
    }
    pub async fn voice_transcribe(&self, metadata: &vw_api_voice::TranscriptionRequest, audio: Vec<u8>) -> Result<vw_api_voice::TranscriptionResponse, VoiceApiError> {
        GatewayVoiceClient::new(self.transport.clone()).voice_transcribe(metadata, audio).await
    }
    pub async fn voice_cancel_transcription(&self, request_id: &vw_api_voice::VoiceId) -> Result<(), VoiceApiError> {
        GatewayVoiceClient::new(self.transport.clone()).voice_cancel_transcription(request_id).await
    }
}
