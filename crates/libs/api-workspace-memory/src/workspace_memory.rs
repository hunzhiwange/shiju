//! OpenClaw workspace wire types. These names do not grant filesystem access;
//! the execution host must still enforce authorization and regular-file boundaries.

use std::{fmt, str::FromStr};

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

/// The complete workspace file whitelist; wire values are case sensitive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WorkspaceFilename {
    #[serde(rename = "AGENTS.md")]
    Agents,
    #[serde(rename = "SOUL.md")]
    Soul,
    #[serde(rename = "USER.md")]
    User,
    #[serde(rename = "IDENTITY.md")]
    Identity,
    #[serde(rename = "TOOLS.md")]
    Tools,
    #[serde(rename = "MEMORY.md")]
    Memory,
    #[serde(rename = "HEARTBEAT.md")]
    Heartbeat,
    #[serde(rename = "BOOTSTRAP.md")]
    Bootstrap,
    #[serde(rename = "BOOT.md")]
    Boot,
}

impl WorkspaceFilename {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Agents => "AGENTS.md",
            Self::Soul => "SOUL.md",
            Self::User => "USER.md",
            Self::Identity => "IDENTITY.md",
            Self::Tools => "TOOLS.md",
            Self::Memory => "MEMORY.md",
            Self::Heartbeat => "HEARTBEAT.md",
            Self::Bootstrap => "BOOTSTRAP.md",
            Self::Boot => "BOOT.md",
        }
    }
}

impl FromStr for WorkspaceFilename {
    type Err = &'static str;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "AGENTS.md" => Ok(Self::Agents),
            "SOUL.md" => Ok(Self::Soul),
            "USER.md" => Ok(Self::User),
            "IDENTITY.md" => Ok(Self::Identity),
            "TOOLS.md" => Ok(Self::Tools),
            "MEMORY.md" => Ok(Self::Memory),
            "HEARTBEAT.md" => Ok(Self::Heartbeat),
            "BOOTSTRAP.md" => Ok(Self::Bootstrap),
            "BOOT.md" => Ok(Self::Boot),
            _ => Err("invalid workspace filename"),
        }
    }
}

/// A single YYYY-MM-DD.md filename, with a valid Gregorian date (years 0001–9999).
/// The private value ensures both Rust constructors and serde enforce the boundary.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct DailyMemoryFilename(String);

impl DailyMemoryFilename {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn date(&self) -> &str {
        &self.0[..10]
    }
}

impl TryFrom<String> for DailyMemoryFilename {
    type Error = &'static str;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let bytes = value.as_bytes();
        let valid_shape = bytes.len() == 13
            && bytes[4] == b'-'
            && bytes[7] == b'-'
            && &bytes[10..] == b".md"
            && bytes[..10].iter().enumerate().all(|(i, b)| i == 4 || i == 7 || b.is_ascii_digit());
        if !valid_shape
            || &bytes[..4] == b"0000"
            || NaiveDate::parse_from_str(&value[..10], "%Y-%m-%d").is_err()
        {
            return Err("invalid daily memory filename: expected Gregorian YYYY-MM-DD.md");
        }
        Ok(Self(value))
    }
}

impl FromStr for DailyMemoryFilename {
    type Err = &'static str;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::try_from(value.to_owned())
    }
}

impl From<DailyMemoryFilename> for String {
    fn from(value: DailyMemoryFilename) -> Self {
        value.0
    }
}

impl fmt::Display for DailyMemoryFilename {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceDirectory {
    Workspace,
    Memory,
}

/// `None` is a missing file (JSON null); `Some("")` is an existing empty file.
/// IO failures and permission denials must use the API error path, never null.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct WorkspaceFileContent(pub Option<String>);

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkspaceWriteRequest {
    pub content: String,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DailyMemoryFileInfo {
    pub filename: DailyMemoryFilename,
    /// YYYY-MM-DD, matching filename.date().
    pub date: String,
    pub size_bytes: u64,
    /// Whole seconds since the Unix epoch, not milliseconds.
    pub modified_at: u64,
    pub preview: String,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DailyMemorySearchResult {
    pub filename: DailyMemoryFilename,
    pub date: String,
    pub size_bytes: u64,
    /// Whole seconds since the Unix epoch, not milliseconds.
    pub modified_at: u64,
    pub snippet: String,
    pub match_count: u64,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DailyMemorySearchRequest {
    pub query: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkspaceOpenDirectoryRequest {
    pub subdir: WorkspaceDirectory,
}
