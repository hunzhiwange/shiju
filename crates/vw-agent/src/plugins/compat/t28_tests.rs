use super::mcp_oauth_tokens::Grant;
use serde_json::json;

#[tokio::test]
#[ignore = "real public OAuth playground; explicitly run with --ignored"]
async fn codex_plugin_compat_t28_public_oauth_refresh() -> anyhow::Result<()> {
    use super::{mcp_oauth_discovery, mcp_oauth_flow::send_provider_request};
    use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
    use sha2::{Digest, Sha256};
    let metadata = mcp_oauth_discovery::discover("https://api.sondahub.com/mcp/secure").await?;
    let endpoint = url::Url::parse(metadata.registration_endpoint.as_deref().unwrap())?;
    let client = mcp_oauth_discovery::public_client(&endpoint).await?;
    let redirect = "http://127.0.0.1:39871/callback";
    let registration = send_provider_request(client,endpoint,json!({"client_name":"VibeWindow compatibility test","redirect_uris":[redirect],"token_endpoint_auth_method":"none"}).to_string(),"application/json").await?;
    let client_id = registration["client_id"].as_str().unwrap();
    let mut random = [0u8; 32];
    getrandom::fill(&mut random)?;
    let verifier = URL_SAFE_NO_PAD.encode(random);
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
    let mut authorization = url::Url::parse(&metadata.authorization_endpoint)?;
    authorization.query_pairs_mut().extend_pairs([
        ("response_type", "code"),
        ("client_id", client_id),
        ("redirect_uri", redirect),
        ("scope", "mcp:read"),
        ("state", "t28-public-test"),
        ("code_challenge", &challenge),
        ("code_challenge_method", "S256"),
        ("resource", &metadata.resource),
        ("auto", "1"),
    ]);
    let response =
        mcp_oauth_discovery::public_client(&url::Url::parse(&metadata.authorization_endpoint)?)
            .await?
            .get(authorization)
            .send()
            .await?;
    anyhow::ensure!(response.status().is_redirection(), "public consent did not redirect");
    let callback = url::Url::parse(response.headers()["location"].to_str()?)?;
    let code =
        callback.query_pairs().find(|(key, _)| key == "code").map(|(_, v)| v.into_owned()).unwrap();
    let body = url::form_urlencoded::Serializer::new(String::new())
        .extend_pairs([
            ("grant_type", "authorization_code"),
            ("client_id", client_id),
            ("code", &code),
            ("code_verifier", &verifier),
            ("redirect_uri", redirect),
            ("resource", &metadata.resource),
        ])
        .finish();
    let endpoint = url::Url::parse(&metadata.token_endpoint)?;
    let value = send_provider_request(
        mcp_oauth_discovery::public_client(&endpoint).await?,
        endpoint,
        body,
        "application/x-www-form-urlencoded",
    )
    .await?;
    let mut grant = Grant::response(
        &value,
        metadata.issuer,
        client_id.into(),
        metadata.resource,
        metadata.token_endpoint,
        None,
    )?;
    grant.expires = 0; // Force the production refresh path before the real MCP connection.
    let temp = tempfile::tempdir()?;
    let options = super::t17_tests::options(temp.path());
    let package = super::t17_tests::plugin(
        &options,
        "public-oauth",
        json!({"url":"https://api.sondahub.com/mcp/secure","oauth":{},"scopes":["mcp:read"]}),
    );
    let manifest = package.join(".codex-plugin/plugin.json");
    let original = std::fs::read(&manifest)?;
    let mut servers = Vec::new();
    crate::plugins::mcp::merge_enabled_plugin_mcp_servers(&mut servers, &options)?;
    let agent_client_protocol::McpServer::Http(server) = &servers[0] else {
        anyhow::bail!("expected HTTP server")
    };
    let owner = super::mcp_elicitation::FormOwner {
        session: format!("t28-public-{}", temp.path().display()),
        tenant: crate::session::ui_store::current_tenant_scope(),
    };
    super::mcp_oauth_tokens::save(server, &owner, &grant)?;
    let pool = super::mcp_connections::McpSessionConnections::for_form_owner(crate::plugins::compat::mcp_connections::host(), owner.clone());
    let tools = pool.list_tools(&servers[0], Some(std::time::Duration::from_secs(30))).await;
    // Always clear the real host grant before propagating a network/transport error.
    super::mcp_oauth_tokens::logout(server, owner.session, owner.tenant).await?;
    anyhow::ensure!(!tools?.is_empty(), "public MCP tool catalog was empty");
    anyhow::ensure!(
        pool.list_tools(&servers[0], Some(std::time::Duration::from_secs(15))).await.is_err(),
        "logged-out connection remained authorized"
    );
    anyhow::ensure!(std::fs::read(manifest)? == original, "plugin package changed");
    Ok(())
}
