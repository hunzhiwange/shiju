//! Shared wire types and input rules for usage settings.

use serde::{Deserialize, Serialize};
use uuid::{Uuid, Variant, Version};

pub const MAX_REFRESH_INTERVAL_MINUTES: u16 = 24 * 60;

/// An opaque reference issued for a host-managed credential.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct UsageCredentialId(Uuid);

impl TryFrom<String> for UsageCredentialId {
    type Error = &'static str;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let error = "invalid usage credential reference";
        let id = Uuid::parse_str(&value).map_err(|_| error)?;
        if id.get_version() != Some(Version::Random) || id.get_variant() != Variant::RFC4122 {
            return Err(error);
        }
        Ok(Self(id))
    }
}

impl From<UsageCredentialId> for String {
    fn from(value: UsageCredentialId) -> Self {
        value.0.to_string()
    }
}

/// Explicit overrides for Coding Plan detection. An absent override uses automatic detection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum UsageCodingPlan {
    ZhipuTeam { organization_id: String, project_id: String },
    Volcengine,
}

pub fn valid_usage_team_id(value: &str) -> bool {
    !value.is_empty()
        && value == value.trim()
        && value.bytes().all(|byte| byte == b' ' || byte.is_ascii_graphic())
}
