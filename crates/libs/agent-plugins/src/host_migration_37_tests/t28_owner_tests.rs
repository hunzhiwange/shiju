use plugin_mcp::mcp_oauth_tokens::{Grant, access_with, write};
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
async fn codex_plugin_compat_t28_owner_keys_and_concurrent_refresh() {
    use plugin_mcp::{mcp_elicitation::FormOwner, mcp_oauth_tokens};
    let dir = tempfile::tempdir().unwrap();
    let options = super::t17_tests::options(dir.path());
    super::t17_tests::plugin(
        &options,
        "oauth-owner",
        json!({"url":"https://mcp.example/mcp","oauth":{"client_id":"public-fixture"}}),
    );
    let mut servers = Vec::new();
    super::merge_enabled_plugin_mcp_servers(&mut servers, &options).unwrap();
    let agent_client_protocol::McpServer::Http(server) = &servers[0] else {
        panic!("HTTP fixture")
    };
    let owner = FormOwner {
        session: format!("t28-{}", dir.path().display()),
        tenant: Default::default(),
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
