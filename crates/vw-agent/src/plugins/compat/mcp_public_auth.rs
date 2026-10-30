//! Public per-server com.openai authentication declarations, separate from connection credentials.
use anyhow::{Context, Result, bail};
use reqwest_012::header::HeaderName;
use serde::Deserialize;
use serde_json::{Map, Value};
use url::Url;

pub(super) enum ConnectionPolicy {
    Anonymous,
    OAuth,
    ApiKey,
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
enum PublicAuth {
    None,
    Oauth(OAuthSettings),
    Mixed(OAuthSettings),
    ApiKey {
        #[serde(rename = "headerScheme")]
        header_scheme: HeaderScheme,
        #[serde(rename = "headerName")]
        header_name: Option<String>,
    },
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum HeaderScheme {
    Bearer,
    Basic,
    CustomHeader,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct OAuthSettings {
    client: Option<OAuthClient>,
    authorization_url: Option<String>,
    token_url: Option<String>,
    registration_url: Option<String>,
    authorization_server_base: Option<String>,
    resource: Option<String>,
    base_scopes: Option<Vec<String>>,
    default_scopes: Option<Vec<String>>,
    oidc_enabled: Option<bool>,
    oidc_configuration_url: Option<String>,
    oidc_userinfo_endpoint: Option<String>,
    oidc_scopes_supported: Option<Vec<String>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct OAuthClient {
    mode: ClientMode,
    client_id: Option<String>,
    token_endpoint_auth_method: Option<ClientAuthMethod>,
}
#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum ClientMode {
    Provided,
    Dcr,
    Cimd,
}
#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum ClientAuthMethod {
    None,
    ClientSecretBasic,
    ClientSecretPost,
    PrivateKeyJwt,
}

pub(super) fn policy(config: &Map<String, Value>) -> Result<Option<ConnectionPolicy>> {
    let Some(extensions) = config.get("extensions") else { return Ok(None) };
    let extensions = extensions.as_object().context("plugin MCP extensions must be an object")?;
    if extensions.keys().any(|name| name != "com.openai") {
        bail!("unsupported plugin MCP extension namespace");
    }
    let Some(openai) = extensions.get("com.openai") else { return Ok(None) };
    let openai = openai.as_object().context("plugin MCP com.openai extension must be an object")?;
    if openai.keys().any(|field| field != "auth") {
        bail!("unsupported public plugin MCP extension field");
    }
    let Some(auth) = openai.get("auth") else { return Ok(None) };
    reject_nulls(auth)?;
    let object = auth.as_object().context("public plugin MCP auth must be an object")?;
    if object.get("type").and_then(Value::as_str) == Some("none") && object.len() != 1 {
        bail!("public plugin MCP auth none cannot include credential or OAuth fields");
    }
    let auth: PublicAuth = serde_json::from_value(auth.clone())
        .map_err(|_| anyhow::anyhow!("invalid public plugin MCP authentication declaration"))?;
    match auth {
        PublicAuth::None => Ok(Some(ConnectionPolicy::Anonymous)),
        PublicAuth::Oauth(settings) => {
            validate_oauth(settings)?;
            Ok(Some(ConnectionPolicy::OAuth))
        }
        PublicAuth::Mixed(settings) => {
            validate_oauth(settings)?;
            Ok(Some(ConnectionPolicy::Anonymous))
        }
        PublicAuth::ApiKey { header_scheme, header_name } => {
            match header_scheme {
                HeaderScheme::CustomHeader => {
                    let name =
                        header_name.context("public API-key custom_header requires headerName")?;
                    let name = HeaderName::from_bytes(name.as_bytes())
                        .map_err(|_| anyhow::anyhow!("invalid public API-key headerName"))?;
                    if name == "host" {
                        bail!("public API-key headerName cannot be Host");
                    }
                }
                HeaderScheme::Bearer | HeaderScheme::Basic if header_name.is_some() => {
                    bail!("public API-key headerName is only valid for custom_header")
                }
                _ => {}
            }
            Ok(Some(ConnectionPolicy::ApiKey))
        }
    }
}

fn reject_nulls(value: &Value) -> Result<()> {
    match value {
        Value::Null => bail!("public plugin MCP auth fields must use their declared types"),
        Value::Object(object) => {
            for value in object.values() {
                reject_nulls(value)?;
            }
        }
        Value::Array(array) => {
            for value in array {
                reject_nulls(value)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn validate_oauth(settings: OAuthSettings) -> Result<()> {
    if let Some(client) = settings.client {
        if client.client_id.as_ref().is_some_and(|id| id.trim().is_empty()) {
            bail!("public OAuth clientId must be nonempty");
        }
        match client.mode {
            ClientMode::Provided if client.client_id.is_none() => {
                bail!("public provided OAuth client requires clientId")
            }
            ClientMode::Dcr | ClientMode::Cimd if client.token_endpoint_auth_method.is_some() => {
                bail!("public OAuth tokenEndpointAuthMethod requires provided client mode")
            }
            _ => {}
        }
    }
    for url in [
        settings.authorization_url,
        settings.token_url,
        settings.registration_url,
        settings.authorization_server_base,
        settings.resource,
        settings.oidc_configuration_url,
        settings.oidc_userinfo_endpoint,
    ]
    .into_iter()
    .flatten()
    {
        let url = Url::parse(&url)
            .map_err(|_| anyhow::anyhow!("invalid public OAuth endpoint override"))?;
        if !matches!(url.scheme(), "http" | "https")
            || url.host().is_none()
            || !url.username().is_empty()
            || url.password().is_some()
            || url.fragment().is_some()
        {
            bail!(
                "public OAuth endpoints must be absolute HTTP(S) without credentials or fragments"
            );
        }
    }
    for scopes in [settings.base_scopes, settings.default_scopes, settings.oidc_scopes_supported]
        .into_iter()
        .flatten()
    {
        if scopes.iter().any(|scope| scope.trim().is_empty()) {
            bail!("public OAuth scope strings must be nonempty");
        }
    }
    // The declaration is retained verbatim, including explicit false and empty arrays.
    let _ = settings.oidc_enabled;
    Ok(())
}
