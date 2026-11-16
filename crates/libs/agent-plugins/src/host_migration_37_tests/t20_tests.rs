use plugin_mcp::mcp_connections::McpSessionConnections;
use plugin_mcp::catalog_test_support::{Fixture, Service};
use agent_client_protocol::McpServer;
use std::sync::{Arc, atomic::Ordering};
async fn tools(pool: &McpSessionConnections, server: &McpServer) -> Vec<rmcp::model::Tool> {
    pool.list_tools(server, None).await.unwrap()
}
#[tokio::test]
async fn codex_plugin_compat_t20_invalidates_catalog_after_notification() {
    let fixture = Fixture::start().await;
    let pool = McpSessionConnections::new(super::connection_host());
    let first = tools(&pool, &fixture.server).await;
    let before = fixture.service.lists.load(Ordering::SeqCst);
    assert_eq!(tools(&pool, &fixture.server).await, first);
    assert_eq!(
        fixture.service.lists.load(Ordering::SeqCst),
        before,
        "unchanged connection must reuse its cached catalog"
    );
    fixture.changed(false).await;
    let next = tools(&pool, &fixture.server).await;
    assert_ne!(next, first, "the notified connection must publish its new definitions");
    pool.shutdown();
}

#[tokio::test]
async fn codex_plugin_compat_t20_retries_notification_during_initial_listing_without_mixed_publication()
 {
    let service = Service::default();
    service.changes_during_listing.store(1, Ordering::SeqCst);
    let fixture = Fixture::with_service(service).await;
    let pool = McpSessionConnections::new(super::connection_host());
    let result = tools(&pool, &fixture.server).await;
    assert_eq!(result[0].description.as_deref(), Some("generation 1"));
    assert_eq!(fixture.service.lists.load(Ordering::SeqCst), 2);
    pool.shutdown();
}

#[tokio::test]
async fn codex_plugin_compat_t20_rejects_persistent_notification_churn() {
    let service = Service::default();
    service.changes_during_listing.store(2, Ordering::SeqCst);
    let fixture = Fixture::with_service(service).await;
    let pool = McpSessionConnections::new(super::connection_host());
    let error = pool.list_tools(&fixture.server, None).await.unwrap_err();
    assert!(format!("{error:#}").contains("changed during pagination"), "{error:#}");
    assert_eq!(fixture.service.lists.load(Ordering::SeqCst), 2);
    assert_eq!(fixture.service.calls.load(Ordering::SeqCst), 0);
    pool.shutdown();
}

#[tokio::test]
async fn codex_plugin_compat_t20_unadvertised_changes_do_not_enable_persistent_caching() {
    let service = Service::default();
    service.unadvertised.store(1, Ordering::SeqCst);
    let fixture = Fixture::with_service(service).await;
    let pool = McpSessionConnections::new(super::connection_host());
    let first = tools(&pool, &fixture.server).await;
    let before = fixture.service.lists.load(Ordering::SeqCst);
    assert_eq!(tools(&pool, &fixture.server).await, first);
    assert!(
        fixture.service.lists.load(Ordering::SeqCst) > before,
        "servers that cannot invalidate must be listed anew"
    );
    pool.shutdown();
    let unsupported = mcp_client::client::unsupported_mcp_capabilities_json();
    assert!(
        unsupported
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["capability"] == "resources.subscribe")
    );
    assert!(
        unsupported
            .as_array()
            .unwrap()
            .iter()
            .all(|item| !item["capability"].as_str().unwrap().ends_with("listChanged"))
    );
}

#[tokio::test]
async fn codex_plugin_compat_t20_rejects_resource_template_and_prompt_cursor_cycles() {
    for mode in [2, 3, 4] {
        let service = Service::default();
        service.invalid.store(mode, Ordering::SeqCst);
        let fixture = Fixture::with_service(service).await;
        let pool = McpSessionConnections::new(super::connection_host());
        let error = pool
            .with_client(&fixture.server, None, move |client| {
                Box::pin(async move {
                    let catalog = &client.service().catalogs;
                    match mode {
                        2 => {
                            catalog.resources(client).await?;
                        }
                        3 => {
                            catalog.templates(client).await?;
                        }
                        4 => {
                            catalog.prompts(client).await?;
                        }
                        _ => unreachable!(),
                    }
                    Ok(())
                })
            })
            .await
            .unwrap_err();
        assert!(format!("{error:#}").contains("repeated pagination cursor"), "{error:#}");
        pool.shutdown();
    }
}

#[tokio::test]
async fn codex_plugin_compat_t20_shutdown_cancels_inflight_resource_refresh() {
    let service = Service::default();
    service.invalid.store(5, Ordering::SeqCst);
    let fixture = Fixture::with_service(service).await;
    let pool = Arc::new(McpSessionConnections::new(super::connection_host()));
    let active = pool.clone();
    let server = fixture.server.clone();
    let refresh = tokio::spawn(async move {
        active
            .with_client(&server, None, |client| {
                Box::pin(async move { client.service().catalogs.resources(client).await })
            })
            .await
    });
    tokio::time::timeout(
        std::time::Duration::from_secs(2),
        fixture.service.resource_started.notified(),
    )
    .await
    .unwrap();
    pool.shutdown();
    let result =
        tokio::time::timeout(std::time::Duration::from_secs(2), refresh).await.unwrap().unwrap();
    assert!(format!("{:#}", result.unwrap_err()).contains("closed"));
    assert_eq!(fixture.service.resource_lists.load(Ordering::SeqCst), 1);
    assert!(pool.list_tools(&fixture.server, None).await.is_err());
}

use super::tools::{Tool, mcp_tool::{McpTool, mcp_tools_to_specs}};
async fn bound(pool: &Arc<McpSessionConnections>, server: &McpServer) -> McpTool {
    let defs = tools(pool, server).await;
    let spec = mcp_tools_to_specs("catalog", None, &defs, &[]).unwrap().remove(0);
    McpTool::from_session_server(server.clone(), "read", spec, pool.clone(), None)
}

#[tokio::test]
async fn codex_plugin_compat_t20_rejects_invalid_input() {
    let fixture = Fixture::start().await;
    let pool = Arc::new(McpSessionConnections::new(super::connection_host()));
    let old = bound(&pool, &fixture.server).await;
    fixture.changed(true).await;
    let error = old
        .call(serde_json::json!({"value":"old"}))
        .await
        .expect_err("an invalid notified directory must not admit old calls");
    assert!(format!("{error:#}").contains("repeated"), "{error:#}");
    assert_eq!(fixture.service.calls.load(Ordering::SeqCst), 0);
    pool.shutdown();
}

#[tokio::test]
async fn codex_plugin_compat_t20_preserves_isolation() {
    let first = Fixture::start().await;
    let second = Fixture::start().await;
    let a = Arc::new(McpSessionConnections::new(super::connection_host()));
    let b = Arc::new(McpSessionConnections::new(super::connection_host()));
    let old_a = bound(&a, &first.server).await;
    let old_b = bound(&b, &second.server).await;
    first.changed(false).await;
    assert!(
        old_a.call(serde_json::json!({"value":"old"})).await.is_err(),
        "old model binding must not invoke changed definitions"
    );
    assert_eq!(
        old_b.call(serde_json::json!({"value":"unchanged"})).await.unwrap().model_text(),
        "called"
    );
    assert_eq!(first.service.calls.load(Ordering::SeqCst), 0);
    assert_eq!(second.service.calls.load(Ordering::SeqCst), 1);
    a.shutdown();
    assert!(old_a.call(serde_json::json!({})).await.is_err());
    b.shutdown();
}

#[tokio::test]
async fn codex_plugin_compat_t20_reconnect_rejects_old_binding_without_closing_current_binding() {
    let fixture = Fixture::start().await;
    let pool = Arc::new(McpSessionConnections::new(super::connection_host()));
    let old = bound(&pool, &fixture.server).await;
    pool.with_peer(&fixture.server, None, |_| {
        Box::pin(async { Err::<(), _>(anyhow::anyhow!("fixture connection failure")) })
    })
    .await
    .unwrap_err();
    let current = bound(&pool, &fixture.server).await;
    assert!(
        old.call(serde_json::json!({})).await.is_err(),
        "a new connection must not reuse old model bindings even with equal definitions"
    );
    assert_eq!(current.call(serde_json::json!({})).await.unwrap().model_text(), "called");
    assert_eq!(fixture.service.calls.load(Ordering::SeqCst), 1);
    pool.shutdown();
}

#[tokio::test]
async fn codex_plugin_compat_t20_notification_does_not_revoke_an_already_admitted_call() {
    let fixture = Fixture::start().await;
    let pool = Arc::new(McpSessionConnections::new(super::connection_host()));
    let tool = bound(&pool, &fixture.server).await;
    let running = tool.clone();
    let call = tokio::spawn(async move { running.call(serde_json::json!({"wait":true})).await });
    tokio::time::timeout(
        std::time::Duration::from_secs(2),
        fixture.service.call_started.notified(),
    )
    .await
    .unwrap();
    fixture.changed(false).await;
    fixture.service.finish_call.notify_one();
    assert_eq!(call.await.unwrap().unwrap().model_text(), "called");
    assert!(tool.call(serde_json::json!({})).await.is_err());
    assert_eq!(fixture.service.calls.load(Ordering::SeqCst), 1);
    pool.shutdown();
}

#[tokio::test]
async fn codex_plugin_compat_t20_equal_notification_preserves_bindings_and_shutdown_closes_catalogs()
 {
    let fixture = Fixture::start().await;
    let pool = Arc::new(McpSessionConnections::new(super::connection_host()));
    let tool = bound(&pool, &fixture.server).await;
    fixture.peer.lock().await.clone().unwrap().notify_tool_list_changed().await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(30)).await;
    assert!(tool.call(serde_json::json!({})).await.is_ok());
    let (catalogs, peer) = pool
        .with_client(&fixture.server, None, |client| {
            Box::pin(async move { Ok((client.service().catalogs.clone(), client.peer().clone())) })
        })
        .await
        .unwrap();
    let before = fixture.service.lists.load(Ordering::SeqCst);
    pool.shutdown();
    assert!(
        catalogs.tools(&peer).await.is_err(),
        "cancelled catalogs cannot publish updates or return a live cache"
    );
    assert_eq!(fixture.service.lists.load(Ordering::SeqCst), before);
}

#[tokio::test]
async fn dynamic_tools_mcp_refresh_notification_marks_only_connection_revision() {
    let fixture = Fixture::start().await;
    let pool = Arc::new(McpSessionConnections::new(super::connection_host()));
    let _ = bound(&pool, &fixture.server).await;
    let before = pool.tool_revision(&fixture.server).unwrap();
    fixture.peer.lock().await.as_ref().unwrap().notify_tool_list_changed().await.unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        while pool.tool_revision(&fixture.server) == Some(before) {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert_eq!(pool.tool_revision(&fixture.server), Some(before + 1));
    pool.shutdown();
}
