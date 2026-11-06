//! Write-only common request options; never return these credential-bearing values.
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::BTreeMap;

pub(crate) fn present<'de, D, T>(d: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    T::deserialize(d).map(Some)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProviderApiKeyField {
    #[serde(rename = "ANTHROPIC_AUTH_TOKEN")]
    AuthToken,
    #[serde(rename = "ANTHROPIC_API_KEY")]
    ApiKey,
}

#[derive(Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProviderRequestOverrides {
    #[serde(
        default,
        deserialize_with = "present_headers",
        skip_serializing_if = "Option::is_none"
    )]
    pub headers: Option<BTreeMap<String, String>>,
    #[serde(default, deserialize_with = "present", skip_serializing_if = "Option::is_none")]
    pub body: Option<serde_json::Map<String, serde_json::Value>>,
}

/// Missing members preserve existing values on update; empty maps explicitly clear them.
/// An empty overrides object clears the entire override; AuthToken restores the default key field.
#[derive(Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProviderRequestOptions {
    #[serde(
        default,
        deserialize_with = "present_headers",
        skip_serializing_if = "Option::is_none"
    )]
    pub request_headers: Option<BTreeMap<String, String>>,
    #[serde(default, deserialize_with = "present", skip_serializing_if = "Option::is_none")]
    pub api_key_field: Option<ProviderApiKeyField>,
    #[serde(default, deserialize_with = "present", skip_serializing_if = "Option::is_none")]
    pub local_proxy_request_overrides: Option<ProviderRequestOverrides>,
}
impl std::fmt::Debug for ProviderRequestOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ProviderRequestOptions([redacted])")
    }
}
impl std::fmt::Debug for ProviderRequestOverrides {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ProviderRequestOverrides([redacted])")
    }
}

// Reject duplicates before a map can discard them; business validation also checks
// values built directly by Rust callers, independently of deserialization.
fn present_headers<'de, D>(d: D) -> Result<Option<BTreeMap<String, String>>, D::Error>
where
    D: Deserializer<'de>,
{
    struct Headers;
    impl<'de> serde::de::Visitor<'de> for Headers {
        type Value = BTreeMap<String, String>;
        fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("an object of unique HTTP header names and string values")
        }
        fn visit_map<A: serde::de::MapAccess<'de>>(
            self,
            mut map: A,
        ) -> Result<Self::Value, A::Error> {
            let mut result = BTreeMap::new();
            while let Some((name, value)) = map.next_entry::<String, String>()? {
                if result.insert(name, value).is_some() {
                    return Err(serde::de::Error::custom("duplicate provider request header"));
                }
            }
            Ok(result)
        }
    }
    d.deserialize_map(Headers).map(Some)
}
