//! Local configuration validation only. This does not grant network access.

use reqwest::Url;

/// Execution-side input, never a public DTO or a persisted secret record.
/// Deliberately has no Debug or serialization implementation.
#[derive(Clone)]
pub struct WebDavConfig {
    pub enabled: bool,
    pub auto_sync: bool,
    pub base_url: String,
    pub username: String,
    pub password: String,
    pub remote_root: String,
    pub profile: String,
}

impl WebDavConfig {
    /// Consume the input so the password is preserved byte-for-byte without copying.
    /// Disabled drafts may omit the address and username; supplied values remain validated.
    pub fn normalize(mut self) -> Result<Self, &'static str> {
        self.base_url = self.base_url.trim().to_owned();
        self.username = self.username.trim().to_owned();
        self.remote_root = self.remote_root.trim().trim_matches('/').to_owned();
        self.profile = self.profile.trim().to_owned();
        if self.remote_root.is_empty() {
            self.remote_root = "shiju-sync".to_owned();
        }
        if self.profile.is_empty() {
            self.profile = "default".to_owned();
        }
        validate_path(&self.remote_root)?;
        validate_path(&self.profile)?;
        if self.profile.contains('/') {
            return Err("WebDAV profile must be a single path segment");
        }
        if self.enabled && (self.base_url.is_empty() || self.username.is_empty()) {
            return Err("Enabled WebDAV requires an address and username");
        }
        if !self.base_url.is_empty() {
            self.base_url = normalize_url(&self.base_url)?;
        }
        Ok(self)
    }
}

pub(super) fn normalize_url(value: &str) -> Result<String, &'static str> {
    // Inspect the raw spelling before Url can erase dot segments or controls.
    if value.chars().any(|c| c.is_control() || c == '\\') {
        return Err("Invalid WebDAV address");
    }
    let (scheme, remainder) = value.split_once("://").ok_or("Invalid WebDAV address")?;
    if !scheme.eq_ignore_ascii_case("http") && !scheme.eq_ignore_ascii_case("https") {
        return Err("WebDAV requires HTTP or HTTPS");
    }
    if remainder.contains(['?', '#']) {
        return Err("WebDAV address cannot contain a query or fragment");
    }
    let (authority, path) = remainder.split_once('/').unwrap_or((remainder, ""));
    if authority.is_empty() || authority.contains('@') {
        return Err("WebDAV address cannot contain credentials");
    }
    let path = path.trim_end_matches('/');
    if !path.is_empty() {
        validate_path(path)?;
    }
    let url = Url::parse(value).map_err(|_| "Invalid WebDAV address")?;
    if url.host_str().is_none() || !url.username().is_empty() || url.password().is_some() {
        return Err("Invalid WebDAV address");
    }
    Ok(url.as_str().trim_end_matches('/').to_owned())
}

fn validate_path(value: &str) -> Result<(), &'static str> {
    for segment in value.split('/') {
        decode_path_segment(segment)?;
    }
    Ok(())
}

/// Decode exactly once, rejecting separators, dot segments and nested escapes.
pub(super) fn decode_path_segment(segment: &str) -> Result<String, &'static str> {
    // Decode once, rejecting residual percent signs to prevent nested encodings.
    let mut decoded = Vec::with_capacity(segment.len());
    let mut bytes = segment.bytes();
    while let Some(byte) = bytes.next() {
        if byte == b'%' {
            let high = bytes.next().and_then(hex).ok_or("Invalid WebDAV path encoding")?;
            let low = bytes.next().and_then(hex).ok_or("Invalid WebDAV path encoding")?;
            decoded.push(high * 16 + low);
        } else {
            decoded.push(byte);
        }
    }
    let decoded = String::from_utf8(decoded).map_err(|_| "Invalid WebDAV path encoding")?;
    if decoded.is_empty()
        || decoded == "."
        || decoded == ".."
        || decoded.chars().any(|c| c.is_control() || matches!(c, '/' | '\\' | '%' | '?' | '#' | ':'))
    {
        return Err("Unsafe WebDAV path segment");
    }
    Ok(decoded)
}

fn hex(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}
