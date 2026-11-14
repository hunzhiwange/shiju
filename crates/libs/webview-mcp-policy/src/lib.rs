//! Pure MCP Apps CSP, origin and internal navigation policy.
use url::Url;
use vw_api_types::mcp_app_network::McpAppNetworkPolicy;

pub const MAX_MESSAGE_BYTES: usize = 1024 * 1024;
pub const WRAPPER_URL: &str = "mcp-app://localhost/index.html";
pub const APP_URL: &str = "mcp-app://localhost/app.html";
#[cfg(target_os = "windows")]
pub const ENGINE_WRAPPER_URL: &str = "http://mcp-app.localhost/index.html";
#[cfg(not(target_os = "windows"))]
pub const ENGINE_WRAPPER_URL: &str = WRAPPER_URL;
#[cfg(target_os = "windows")]
pub const ENGINE_APP_URL: &str = "http://mcp-app.localhost/app.html";
#[cfg(not(target_os = "windows"))]
pub const ENGINE_APP_URL: &str = APP_URL;

const WRAPPER_CSP: &str = "default-src 'none'; script-src 'unsafe-inline'; style-src 'unsafe-inline'; frame-src 'self'; base-uri 'none'; form-action 'none'";

pub fn app_csp(policy: &McpAppNetworkPolicy) -> String {
    fn sources(origins: &[String]) -> String {
        if origins.is_empty() { "'none'".into() } else { origins.join(" ") }
    }
    let resources = policy.resource_domains.join(" ");
    let connect = sources(&policy.connect_domains);
    let frame = sources(&policy.frame_domains);
    let base = sources(&policy.base_uri_domains);
    let media = sources(&policy.resource_domains);
    let forms = sources(&allowed_origins(policy));
    format!(
        "default-src 'none'; script-src 'unsafe-inline' {resources}; style-src 'unsafe-inline' {resources}; font-src data: {resources}; img-src data: {resources}; media-src {media}; connect-src {connect}; frame-src {frame}; base-uri {base}; object-src 'none'; form-action {forms}"
    )
}

pub fn wrapper_csp(origins: &[String]) -> String {
    if origins.is_empty() {
        return WRAPPER_CSP.into();
    }
    WRAPPER_CSP.replace("frame-src 'self';", &format!("frame-src 'self' {};", origins.join(" ")))
}

pub fn internal_navigation_allowed(url: &str) -> bool {
    matches!(url, WRAPPER_URL | APP_URL | "about:blank" | "about:srcdoc")
        || url == ENGINE_WRAPPER_URL
        || url == ENGINE_APP_URL
}

pub fn https_origin(target: &str) -> Option<String> {
    let url = Url::parse(target).ok()?;
    (url.scheme() == "https" && url.username().is_empty() && url.password().is_none())
        .then(|| url.origin().ascii_serialization())
}

pub fn frame_origin_allowed(domains: &[String], target: &str) -> bool {
    https_origin(target).is_some_and(|origin| domains.contains(&origin))
}

pub fn allowed_origins(policy: &McpAppNetworkPolicy) -> Vec<String> {
    if cfg!(any(target_os = "windows", coverage)) {
        return Vec::new();
    }
    policy
        .connect_domains
        .iter()
        .filter(|origin| {
            // GTK's action API does not identify the destination frame. Keep
            // the form destinations disjoint from ordinary iframe navigations.
            cfg!(target_os = "macos") || !policy.frame_domains.contains(origin)
        })
        .cloned()
        .collect()
}

#[cfg(test)]
mod policy_tests;
