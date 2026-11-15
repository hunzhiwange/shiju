use serde::{Deserialize, Serialize};

pub const MAX_SAFE_INTEGER: u64 = (1 << 53) - 1;

/// Opaque ASCII identifier (1–128 printable bytes). Never a URL, credential or device name.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct VoiceId(String);
impl TryFrom<String> for VoiceId {
    type Error = &'static str;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.is_empty() || value.len() > 128 || !value.bytes().all(|b| b.is_ascii_graphic()) {
            return Err("invalid voice identifier");
        }
        Ok(Self(value))
    }
}
impl From<VoiceId> for String {
    fn from(id: VoiceId) -> Self {
        id.0
    }
}
impl AsRef<str> for VoiceId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

/// Both generation and per-direction sequence start at one and fit exactly in JSON clients.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "u64", into = "u64")]
pub struct VoiceCounter(u64);
impl VoiceCounter {
    pub const FIRST: Self = Self(1);
    pub fn get(self) -> u64 {
        self.0
    }
    pub fn checked_next(self) -> Result<Self, &'static str> {
        Self::try_from(self.0 + 1)
    }
}
impl TryFrom<u64> for VoiceCounter {
    type Error = &'static str;
    fn try_from(value: u64) -> Result<Self, Self::Error> {
        if !(1..=MAX_SAFE_INTEGER).contains(&value) {
            return Err("invalid voice counter");
        }
        Ok(Self(value))
    }
}
impl From<VoiceCounter> for u64 {
    fn from(value: VoiceCounter) -> Self {
        value.0
    }
}
