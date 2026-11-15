//! Local S3-compatible configuration validation; grants no network access.

use reqwest::Url;

/// Execution-side input after the host resolves explicit secret intent.
/// No Debug or serialization implementation: this contains raw secret material.
pub struct S3Config {
    pub enabled: bool,
    pub auto_sync: bool,
    pub bucket: String,
    pub region: String,
    pub access_key_id: String,
    pub secret_access_key: String,
    pub endpoint: Option<String>,
    pub remote_root: String,
    pub profile: String,
}

impl S3Config {
    /// Disabled drafts may omit required fields, but supplied fields must be safe.
    /// The consumed secret is preserved byte-for-byte, including whitespace.
    pub fn normalize(mut self) -> Result<Self, &'static str> {
        self.bucket = self.bucket.trim().to_owned();
        self.region = self.region.trim().to_owned();
        self.access_key_id = self.access_key_id.trim().to_owned();
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
            return Err("S3 profile must be a single key segment");
        }
        if !self.bucket.is_empty() {
            validate_path(&self.bucket)?;
            if self.bucket.contains('/')
                || !self.bucket.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_'))
            {
                return Err("Invalid S3 bucket");
            }
        }
        if !self.region.is_empty()
            && !self.region.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
        {
            return Err("Invalid S3 region");
        }
        if self.access_key_id.chars().any(|c| c.is_control() || c.is_whitespace()) {
            return Err("Invalid S3 access key ID");
        }
        self.endpoint = match self.endpoint {
            Some(value) if !value.trim().is_empty() => Some(normalize_endpoint(value.trim())?),
            _ => None,
        };
        if self.enabled
            && (self.bucket.is_empty()
                || self.region.is_empty()
                || self.access_key_id.is_empty()
                || self.secret_access_key.is_empty())
        {
            return Err("Enabled S3 requires bucket, region, access key ID and secret access key");
        }
        Ok(self)
    }
}

fn normalize_endpoint(value: &str) -> Result<String, &'static str> {
    // Inspect raw spelling before URL parsing can erase controls or dot segments.
    if value.chars().any(|c| c.is_control() || c == '\\') {
        return Err("Invalid S3 endpoint");
    }
    let (scheme, remainder) = value.split_once("://").ok_or("Invalid S3 endpoint")?;
    if !scheme.eq_ignore_ascii_case("http") && !scheme.eq_ignore_ascii_case("https") {
        return Err("S3 endpoint requires HTTP or HTTPS");
    }
    if remainder.contains(['?', '#']) {
        return Err("S3 endpoint cannot contain a query or fragment");
    }
    let (authority, path) = remainder.split_once('/').unwrap_or((remainder, ""));
    if authority.is_empty() || authority.contains('@') {
        return Err("S3 endpoint cannot contain credentials");
    }
    if !path.is_empty() {
        return Err("S3 endpoint must be an origin without a path");
    }
    let url = Url::parse(value).map_err(|_| "Invalid S3 endpoint")?;
    if url.host_str().is_none() || !url.username().is_empty() || url.password().is_some() {
        return Err("Invalid S3 endpoint");
    }
    Ok(url.as_str().trim_end_matches('/').to_owned())
}

fn validate_path(value: &str) -> Result<(), &'static str> {
    for segment in value.split('/') {
        // Decode once, rejecting residual percent signs to prevent nested encodings.
        let mut decoded = Vec::with_capacity(segment.len());
        let mut bytes = segment.bytes();
        while let Some(byte) = bytes.next() {
            if byte == b'%' {
                let high = bytes.next().and_then(hex).ok_or("Invalid S3 path encoding")?;
                let low = bytes.next().and_then(hex).ok_or("Invalid S3 path encoding")?;
                decoded.push(high * 16 + low);
            } else {
                decoded.push(byte);
            }
        }
        let decoded = std::str::from_utf8(&decoded).map_err(|_| "Invalid S3 path encoding")?;
        if decoded.is_empty()
            || decoded == "."
            || decoded == ".."
            || decoded.chars().any(|c| c.is_control() || matches!(c, '/' | '\\' | '%' | '?' | '#' | ':'))
        {
            return Err("Unsafe S3 path segment");
        }
    }
    Ok(())
}

fn hex(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}
