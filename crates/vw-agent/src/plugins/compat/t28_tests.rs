use super::mcp_oauth_tokens::{Grant, access_with, read, remove, write};
use serde_json::json;
fn grant(expires: u64) -> Grant {
    Grant {
        issuer: "https://issuer.example".into(),
        client: "client".into(),
        resource: "https://server.example".into(),
        endpoint: "https://issuer.example/token".into(),
        access: "fixture-access".into(),
        refresh: Some("fixture-refresh".into()),
        expires,
    }
}
#[tokio::test]
async fn codex_plugin_compat_t28_invalidates_tokens_after_logout() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "owner", &grant(u64::MAX)).unwrap();
    assert!(read(dir.path(), "owner").unwrap().is_some());
    remove(dir.path(), "owner").unwrap();
    let result =
        access_with(dir.path(), "owner", |_| async { panic!("logged-out grant cannot refresh") })
            .await
            .unwrap();
    assert!(result.is_none());
}
#[tokio::test]
async fn codex_plugin_compat_t28_rejects_invalid_input() {
    assert!(
        Grant::response(
            &json!({"token_type":"bearer","access_token":"unsafe\nvalue","expires_in":60}),
            "issuer".into(),
            "client".into(),
            "resource".into(),
            "endpoint".into(),
            None
        )
        .is_err()
    );
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "owner", &grant(0)).unwrap();
    assert!(
        access_with(dir.path(), "owner", |_| async { anyhow::bail!("provider rejected refresh") })
            .await
            .is_err()
    );
    assert!(
        read(dir.path(), "owner").unwrap().is_none(),
        "failed refresh must invalidate old credentials"
    );
}
#[tokio::test]
async fn codex_plugin_compat_t28_preserves_isolation() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "owner-a", &grant(0)).unwrap();
    write(dir.path(), "owner-b", &grant(u64::MAX)).unwrap();
    let access = access_with(dir.path(), "owner-a", |old| async move {
        assert_eq!(old.refresh.as_deref(), Some("fixture-refresh"));
        let mut new = grant(u64::MAX);
        new.access = "rotated-access".into();
        Ok(new)
    })
    .await
    .unwrap();
    assert_eq!(access.as_deref(), Some("rotated-access"));
    assert_eq!(read(dir.path(), "owner-b").unwrap().unwrap().access, "fixture-access");
    let bytes = std::fs::read_to_string(dir.path().join("owner-a.grant")).unwrap();
    assert!(!bytes.contains("rotated-access"));
    remove(dir.path(), "owner-a").unwrap();
    assert!(read(dir.path(), "owner-b").unwrap().is_some());
}

#[test]
fn codex_plugin_compat_t28_rejects_transplanted_ciphertext() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "owner-a", &grant(u64::MAX)).unwrap();
    std::fs::copy(dir.path().join("owner-a.grant"), dir.path().join("owner-b.grant")).unwrap();
    assert!(read(dir.path(), "owner-b").is_err(), "ciphertext must authenticate its owner binding");
}

#[tokio::test]
async fn codex_plugin_compat_t28_owner_keys_and_concurrent_refresh() {
    use super::{mcp_elicitation::FormOwner, mcp_oauth_tokens};
    let dir = tempfile::tempdir().unwrap();
    let options = super::t17_tests::options(dir.path());
    super::t17_tests::plugin(
        &options,
        "oauth-owner",
        json!({"url":"https://mcp.example/mcp","oauth":{"client_id":"public-fixture"}}),
    );
    let mut servers = Vec::new();
    crate::plugins::mcp::merge_enabled_plugin_mcp_servers(&mut servers, &options).unwrap();
    let agent_client_protocol::McpServer::Http(server) = &servers[0] else {
        panic!("HTTP fixture")
    };
    let owner = FormOwner {
        session: format!("t28-{}", dir.path().display()),
        tenant: crate::session::ui_store::current_tenant_scope(),
    };
    let cancelled = mcp_oauth_tokens::connection_cancellation(server, &owner).unwrap();
    let held = mcp_oauth_tokens::lock(server, &owner).await.unwrap();
    assert!(
        tokio::time::timeout(
            std::time::Duration::from_millis(20),
            mcp_oauth_tokens::lock(server, &owner)
        )
        .await
        .is_err()
    );
    let other =
        FormOwner { session: format!("{}-other", owner.session), tenant: owner.tenant.clone() };
    let isolated = mcp_oauth_tokens::lock(server, &other).await.unwrap();
    drop(isolated);
    drop(held);
    write(dir.path(), "concurrent", &grant(0)).unwrap();
    let first = async {
        let _lock = mcp_oauth_tokens::lock(server, &owner).await.unwrap();
        access_with(dir.path(), "concurrent", |_| async {
            let mut g = grant(u64::MAX);
            g.access = "first-refresh".into();
            Ok(g)
        })
        .await
        .unwrap()
    };
    let second = async {
        let _lock = mcp_oauth_tokens::lock(server, &owner).await.unwrap();
        access_with(dir.path(), "concurrent", |_| async {
            let mut g = grant(u64::MAX);
            g.access = "second-refresh".into();
            Ok(g)
        })
        .await
        .unwrap()
    };
    let (a, b) = tokio::join!(first, second);
    assert_eq!(a, b, "concurrent waiters must observe the same refreshed grant");
    assert_ne!(a.as_deref(), Some("fixture-access"));
    mcp_oauth_tokens::logout(server, owner.session.clone(), owner.tenant).await.unwrap();
    assert!(cancelled.is_cancelled());
    assert!(!mcp_oauth_tokens::connection_cancellation(server, &other).unwrap().is_cancelled());
}

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
    let pool = super::mcp_connections::McpSessionConnections::for_form_owner(owner.clone());
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
