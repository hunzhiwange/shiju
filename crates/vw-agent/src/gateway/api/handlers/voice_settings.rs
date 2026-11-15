//! Host integration for voice settings persistence tests.
#[cfg(test)]
use gateway_voice::voice_settings::{validate_patch, save};
#[cfg(test)]
use config_voice::VoiceConfig;
#[cfg(test)]
use axum::{Extension, Json, http::HeaderMap};
#[cfg(test)]
use vw_api_transport::TenantScope;
#[cfg(test)]
#[path = "../../../../../libs/gateway-voice/src/host_tests/voice_settings_tests.rs"]
mod tests;

#[cfg(test)]
use config_voice::VoiceName;
