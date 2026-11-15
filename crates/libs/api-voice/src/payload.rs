use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde::{Deserialize, Serialize};

pub const MAX_AUDIO_SAMPLES: usize = 2400;
pub const MAX_AUDIO_BYTES: usize = 4800;
/// The sole negotiated media format in v1: raw little-endian PCM16, mono, 24 kHz.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AudioFormat {
    pub encoding: AudioEncoding,
    #[serde(deserialize_with = "sample_rate_24k")]
    pub sample_rate: u32,
    #[serde(deserialize_with = "mono_channels")]
    pub channels: u8,
}
impl Default for AudioFormat {
    fn default() -> Self {
        Self { encoding: AudioEncoding::PcmS16le, sample_rate: 24000, channels: 1 }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AudioEncoding {
    PcmS16le,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "UncheckedAudio")]
pub struct AudioPayload {
    pub audio: String,
    pub samples_per_channel: usize,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct UncheckedAudio {
    audio: String,
    samples_per_channel: usize,
}
impl TryFrom<UncheckedAudio> for AudioPayload {
    type Error = &'static str;
    fn try_from(raw: UncheckedAudio) -> Result<Self, Self::Error> {
        validate_audio(&raw.audio, raw.samples_per_channel)?;
        Ok(Self { audio: raw.audio, samples_per_channel: raw.samples_per_channel })
    }
}
pub(crate) fn validate_audio(audio: &str, samples: usize) -> Result<(), &'static str> {
    if samples == 0 || samples > MAX_AUDIO_SAMPLES || audio.len() > MAX_AUDIO_BYTES / 3 * 4 {
        return Err("voice audio frame exceeds sample or byte limit");
    }
    let bytes = STANDARD.decode(audio).map_err(|_| "invalid voice audio base64")?;
    if bytes.len() != samples * 2 {
        return Err("voice audio sample count mismatch");
    }
    Ok(())
}

/// One in-memory PNG, with no URLs, OCR instructions or permission changes.
pub const MAX_SCREEN_BYTES: usize = 40 * 1024;
pub const MAX_SCREEN_EDGE: u32 = 1024;
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScreenPayload {
    pub png: String,
}
impl std::fmt::Debug for ScreenPayload {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ScreenPayload([redacted])")
    }
}
impl ScreenPayload {
    pub fn bytes(&self) -> Result<Vec<u8>, &'static str> {
        if self.png.len() > MAX_SCREEN_BYTES.div_ceil(3) * 4 {
            return Err("screen frame exceeds byte limit");
        }
        let bytes = STANDARD.decode(&self.png).map_err(|_| "invalid screen base64")?;
        if bytes.len() < 33
            || bytes.len() > MAX_SCREEN_BYTES
            || &bytes[..8] != b"\x89PNG\r\n\x1a\n"
            || &bytes[8..16] != b"\0\0\0\rIHDR"
        {
            return Err("invalid screen PNG");
        }
        let width = u32::from_be_bytes(bytes[16..20].try_into().unwrap());
        let height = u32::from_be_bytes(bytes[20..24].try_into().unwrap());
        if width == 0 || height == 0 || width > MAX_SCREEN_EDGE || height > MAX_SCREEN_EDGE {
            return Err("screen dimensions exceed limit");
        }
        Ok(bytes)
    }
}
fn sample_rate_24k<'de, D: serde::Deserializer<'de>>(d: D) -> Result<u32, D::Error> {
    let rate = u32::deserialize(d)?;
    if rate != 24000 {
        return Err(serde::de::Error::custom("unsupported voice sample rate"));
    }
    Ok(rate)
}
fn mono_channels<'de, D: serde::Deserializer<'de>>(d: D) -> Result<u8, D::Error> {
    let channels = u8::deserialize(d)?;
    if channels != 1 {
        return Err(serde::de::Error::custom("unsupported voice channel count"));
    }
    Ok(channels)
}
