//! Local-only system speech. Call blocking operations on a background worker.
//! Input is the existing capture::Recording::pcm (mono 24 kHz PCM16).
//! The desktop macOS main run loop must be active during synthesis.
//! No API requests permission; use VoicePermissions' explicit user action.
use super::permissions::PermissionStatus;
use std::{
    fmt,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

#[cfg(target_os = "macos")]
#[path = "local_macos.rs"]
mod native;

pub const SAMPLE_RATE: u32 = 24_000;
pub const MAX_RECOGNITION_SECONDS: usize = 60;
pub const MAX_SYNTHESIS_SECONDS: usize = 120;
pub const MAX_TEXT_CHARS: usize = 2_000;
pub type Result<T> = std::result::Result<T, LocalSpeechError>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LocalSpeechError {
    UnsupportedPlatform,
    UnsupportedLanguage,
    OfflineUnavailable,
    Permission(PermissionStatus),
    Unavailable,
    InvalidAudio,
    UnsupportedSynthesisFormat,
    AppMetadata,
    InvalidText,
    VoiceUnavailable,
    Cancelled,
    Timeout,
    Limit,
    EmptyResult,
    Storage,
    System(i64),
}
impl fmt::Display for LocalSpeechError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::UnsupportedPlatform => "Local system speech is supported on macOS only",
            Self::UnsupportedLanguage => "Unsupported language; choose a supported BCP-47 language",
            Self::OfflineUnavailable => {
                "No installed on-device recognition for this language; check macOS Keyboard > Dictation language resources. Network recognition is disabled"
            }
            Self::Permission(_) => {
                "Speech recognition permission required; request authorization for ShiJu, then return from System Settings and retry"
            }
            Self::Unavailable => {
                "Local speech service unavailable; check macOS speech settings and retry"
            }
            Self::InvalidAudio => "Expected non-empty mono 24 kHz PCM16 audio, at most 60 seconds",
            Self::UnsupportedSynthesisFormat => {
                "The installed voice returned an unsupported audio format; choose another Apple system voice"
            }
            Self::AppMetadata => {
                "Local recognition requires an updated ShiJu application bundle with a Speech privacy description; rebuild or reinstall ShiJu"
            }
            Self::InvalidText => "Expected non-empty speech text, at most 2000 characters",
            Self::VoiceUnavailable => {
                "Selected Apple system voice is not installed; choose an available local voice"
            }
            Self::Cancelled => "Local speech cancelled",
            Self::Timeout => "Local speech timed out and was stopped",
            Self::Limit => "Local speech output exceeded its bounded limit",
            Self::EmptyResult => "Local speech returned no usable output",
            Self::Storage => {
                "Cannot create private temporary audio; check free disk space and temporary-directory access"
            }
            Self::System(_) => {
                "Local speech service failed; check installed language resources and system speech settings"
            }
        };
        f.write_str(message)
    }
}
impl std::error::Error for LocalSpeechError {}

/// One token per operation. Cancellation never grants permission or starts IO.
#[derive(Clone, Default)]
pub struct Cancellation(Arc<AtomicBool>);
impl Cancellation {
    pub fn cancel(&self) {
        self.0.store(true, Ordering::Release);
    }
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
    pub(super) fn check(&self) -> Result<()> {
        if self.is_cancelled() { Err(LocalSpeechError::Cancelled) } else { Ok(()) }
    }
    #[cfg(any(target_os = "macos", test))]
    pub(super) fn complete<T>(&self, result: Result<T>) -> Result<T> {
        self.check()?;
        result
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SystemVoice {
    pub id: String,
    pub name: String,
    pub language: String,
}
#[derive(Clone, Debug)]
pub struct Capabilities {
    /// Resolved system locale when the caller supplied None. No language detection.
    pub language: String,
    pub supports_on_device: bool,
    pub available: bool,
    pub permission: PermissionStatus,
    pub voices: Vec<SystemVoice>,
}
/// Debug deliberately excludes recognized text.
pub struct Transcript {
    pub text: String,
    pub language: String,
}
impl fmt::Debug for Transcript {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Transcript")
            .field("language", &self.language)
            .field("characters", &self.text.chars().count())
            .finish()
    }
}
/// Bounded mono 24 kHz PCM16 for existing voice::playback::Playback::push_pcm (convert samples to little-endian bytes).
pub struct SpeechAudio {
    pub pcm: Vec<i16>,
}
impl fmt::Debug for SpeechAudio {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SpeechAudio")
            .field("sample_rate", &SAMPLE_RATE)
            .field("sample_count", &self.pcm.len())
            .finish()
    }
}

pub(super) fn language(value: Option<&str>) -> Result<Option<String>> {
    value
        .map(|s| {
            let raw = s.trim();
            if raw.len() > 35 {
                return Err(LocalSpeechError::UnsupportedLanguage);
            }
            let s = raw.replace('_', "-");
            if s.is_empty()
                || s.len() > 35
                || !s.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-')
                || s.split('-').any(str::is_empty)
            {
                return Err(LocalSpeechError::UnsupportedLanguage);
            }
            Ok(s)
        })
        .transpose()
}
#[cfg(any(target_os = "macos", test))]
pub(super) fn recognition_gate(capabilities: &Capabilities) -> Result<()> {
    if !capabilities.supports_on_device {
        return Err(LocalSpeechError::OfflineUnavailable);
    }
    if capabilities.permission != PermissionStatus::Granted {
        return Err(LocalSpeechError::Permission(capabilities.permission));
    }
    if !capabilities.available {
        return Err(LocalSpeechError::Unavailable);
    }
    Ok(())
}

/// Read-only; does not request permission or install/download resources.
pub fn capabilities(locale: Option<&str>) -> Result<Capabilities> {
    let locale = language(locale)?;
    #[cfg(target_os = "macos")]
    {
        native::capabilities(locale.as_deref())
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = locale;
        Err(LocalSpeechError::UnsupportedPlatform)
    }
}
/// Installed Apple voices only, independent of recognition language/permission.
pub fn voices() -> Result<Vec<SystemVoice>> {
    #[cfg(target_os = "macos")]
    {
        native::voices()
    }
    #[cfg(not(target_os = "macos"))]
    {
        Err(LocalSpeechError::UnsupportedPlatform)
    }
}
/// Existing capture::Recording.pcm can be passed directly. None uses the system
/// speech locale, not automatic language detection. Never falls back to a server.
pub fn transcribe_pcm(
    pcm: &[i16],
    locale: Option<&str>,
    cancel: &Cancellation,
) -> Result<Transcript> {
    cancel.check()?;
    if pcm.is_empty() || pcm.len() > SAMPLE_RATE as usize * MAX_RECOGNITION_SECONDS {
        return Err(LocalSpeechError::InvalidAudio);
    }
    let locale = language(locale)?;
    #[cfg(target_os = "macos")]
    {
        native::transcribe(pcm, locale.as_deref(), cancel)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = locale;
        Err(LocalSpeechError::UnsupportedPlatform)
    }
}
/// Generates local PCM; it does not play audio. IDs must come from capabilities().
pub fn synthesize(text: &str, voice: Option<&str>, cancel: &Cancellation) -> Result<SpeechAudio> {
    cancel.check()?;
    if text.trim().is_empty() || text.chars().count() > MAX_TEXT_CHARS || text.contains('\0') {
        return Err(LocalSpeechError::InvalidText);
    }
    #[cfg(target_os = "macos")]
    {
        native::synthesize(text, voice, cancel)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = voice;
        Err(LocalSpeechError::UnsupportedPlatform)
    }
}

/// Streaming area resampling, independent of callback chunk boundaries.
#[cfg(any(target_os = "macos", test))]
#[derive(Default)]
pub(super) struct PcmOutput {
    pub(super) pcm: Vec<i16>,
    rate: u32,
    remaining: u32,
    sum: f64,
}
#[cfg(any(target_os = "macos", test))]
impl PcmOutput {
    pub(super) fn push(&mut self, sample: f64, rate: u32) -> Result<()> {
        if !(8_000..=192_000).contains(&rate) {
            return Err(LocalSpeechError::InvalidAudio);
        }
        if self.rate == 0 {
            self.rate = rate;
            self.remaining = rate;
        }
        if self.rate != rate {
            return Err(LocalSpeechError::InvalidAudio);
        }
        let sample = if sample.is_finite() { sample.clamp(-1.0, 1.0) } else { 0.0 };
        let mut width = SAMPLE_RATE;
        while width > 0 {
            let take = width.min(self.remaining);
            self.sum += sample * take as f64;
            self.remaining -= take;
            width -= take;
            if self.remaining == 0 {
                if self.pcm.len() >= SAMPLE_RATE as usize * MAX_SYNTHESIS_SECONDS {
                    return Err(LocalSpeechError::Limit);
                }
                self.pcm.push((self.sum / self.rate as f64 * i16::MAX as f64).round() as i16);
                self.sum = 0.0;
                self.remaining = self.rate;
            }
        }
        Ok(())
    }
}

#[cfg(all(test, target_os = "macos"))]
pub(super) fn offline_request_probe() -> Result<bool> {
    native::offline_request_probe()
}
