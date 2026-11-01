//! Credential-free OAuth metadata discovery bound to a protected resource and exact issuer.
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Debug, Deserialize, Serialize)]
pub struct OAuthDiscovery {
    pub resource: String,
    pub issuer: String,
    pub authorization_endpoint: String,
    pub token_endpoint: String,
    pub scopes_supported: Option<Vec<String>>,
    pub client_id: Option<String>,
    pub registration_endpoint: Option<String>,
    #[serde(default)]
    pub client_id_metadata_document_supported: bool,
    pub token_endpoint_auth_methods_supported: Option<Vec<String>>,
}
#[derive(Deserialize)]
struct Resource {
    resource: String,
    authorization_servers: Vec<String>,
}
#[derive(Deserialize)]
struct Authority {
    issuer: String,
    authorization_endpoint: String,
    token_endpoint: String,
    scopes_supported: Option<Vec<String>>,
    registration_endpoint: Option<String>,
    #[serde(default)]
    client_id_metadata_document_supported: bool,
    token_endpoint_auth_methods_supported: Option<Vec<String>>,
}
pub(super) fn endpoint(raw: &str) -> Result<Url> {
    let url = Url::parse(raw).context("invalid OAuth metadata URL")?;
    if url.scheme() != "https"
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
        || url.query().is_some()
        || crate::tools::url_validation::is_private_or_local_host(url.host_str().unwrap_or(""))
    {
        bail!("OAuth discovery requires a public HTTPS URL without credentials, query or fragment");
    }
    Ok(url)
}
fn metadata_url(url: &Url, kind: &str) -> Url {
    let mut result = url.clone();
    result.set_path(&format!("/.well-known/{kind}{}", url.path().trim_end_matches('/')));
    result
}
pub(super) fn bind(
    server: &str,
    resource: serde_json::Value,
    authority: serde_json::Value,
) -> Result<OAuthDiscovery> {
    endpoint(server)?;
    let resource: Resource =
        serde_json::from_value(resource).context("invalid protected resource metadata")?;
    let authority: Authority =
        serde_json::from_value(authority).context("invalid authorization server metadata")?;
    if resource.resource != server || resource.authorization_servers.len() != 1 {
        bail!("OAuth resource mismatch or ambiguous authorization server");
    }
    let issuer = &resource.authorization_servers[0];
    endpoint(issuer)?;
    if &authority.issuer != issuer {
        bail!("OAuth authorization server issuer mismatch");
    }
    endpoint(&authority.authorization_endpoint)?;
    endpoint(&authority.token_endpoint)?;
    if let Some(registration) = &authority.registration_endpoint {
        endpoint(registration)?;
    }
    if authority
        .scopes_supported
        .as_ref()
        .is_some_and(|scopes| scopes.iter().any(|scope| scope.trim().is_empty()))
    {
        bail!("OAuth metadata contains an empty scope");
    }
    Ok(OAuthDiscovery {
        resource: resource.resource,
        issuer: authority.issuer,
        authorization_endpoint: authority.authorization_endpoint,
        token_endpoint: authority.token_endpoint,
        scopes_supported: authority.scopes_supported,
        client_id: None,
        registration_endpoint: authority.registration_endpoint,
        client_id_metadata_document_supported: authority.client_id_metadata_document_supported,
        token_endpoint_auth_methods_supported: authority.token_endpoint_auth_methods_supported,
    })
}
/// Discover without package headers or bearer tokens. Redirects and private DNS answers are rejected.
pub(super) async fn discover(server: &str) -> Result<OAuthDiscovery> {
    tokio::time::timeout(
        std::time::Duration::from_secs(30),
        discover_with(server, |url| async move { fetch(&url).await }),
    )
    .await
    .context("OAuth discovery timed out")?
}
pub(super) async fn discover_with<F, Fut>(server: &str, mut fetch: F) -> Result<OAuthDiscovery>
where
    F: FnMut(Url) -> Fut,
    Fut: std::future::Future<Output = Result<serde_json::Value>>,
{
    let server_url = endpoint(server)?;
    let resource = fetch(metadata_url(&server_url, "oauth-protected-resource")).await?;
    let parsed: Resource =
        serde_json::from_value(resource.clone()).context("invalid protected resource metadata")?;
    if parsed.resource != server || parsed.authorization_servers.len() != 1 {
        bail!("OAuth resource mismatch or ambiguous authorization server");
    }
    let issuer = endpoint(&parsed.authorization_servers[0])?;
    let authority = fetch(metadata_url(&issuer, "oauth-authorization-server")).await?;
    bind(server, resource, authority)
}
pub(super) async fn public_client(url: &Url) -> Result<reqwest_012::Client> {
    endpoint(url.as_str())?;
    let host = url.host_str().context("missing discovery host")?;
    let addresses: Vec<_> =
        tokio::net::lookup_host((host, url.port_or_known_default().unwrap_or(443)))
            .await
            .context("OAuth discovery DNS failed")?
            .collect();
    if addresses.is_empty()
        || addresses.iter().any(|address| {
            crate::tools::url_validation::is_private_or_local_host(&address.ip().to_string())
        })
    {
        bail!("OAuth discovery DNS returned a nonpublic address");
    }
    let client = reqwest_012::Client::builder()
        .no_proxy()
        .redirect(reqwest_012::redirect::Policy::none())
        .timeout(std::time::Duration::from_secs(10))
        .resolve_to_addrs(host, &addresses)
        .build()?;
    Ok(client)
}
async fn fetch(url: &Url) -> Result<serde_json::Value> {
    let mut response = public_client(url)
        .await?
        .get(url.clone())
        .send()
        .await
        .context("OAuth metadata request failed")?;
    if !response.status().is_success() {
        bail!("OAuth metadata HTTP status {}", response.status());
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await? {
        if bytes.len() + chunk.len() > 1024 * 1024 {
            bail!("OAuth metadata exceeds size limit");
        }
        bytes.extend_from_slice(&chunk);
    }
    serde_json::from_slice(&bytes).context("invalid OAuth metadata JSON")
}
