//! Gateway-owned voice settings. Physical devices, permissions and recordings stay local.
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const MAX_DICTIONARY_ENTRIES: usize = 100;
pub const MAX_DICTIONARY_ENTRY_CHARS: usize = 80;
pub const MAX_PROMPT_CHARS: usize = 2000;

/// Only the verified public service adapter is selectable in v1.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum VoiceProvider {
    #[default]
    Openai,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum TranscriptionModel {
    #[default]
    #[serde(rename = "gpt-transcribe")]
    GptTranscribe,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum RealtimeModel {
    #[default]
    #[serde(rename = "gpt-realtime-2.1")]
    GptRealtime21,
}

/// Auto omits the upstream languages field; detected language codes remain unrestricted.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RecognitionLanguage {
    #[default]
    Auto,
    En,
    Es,
    Fr,
    Zh,
}
impl RecognitionLanguage {
    pub fn service_code(self) -> Option<&'static str> {
        match self {
            Self::Auto => None,
            Self::En => Some("en"),
            Self::Es => Some("es"),
            Self::Fr => Some("fr"),
            Self::Zh => Some("zh"),
        }
    }
}

/// Static public capability snapshot dated 2026-11-11; changing voice needs a new session.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum VoiceName {
    Alloy,
    Ash,
    Ballad,
    Coral,
    Echo,
    Sage,
    Shimmer,
    Verse,
    #[default]
    Marin,
    Cedar,
}

/// Persisted at app_ui.voice on the gateway. Neither enable flag grants OS permission.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(default, deny_unknown_fields)]
pub struct VoiceConfig {
    pub dictation_enabled: bool,
    pub realtime_enabled: bool,
    pub provider: VoiceProvider,
    /// Existing model_providers key used to resolve credentials on the gateway; never a key itself.
    #[serde(deserialize_with = "deserialize_provider_id")]
    pub provider_id: Option<String>,
    pub transcription_model: TranscriptionModel,
    pub realtime_model: RealtimeModel,
    pub voice: VoiceName,
    pub language: RecognitionLanguage,
    /// Recognition hints only, not replacement rules or executable instructions.
    #[serde(deserialize_with = "deserialize_dictionary")]
    pub dictionary: Vec<String>,
    /// Recognition context only, limited by Unicode scalar count.
    #[serde(deserialize_with = "deserialize_prompt")]
    pub prompt: String,
}

impl VoiceConfig {
    /// Also validate values constructed or edited in Rust before persisting/starting capture.
    pub fn validate(&self) -> Result<(), &'static str> {
        if let Some(id) = &self.provider_id
            && (id.is_empty() || id.len() > 128 || !id.bytes().all(|b| b.is_ascii_graphic()))
        {
            return Err("invalid voice provider reference");
        }
        if self.dictionary.len() > MAX_DICTIONARY_ENTRIES {
            return Err("voice dictionary has too many entries");
        }
        for entry in &self.dictionary {
            if entry.trim().is_empty()
                || entry.chars().count() > MAX_DICTIONARY_ENTRY_CHARS
                || entry.contains(['<', '>', '\r', '\n'])
            {
                return Err("invalid voice dictionary entry");
            }
        }
        if self.prompt.chars().count() > MAX_PROMPT_CHARS {
            return Err("voice prompt is too long");
        }
        Ok(())
    }
}

fn deserialize_provider_id<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> Result<Option<String>, D::Error> {
    let value = Option::<String>::deserialize(d)?;
    VoiceConfig { provider_id: value.clone(), ..Default::default() }
        .validate()
        .map_err(serde::de::Error::custom)?;
    Ok(value)
}
fn deserialize_dictionary<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Vec<String>, D::Error> {
    let value = Vec::<String>::deserialize(d)?;
    VoiceConfig { dictionary: value.clone(), ..Default::default() }
        .validate()
        .map_err(serde::de::Error::custom)?;
    Ok(value)
}
fn deserialize_prompt<'de, D: serde::Deserializer<'de>>(d: D) -> Result<String, D::Error> {
    let value = String::deserialize(d)?;
    if value.chars().count() > MAX_PROMPT_CHARS {
        return Err(serde::de::Error::custom("voice prompt is too long"));
    }
    Ok(value)
}
