//! Conservative filtering of parsed migration items, never a write authorization.
//!
//! Call once per normalized domain item (not an ID-keyed catalog). Parse embedded
//! JSON/TOML with the owning domain first; opaque configuration strings are refused.
//! Unknown fields are excluded until their domain defines portable semantics.
use serde_json::{Map, Value};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum MigrationLocation {
    #[default]
    CrossDevice,
    /// The caller must establish matching authorized execution-host identities.
    /// This permits host paths only; it does not authorize credentials or actions.
    SameHost,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExclusionReason {
    CredentialRequiresAccountAuthorization,
    HostPath,
    SystemIntegration,
    ExecutionPermission,
    UnknownField,
    UnsupportedShape,
    DepthLimit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExcludedField {
    /// JSON pointer into the input item, not a filesystem path. Contains no value.
    pub pointer: String,
    pub reason: ExclusionReason,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FilteredItem {
    pub value: Value,
    pub excluded: Vec<ExcludedField>,
}

/// Credentials are always excluded, including on the same host. Any credential
/// reuse must be performed separately by the existing scoped account owner; a
/// migration flag cannot substitute for its explicit authorization protocol.
pub fn filter_portable_item(input: &Value, location: MigrationLocation) -> FilteredItem {
    let mut excluded = Vec::new();
    let value = if input.is_object() {
        filter_value(input, "", location, 0, &mut excluded).unwrap_or(Value::Null)
    } else {
        exclude("", ExclusionReason::UnsupportedShape, &mut excluded);
        Value::Null
    };
    FilteredItem { value, excluded }
}

fn exclude(pointer: &str, reason: ExclusionReason, excluded: &mut Vec<ExcludedField>) {
    excluded.push(ExcludedField { pointer: pointer.to_owned(), reason });
}

fn filter_value(
    value: &Value,
    pointer: &str,
    location: MigrationLocation,
    depth: usize,
    excluded: &mut Vec<ExcludedField>,
) -> Option<Value> {
    if depth > 64 {
        exclude(pointer, ExclusionReason::DepthLimit, excluded);
        return None;
    }
    match value {
        Value::Object(fields) => {
            let mut kept = Map::new();
            for (key, value) in fields {
                let path = format!("{pointer}/{}", key.replace('~', "~0").replace('/', "~1"));
                let normalized = normalize_key(key);
                let in_env = pointer.rsplit('/').next().is_some_and(|key| normalize_key(key) == "env");
                let reason = field_exclusion(&normalized, value, location).or_else(|| {
                    if in_env && !matches!(normalized.as_str(), "anthropicmodel" | "anthropicbaseurl"
                        | "openaibaseurl" | "openaiapibase" | "geminibaseurl") {
                        Some(ExclusionReason::UnknownField)
                    } else {
                        None
                    }
                });
                if let Some(reason) = reason {
                    exclude(&path, reason, excluded);
                } else if let Some(value) = filter_value(value, &path, location, depth + 1, excluded) {
                    kept.insert(key.clone(), value);
                }
            }
            Some(Value::Object(kept))
        }
        Value::Array(values) => Some(Value::Array(
            values.iter().enumerate().filter_map(|(index, value)| {
                filter_value(value, &format!("{pointer}/{index}"), location, depth + 1, excluded)
            }).collect(),
        )),
        Value::String(text) => {
            // Also inspect array entries, URL userinfo/query/fragment, and local
            // path strings hidden below otherwise portable field names.
            if text.contains("://") {
                let safe_url = reqwest::Url::parse(text).is_ok_and(|url| {
                    matches!(url.scheme(), "http" | "https")
                        && url.username().is_empty() && url.password().is_none()
                        && url.query().is_none() && url.fragment().is_none()
                });
                if !safe_url {
                    exclude(pointer, ExclusionReason::CredentialRequiresAccountAuthorization, excluded);
                    return None;
                }
            }
            if location == MigrationLocation::CrossDevice && is_host_path(text) {
                exclude(pointer, ExclusionReason::HostPath, excluded);
                return None;
            }
            Some(value.clone())
        }
        _ => Some(value.clone()),
    }
}

fn normalize_key(key: &str) -> String {
    key.chars().filter(|c| c.is_ascii_alphanumeric()).map(|c| c.to_ascii_lowercase()).collect()
}

fn field_exclusion(key: &str, value: &Value, location: MigrationLocation) -> Option<ExclusionReason> {
    use ExclusionReason::*;
    if ["token", "secret", "password", "credential", "apikey", "privatekey", "authorization", "cookie"]
        .iter().any(|part| key.contains(part))
        || matches!(key, "auth" | "headers" | "username" | "accountid" | "accountref")
    {
        return Some(CredentialRequiresAccountAuthorization);
    }
    if matches!(key, "permissions" | "permissionmode" | "approvalpolicy" | "sandbox"
        | "sandboxmode" | "allowedtools" | "autoapprove" | "dangerouslyskippermissions"
        | "command" | "args" | "enabled" | "iscurrent" | "infailoverqueue")
        || key.starts_with("enabled") || key.starts_with("currentprofileid")
    {
        return Some(ExecutionPermission);
    }
    if matches!(key, "launchonstartup" | "silentstartup" | "showintray" | "minimizetotrayonclose"
        | "enableclaudepluginintegration" | "skipclaudeonboarding" | "enablelocalproxy"
        | "proxyconfirmed" | "preferredterminal" | "wslshells" | "systemintegration")
    {
        return Some(SystemIntegration);
    }
    if matches!(key, "path" | "directory" | "cwd" | "home" | "configdir" | "filepath") {
        return if location == MigrationLocation::SameHost && value.is_string() {
            None
        } else {
            Some(HostPath)
        };
    }
    if matches!(key, "settingsconfig" | "serverconfig" | "providers" | "mcp" | "mcpservers"
        | "env" | "meta" | "payload" | "settings")
    {
        let structured = value.is_object() || (matches!(key, "providers" | "mcpservers")
            && value.as_array().is_some_and(|items| items.iter().all(Value::is_object)));
        return if structured { None } else { Some(UnsupportedShape) };
    }
    // This allowlist deliberately excludes arbitrary environment variables,
    // unparsed native config, executable arguments, and unknown future fields.
    if matches!(key, "id" | "apptype" | "name" | "description" | "content" | "model"
        | "basemodel" | "defaultmodel" | "url" | "baseurl" | "endpoint" | "websiteurl"
        | "homepage" | "docs" | "tags" | "category" | "icon" | "iconcolor"
        | "sortindex" | "sortorder" | "createdat" | "updatedat" | "addedat"
        | "repoowner" | "reponame" | "repobranch" | "contenthash" | "type"
        | "theme" | "language" | "locale" | "currency" | "anthropicmodel"
        | "anthropicbaseurl" | "openaibaseurl" | "openaiapibase" | "geminibaseurl")
        && !value.is_object() && !value.is_array()
    {
        return None;
    }
    if matches!(key, "apporder" | "visibleapps") && value.as_array().is_some_and(|items| items.iter().all(Value::is_string)) {
        return None;
    }
    if key == "tags" && value.as_array().is_some_and(|items| items.iter().all(Value::is_string)) {
        return None;
    }
    Some(UnknownField)
}

fn is_host_path(text: &str) -> bool {
    let text = text.trim();
    let bytes = text.as_bytes();
    text.starts_with('/') || text.starts_with('\\') || text.starts_with("~/")
        || text.starts_with("~\\") || text.starts_with("file:")
        || (bytes.len() >= 3 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':'
            && matches!(bytes[2], b'/' | b'\\'))
}
