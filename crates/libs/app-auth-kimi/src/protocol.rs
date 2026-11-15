//! Regional endpoints and bounded OAuth response fields.

pub use app_auth_core::response::{read_json_limited, utc_now};
use reqwest::{RequestBuilder, Url};
use serde_json::Value;
use vw_api_app_auth::{AuthErrorCode, AuthRegion};

pub const CLIENT_ID: &str = "17e5f671-d194-4dfb-9706-5516cb48c098";
pub const MAX_RESPONSE_BYTES: usize = 64 * 1024;
pub const MAX_TOKEN_LIFETIME: u64 = 31_536_000;

pub struct Endpoints {
    pub oauth: &'static str,
    pub api: &'static str,
    domain: &'static str,
    pub identity_prefix: &'static str,
}

pub fn endpoints(region: AuthRegion) -> Endpoints {
    match region {
        AuthRegion::MainlandCn => Endpoints {
            oauth: "https://auth.kimi.com",
            api: "https://api.kimi.com/coding/v1",
            domain: "kimi.com",
            identity_prefix: "mainland-cn",
        },
        AuthRegion::Global => Endpoints {
            oauth: "https://auth.kimi.ai",
            api: "https://api.kimi.ai/coding/v1",
            domain: "kimi.ai",
            identity_prefix: "global",
        },
    }
}

pub fn identify(request: RequestBuilder, device_id: &str) -> RequestBuilder {
    request
        .header("Accept", "application/json")
        .header("X-Msh-Platform", "kimi_code_cli")
        .header("X-Msh-Version", env!("CARGO_PKG_VERSION"))
        .header("X-Msh-Device-Id", device_id)
        .header("X-Msh-Device-Name", "ShiJu")
        .header(
            "X-Msh-Device-Model",
            format!("{} {}", std::env::consts::OS, std::env::consts::ARCH),
        )
        .header("X-Msh-Os-Version", std::env::consts::OS)
}

pub fn token<'a>(payload: &'a Value, field: &str) -> Result<&'a str, AuthErrorCode> {
    payload[field].as_str().filter(|value| !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_graphic())).ok_or(AuthErrorCode::UpstreamFailure)
}

pub fn seconds(
    payload: &Value,
    field: &str,
    default: Option<u64>,
    max: u64,
) -> Result<u64, AuthErrorCode> {
    let value = match payload.get(field) {
        Some(value) => value.as_u64().or_else(|| value.as_str()?.parse().ok()),
        None => default,
    };
    value.filter(|value| (1..=max).contains(value)).ok_or(AuthErrorCode::UpstreamFailure)
}

pub fn verification_url(uri: &str, endpoints: &Endpoints, grant: &str) -> Result<Url, AuthErrorCode> {
    let url = Url::parse(uri).map_err(|_| AuthErrorCode::UpstreamFailure)?;
    if url.scheme() != "https"
        || !url.host_str().is_some_and(|host| {
            host == endpoints.domain
                || host
                    .strip_suffix(endpoints.domain)
                    .is_some_and(|prefix| matches!(prefix, "auth." | "www." | "code."))
        })
        || url.port_or_known_default() != Some(443)
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
        || url.as_str().contains(grant)
        || url.query_pairs().any(|(key, value)| key == "device_code" || value.contains(grant))
    {
        return Err(AuthErrorCode::UpstreamFailure);
    }
    Ok(url)
}

pub fn identity_field(value: &Value) -> Option<String> {
    match value {
        Value::Number(number) => Some(number.to_string()),
        Value::String(value) => {
            let value = value.trim();
            (!value.is_empty() && value.len() <= 256 && !value.chars().any(char::is_control))
                .then(|| value.to_owned())
        }
        _ => None,
    }
}
