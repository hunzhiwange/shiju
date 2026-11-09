//! Volcengine Ark control-plane V4 request signing.

use std::fmt::Write;

use chrono::{DateTime, Utc};
use hmac::{Hmac, KeyInit, Mac};
use reqwest::{Method, Request, Url, header};
use sha2::{Digest, Sha256};
use vw_api_types::apps_usage::UsageQueryErrorCode;

use crate::providers::validation::header_value;

const HOST: &str = "open.volcengineapi.com";
const VERSION: &str = "2024-01-01";
const SERVICE: &str = "ark";
const CONTENT_TYPE: &str = "application/json; charset=utf-8";
const SIGNED_HEADERS: &str = "host;x-date;x-content-sha256;content-type";

/// Builds a signed POST for the Ark usage endpoint using credentials resolved by the caller.
/// The request contains the same query and body bytes used by the signer.
pub fn sign_volcengine_request(
    access_key_id: &str,
    secret_access_key: &str,
    region: &str,
    action: &str,
    body: &[u8],
    now: DateTime<Utc>,
) -> Result<Request, UsageQueryErrorCode> {
    use UsageQueryErrorCode::{CredentialUnavailable, MissingConfiguration};

    let access_key_id = access_key_id.trim();
    let secret_access_key = secret_access_key.trim();
    if access_key_id.is_empty()
        || !access_key_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'+' | b'='))
        || secret_access_key.is_empty()
        || secret_access_key.chars().any(char::is_control)
    {
        return Err(CredentialUnavailable);
    }
    if region.is_empty()
        || !region.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        || action.trim().is_empty()
    {
        return Err(MissingConfiguration);
    }

    let query = format!("Action={}&Region={region}&Version={VERSION}", uri_encode(action));
    let url = Url::parse(&format!("https://{HOST}/?{query}")).map_err(|_| MissingConfiguration)?;
    let date = now.format("%Y%m%dT%H%M%SZ").to_string();
    let short_date = now.format("%Y%m%d").to_string();
    let payload_hash = lower_hex(&Sha256::digest(body));

    // Volcengine signs these headers in protocol order.
    let canonical_request = format!(
        "POST\n/\n{query}\nhost:{HOST}\nx-date:{date}\nx-content-sha256:{payload_hash}\ncontent-type:{CONTENT_TYPE}\n\n{SIGNED_HEADERS}\n{payload_hash}"
    );
    let scope = format!("{short_date}/{region}/{SERVICE}/request");
    let canonical_hash = lower_hex(&Sha256::digest(canonical_request.as_bytes()));
    let string_to_sign = format!("HMAC-SHA256\n{date}\n{scope}\n{canonical_hash}");

    let date_key = hmac_sha256(secret_access_key.as_bytes(), short_date.as_bytes());
    let region_key = hmac_sha256(&date_key, region.as_bytes());
    let service_key = hmac_sha256(&region_key, SERVICE.as_bytes());
    let signing_key = hmac_sha256(&service_key, b"request");
    let signature = lower_hex(&hmac_sha256(&signing_key, string_to_sign.as_bytes()));
    let mut authorization = header_value(&format!(
        "HMAC-SHA256 Credential={access_key_id}/{scope}, SignedHeaders={SIGNED_HEADERS}, Signature={signature}"
    ))
    .map_err(|_| CredentialUnavailable)?;
    authorization.set_sensitive(true);

    let mut request = Request::new(Method::POST, url);
    let headers = request.headers_mut();
    headers.insert(header::AUTHORIZATION, authorization);
    headers.insert(header::HOST, header::HeaderValue::from_static(HOST));
    headers.insert(header::CONTENT_TYPE, header::HeaderValue::from_static(CONTENT_TYPE));
    headers.insert("x-date", header_value(&date).map_err(|_| MissingConfiguration)?);
    headers
        .insert("x-content-sha256", header_value(&payload_hash).map_err(|_| MissingConfiguration)?);
    *request.body_mut() = Some(body.to_vec().into());
    Ok(request)
}

fn hmac_sha256(key: &[u8], message: &[u8]) -> [u8; 32] {
    let mut mac = Hmac::<Sha256>::new_from_slice(key).expect("HMAC accepts any key length");
    mac.update(message);
    mac.finalize().into_bytes().into()
}

fn lower_hex(bytes: &[u8]) -> String {
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        write!(encoded, "{byte:02x}").expect("writing to a String cannot fail");
    }
    encoded
}

fn uri_encode(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            encoded.push(char::from(byte));
        } else {
            write!(encoded, "%{byte:02X}").expect("writing to a String cannot fail");
        }
    }
    encoded
}
