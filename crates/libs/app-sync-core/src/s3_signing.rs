//! S3 addressing and deterministic SigV4 signing. Performs no I/O.

use std::fmt::Write;

use chrono::{DateTime, Utc};
use hmac::{Hmac, KeyInit, Mac};
use reqwest::{Method, Request, Url, header::{self, HeaderMap, HeaderValue}};
use sha2::{Digest, Sha256};

use super::s3_config::S3Config;

/// Builds an object URL from a normalized configuration and a raw UTF-8 key.
/// Empty key addresses the bucket. Slashes are retained; percent signs are literal.
pub fn build_object_url(config: &S3Config, key: &str) -> Result<Url, &'static str> {
    if config.bucket.is_empty()
        || !config.bucket.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_'))
        || config.region.is_empty()
        || !config.region.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
    {
        return Err("Invalid S3 addressing configuration");
    }
    let key = key.trim_start_matches('/');
    // URL parsers normalize dot segments, which would silently change the object.
    if key.split('/').any(|part| matches!(part, "." | ".."))
        || key.chars().any(char::is_control)
    {
        return Err("Unsafe S3 object key");
    }
    let endpoint = match config.endpoint.as_deref() {
        None => None,
        Some(value) => {
            if value.chars().any(|c| c.is_control() || c == '\\') {
                return Err("Invalid S3 endpoint");
            }
            let url = Url::parse(value).map_err(|_| "Invalid S3 endpoint")?;
            if !matches!(url.scheme(), "https" | "http") || url.host_str().is_none()
                || !url.username().is_empty() || url.password().is_some()
                || url.path() != "/" || url.query().is_some() || url.fragment().is_some()
            {
                return Err("S3 endpoint must be an HTTP origin");
            }
            Some(url)
        }
    };
    // Match an actual AWS DNS suffix rather than a substring in an arbitrary host.
    let aws = endpoint.as_ref().is_none_or(|url| {
        url.host_str().is_some_and(|host| host == "amazonaws.com" || host.ends_with(".amazonaws.com"))
    });
    let raw = if aws {
        if !config.bucket.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'.' | b'-')) {
            return Err("AWS bucket must be a DNS name");
        }
        format!("https://{}.s3.{}.amazonaws.com/{}", config.bucket, config.region, uri_encode(key, false))
    } else {
        let origin = endpoint.as_ref().ok_or("Missing S3 endpoint")?;
        format!("{}{}/{key}", origin.as_str(), uri_encode(&config.bucket, true), key = uri_encode(key, false))
    };
    Url::parse(&raw).map_err(|_| "Invalid S3 object URL")
}

/// Returns the exact signed request, including the body bytes that were hashed.
/// Query pairs are raw strings (duplicates retained). The caller supplies UTC time.
/// Headers such as content-type and range participate in the signature. Callers
/// must not modify the returned URL, signed headers or body before sending it.
#[allow(clippy::too_many_arguments)]
pub fn sign_request(
    config: &S3Config,
    method: Method,
    key: &str,
    query: &[(String, String)],
    mut headers: HeaderMap,
    body: Vec<u8>,
    now: DateTime<Utc>,
) -> Result<Request, &'static str> {
    if !config.enabled || config.access_key_id.is_empty() || config.secret_access_key.is_empty()
        || !config.access_key_id.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'+' | b'='))
    {
        return Err("S3 signing requires enabled configuration and credentials");
    }
    if headers.contains_key(header::AUTHORIZATION) {
        return Err("S3 request already has authorization");
    }
    let mut url = build_object_url(config, key)?;
    let mut pairs: Vec<_> = query.iter().map(|(k, v)| (uri_encode(k, true), uri_encode(v, true))).collect();
    pairs.sort();
    let canonical_query = pairs.iter().map(|(k, v)| format!("{k}={v}")).collect::<Vec<_>>().join("&");
    if !pairs.is_empty() {
        url.set_query(Some(&canonical_query));
    }
    let host = url.host_str().ok_or("Missing S3 host")?;
    let host = match url.port() {
        Some(port) => format!("{host}:{port}"),
        None => host.to_owned(),
    };
    let timestamp = now.format("%Y%m%dT%H%M%SZ").to_string();
    let date = now.format("%Y%m%d").to_string();
    let payload_hash = sha256_hex(&body);
    headers.insert(header::HOST, header_value(&host)?);
    headers.insert("x-amz-date", header_value(&timestamp)?);
    headers.insert("x-amz-content-sha256", header_value(&payload_hash)?);
    let mut names: Vec<_> = headers.keys().map(|name| name.as_str()).collect();
    names.sort_unstable();
    names.dedup();
    let mut canonical_headers = String::new();
    for name in &names {
        let mut values = Vec::new();
        for value in headers.get_all(*name) {
            let value = value.to_str().map_err(|_| "S3 signing requires ASCII headers")?;
            values.push(value.split_ascii_whitespace().collect::<Vec<_>>().join(" "));
        }
        writeln!(canonical_headers, "{name}:{}", values.join(",")).expect("String write");
    }
    let signed_headers = names.join(";");
    let canonical = format!("{}\n{}\n{canonical_query}\n{canonical_headers}\n{signed_headers}\n{payload_hash}", method.as_str(), url.path());
    let scope = format!("{date}/{}/s3/aws4_request", config.region);
    let to_sign = format!("AWS4-HMAC-SHA256\n{timestamp}\n{scope}\n{}", sha256_hex(canonical.as_bytes()));
    let date_key = hmac_sha256(format!("AWS4{}", config.secret_access_key).as_bytes(), date.as_bytes());
    let region_key = hmac_sha256(&date_key, config.region.as_bytes());
    let service_key = hmac_sha256(&region_key, b"s3");
    let signing_key = hmac_sha256(&service_key, b"aws4_request");
    let signature = lower_hex(&hmac_sha256(&signing_key, to_sign.as_bytes()));
    let mut authorization = header_value(&format!("AWS4-HMAC-SHA256 Credential={}/{scope}, SignedHeaders={signed_headers}, Signature={signature}", config.access_key_id))?;
    authorization.set_sensitive(true);
    headers.insert(header::AUTHORIZATION, authorization);
    let mut request = Request::new(method, url);
    *request.headers_mut() = headers;
    *request.body_mut() = Some(body.into());
    Ok(request)
}

fn header_value(value: &str) -> Result<HeaderValue, &'static str> {
    HeaderValue::from_str(value).map_err(|_| "Invalid S3 signing header")
}

fn hmac_sha256(key: &[u8], value: &[u8]) -> [u8; 32] {
    let mut mac = Hmac::<Sha256>::new_from_slice(key).expect("HMAC accepts any key length");
    mac.update(value);
    mac.finalize().into_bytes().into()
}

fn sha256_hex(value: &[u8]) -> String {
    lower_hex(&Sha256::digest(value))
}

fn lower_hex(value: &[u8]) -> String {
    let mut output = String::with_capacity(value.len() * 2);
    for byte in value {
        write!(output, "{byte:02x}").expect("String write");
    }
    output
}

fn uri_encode(value: &str, encode_slash: bool) -> String {
    let mut output = String::with_capacity(value.len());
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') || (byte == b'/' && !encode_slash) {
            output.push(char::from(byte));
        } else {
            write!(output, "%{byte:02X}").expect("String write");
        }
    }
    output
}
