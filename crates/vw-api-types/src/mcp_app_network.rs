//! Validated network declarations shared by the desktop host and WebView helper.

use serde::Serialize;
use serde_json::Value;
use url::Url;

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpAppNetworkPolicy {
    pub connect_domains: Vec<String>,
    pub resource_domains: Vec<String>,
    pub frame_domains: Vec<String>,
    pub base_uri_domains: Vec<String>,
    /// Origins reserved for host-side external-link admission.
    pub redirect_domains: Vec<String>,
}

impl McpAppNetworkPolicy {
    pub fn parse(declaration: Option<&Value>) -> Result<Self, &'static str> {
        let mut policy = Self::default();
        let Some(declaration) = declaration.filter(|value| !value.is_null()) else {
            return Ok(policy);
        };
        let fields = declaration.as_object().ok_or("invalid MCP app CSP object")?;
        for (field, value) in fields {
            let domains = match field.as_str() {
                "connectDomains" => &mut policy.connect_domains,
                "resourceDomains" => &mut policy.resource_domains,
                "frameDomains" => &mut policy.frame_domains,
                "baseUriDomains" => &mut policy.base_uri_domains,
                "redirectDomains" => &mut policy.redirect_domains,
                _ => return Err("unsupported MCP app CSP directive"),
            };
            let entries = value
                .as_array()
                .filter(|entries| entries.len() <= 32)
                .ok_or("invalid MCP app CSP domain list")?;
            for entry in entries {
                let source = entry.as_str().ok_or("invalid MCP app origin")?;
                domains.push(parse_https_origin(source)?);
            }
        }
        Ok(policy)
    }
}

fn parse_https_origin(source: &str) -> Result<String, &'static str> {
    const INVALID_ORIGIN: &str = "MCP app CSP requires an exact HTTPS origin";
    let authority = source.strip_prefix("https://").ok_or(INVALID_ORIGIN)?;
    let host = match authority.split_once(':') {
        Some((host, port)) => {
            if !port.bytes().all(|byte| byte.is_ascii_digit())
                || port.parse::<u16>().ok().is_none_or(|port| port == 0)
            {
                return Err("invalid MCP app origin port");
            }
            host
        }
        None => authority,
    };
    if host.len() > 253
        || !host.contains('.')
        || host.split('.').any(|label| {
            label.is_empty()
                || label.len() > 63
                || label.starts_with('-')
                || label.ends_with('-')
                || !label.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        })
    {
        return Err(INVALID_ORIGIN);
    }
    let url = Url::parse(source).map_err(|_| INVALID_ORIGIN)?;
    if !url.host_str().is_some_and(|parsed| parsed.eq_ignore_ascii_case(host)) {
        return Err(INVALID_ORIGIN);
    }
    Ok(url.origin().ascii_serialization())
}
