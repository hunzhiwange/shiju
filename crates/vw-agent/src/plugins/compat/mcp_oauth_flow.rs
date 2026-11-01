//! One-shot public MCP authorization. Secrets stay in the owning invocation.
use anyhow::{Context, Result, bail};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use sha2::{Digest, Sha256};
use url::Url;

pub(super) struct PendingAuthorization {
    state: String,
    verifier: String,
    redirect: Url,
    pub(super) authorization_url: Url,
}

impl PendingAuthorization {
    pub(super) fn new(
        endpoint: &str,
        client: &str,
        redirect: &str,
        resource: &str,
    ) -> Result<Self> {
        let mut authorization_url = Url::parse(endpoint)?;
        let redirect = Url::parse(redirect)?;
        if authorization_url.scheme() != "https"
            || authorization_url.host_str().is_none()
            || !authorization_url.username().is_empty()
            || authorization_url.password().is_some()
            || authorization_url.fragment().is_some()
            || authorization_url.query().is_some()
            || client.trim().is_empty()
            || redirect.scheme() != "http"
            || redirect.host_str() != Some("127.0.0.1")
            || redirect.port().is_none()
            || redirect.query().is_some()
            || redirect.fragment().is_some()
            || !redirect.username().is_empty()
            || redirect.password().is_some()
        {
            bail!("invalid public MCP authorization endpoint, client or loopback callback");
        }
        let mut random = [0u8; 64];
        getrandom::fill(&mut random).context("MCP authorization entropy unavailable")?;
        let verifier = URL_SAFE_NO_PAD.encode(&random[..32]);
        let state = URL_SAFE_NO_PAD.encode(&random[32..]);
        let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
        authorization_url.query_pairs_mut().extend_pairs([
            ("response_type", "code"),
            ("client_id", client),
            ("redirect_uri", redirect.as_str()),
            ("resource", resource),
            ("code_challenge_method", "S256"),
            ("code_challenge", challenge.as_str()),
            ("state", state.as_str()),
        ]);
        Ok(Self { state, verifier, redirect, authorization_url })
    }

    pub(super) fn complete(self, callback: &str) -> Result<(String, String)> {
        let callback = Url::parse(callback)?;
        if callback.origin() != self.redirect.origin()
            || callback.path() != self.redirect.path()
            || callback.fragment().is_some()
            || !callback.username().is_empty()
            || callback.password().is_some()
        {
            bail!("MCP authorization callback origin or path mismatch");
        }
        let pairs: Vec<_> = callback.query_pairs().collect();
        let unique = |key: &str| -> Result<String> {
            let values: Vec<_> = pairs.iter().filter(|(name, _)| name == key).collect();
            if values.len() != 1 || values[0].1.is_empty() {
                bail!("missing or duplicate MCP authorization callback parameter");
            }
            Ok(values[0].1.to_string())
        };
        if unique("state")? != self.state {
            bail!("MCP authorization state mismatch");
        }
        if pairs.iter().any(|(name, _)| name == "error") {
            bail!("MCP authorization denied by authorization server");
        }
        Ok((unique("code")?, self.verifier))
    }
}

/// Authorize a source-validated public MCP server in an existing user session.
/// The returned access token is owned by the caller and is never logged or persisted here.
pub async fn authorize(
    server: &agent_client_protocol::McpServerHttp,
    session: String,
    tenant: vw_api_types::id::TenantScope,
    cancelled: tokio_util::sync::CancellationToken,
) -> Result<String> {
    let operation = async {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let config = super::mcp_auth::source_config(server)?
            .context("MCP authorization requires plugin provenance")?;
        if config
            .get("oauth")
            .and_then(|oauth| oauth.get("client_secret"))
            .is_some_and(|v| !v.is_null())
        {
            bail!("MCP authorization currently requires a public client without a secret");
        }
        let metadata = super::mcp_auth::discover_oauth(server).await?;
        let client = metadata
            .client_id
            .as_deref()
            .context("MCP authorization requires a registered public client identifier")?;
        let configured_callback = config
            .get("oauth")
            .and_then(|v| v.get("callback_url"))
            .and_then(serde_json::Value::as_str);
        let callback_port = config
            .get("oauth")
            .and_then(|v| v.get("callback_port"))
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0);
        let port = if let Some(callback) = configured_callback {
            let callback = Url::parse(callback)?;
            PendingAuthorization::new(
                &metadata.authorization_endpoint,
                client,
                callback.as_str(),
                &metadata.resource,
            )?;
            callback.port().context("MCP callback requires an explicit loopback port")?
        } else {
            u16::try_from(callback_port).context("invalid MCP callback port")?
        };
        let listener = tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, port)).await?;
        let redirect = configured_callback.map(str::to_owned).unwrap_or_else(|| {
            format!(
                "http://127.0.0.1:{}/callback",
                listener.local_addr().expect("bound listener address").port()
            )
        });
        let redirect_url = Url::parse(&redirect)?;
        let mut pending = PendingAuthorization::new(
            &metadata.authorization_endpoint,
            client,
            &redirect,
            &metadata.resource,
        )?;
        let scopes = config.get("scopes").or_else(|| {
            config.get("extensions").and_then(|v| v.pointer("/com.openai/auth/scopes"))
        });
        if let Some(scopes) = scopes.filter(|v| !v.is_null()) {
            let scopes = scopes.as_array().context("MCP OAuth scopes must be an array")?;
            let scopes = scopes
                .iter()
                .map(|v| {
                    v.as_str().filter(|s| !s.trim().is_empty()).context("invalid MCP OAuth scope")
                })
                .collect::<Result<Vec<_>>>()?;
            if !scopes.is_empty() {
                pending.authorization_url.query_pairs_mut().append_pair("scope", &scopes.join(" "));
            }
        }
        let answers = crate::session::ui_store::with_tenant_scope(tenant,
            crate::question::ask_sensitive(crate::question::AskInput {
                session_id: session,
                questions: vec![crate::question::Info {
                    question: format!("Authorize MCP server {}? Open this URL in your browser, then accept to continue:\n{}", server.name, pending.authorization_url),
                    header: server.name.clone(),
                    options: ["accept", "decline"].into_iter().map(|label| crate::question::OptionInfo { label: label.into(), description: String::new(), preview: None }).collect(),
                    multiple: Some(false), custom: Some(false),
                }], tool: None,
            }, vw_shared::question::McpForm { server: server.name.clone(), schema: serde_json::json!({"type":"object", "properties": {}}) })
        ).await.context("MCP authorization interaction cancelled")?;
        if answers != vec![vec![String::from("accept")]] {
            bail!("MCP authorization denied");
        }
        let (mut socket, _) = listener.accept().await?;
        let mut request = Vec::new();
        loop {
            let byte = socket.read_u8().await?;
            request.push(byte);
            if request.ends_with(b"\r\n\r\n") {
                break;
            }
            if request.len() >= 8192 {
                bail!("MCP authorization callback too large");
            }
        }
        let request = std::str::from_utf8(&request)?;
        let line = request.lines().next().context("missing MCP callback request")?;
        let parts: Vec<_> = line.split_whitespace().collect();
        if parts.len() != 3
            || parts[0] != "GET"
            || !parts[1].starts_with(&format!("{}?", redirect_url.path()))
        {
            bail!("invalid MCP callback request");
        }
        let (code, verifier) = pending.complete(&format!(
            "{}{}",
            redirect_url.origin().ascii_serialization(),
            parts[1]
        ))?;
        socket.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 23\r\nConnection: close\r\nContent-Type: text/plain\r\n\r\nAuthorization received.").await?;
        // Recheck package provenance after user interaction, before exchanging the code.
        super::mcp_auth::source_config(server)?.context("MCP source context disappeared")?;
        let endpoint = Url::parse(&metadata.token_endpoint)?;
        let body = url::form_urlencoded::Serializer::new(String::new())
            .extend_pairs([
                ("grant_type", "authorization_code"),
                ("client_id", client),
                ("code", &code),
                ("code_verifier", &verifier),
                ("redirect_uri", &redirect),
                ("resource", &metadata.resource),
            ])
            .finish();
        let mut response = super::mcp_oauth_discovery::public_client(&endpoint)
            .await?
            .post(endpoint)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .context("MCP token exchange transport failed")?;
        if !response.status().is_success() {
            bail!("MCP token exchange rejected");
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await? {
            if bytes.len() + chunk.len() > 1024 * 1024 {
                bail!("MCP token response exceeds size limit");
            }
            bytes.extend_from_slice(&chunk);
        }
        let value: serde_json::Value =
            serde_json::from_slice(&bytes).context("invalid MCP token response")?;
        if value["token_type"].as_str().is_none_or(|value| !value.eq_ignore_ascii_case("bearer")) {
            bail!("unsupported MCP token type");
        }
        value["access_token"]
            .as_str()
            .filter(|value| !value.is_empty())
            .map(str::to_owned)
            .context("missing MCP access token")
    };
    tokio::select! {
        biased;
        _ = cancelled.cancelled() => bail!("MCP authorization cancelled"),
        result = tokio::time::timeout(std::time::Duration::from_secs(120), operation) => result.context("MCP authorization timed out")?,
    }
}
