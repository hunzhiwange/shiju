//! HTTP regression coverage using production authentication, tenant and connection middleware.
use crate::app::agent::gateway;
use std::net::SocketAddr;
use agent_gateway::file_drag_contract::{self as contract, Server};
async fn server(bootstrap: bool) -> Server {
    server_on(bootstrap, "127.0.0.1").await
}
async fn server_on(bootstrap: bool, host: &str) -> Server {
    if bootstrap {
        // 状态 fixture 关闭外部依赖，只保留路由装配所需的最小共享组件。
        let state: gateway::AppState = agent_gateway::gateway_test_fixture::state(
            crate::config::Config::default(), true, &["drag-test-key".into()],
        );
        let router = gateway::bootstrap::client_router(
            state.clone(),
            axum::Router::new(),
            axum::Router::new(),
        )
        .with_state(state);
        let listener = tokio::net::TcpListener::bind((host, 0)).await.unwrap();
        let addr = listener.local_addr().unwrap();
        let handle = tokio::spawn(async move {
            axum::serve(listener, router.into_make_service_with_connect_info::<SocketAddr>())
                .await
                .map_err(|error| gateway::ApiError::internal(error.to_string()))
        });
        Server { addr, handle }
    } else {
        let (addr, task) =
            gateway::start(gateway::ServeOptions { hostname: host.into(), port: 0, cors: vec![] })
                .await
                .unwrap();
        Server { addr, handle: task }
    }
}

#[tokio::test]
async fn file_drag_integration_both_listeners_preserve_data_and_reject_conflicts() {
    for bootstrap in [true, false] {
        let server = server(bootstrap).await;
        contract::file_drag_integration_both_listeners_preserve_data_and_reject_conflicts(vec![server]).await;
    }
}

#[tokio::test]
async fn file_drag_integration_auth_tenant_origin_rejections_have_no_disk_effects() {
    let servers = vec![server(true).await];
    contract::file_drag_integration_auth_tenant_origin_rejections_have_no_disk_effects(servers).await;
}

#[cfg(unix)]
#[tokio::test]
async fn file_drag_integration_recursive_link_failure_keeps_source_and_other_items() {
    for bootstrap in [true, false] {
        let server = server(bootstrap).await;
        contract::file_drag_integration_recursive_link_failure_keeps_source_and_other_items(vec![server]).await;
    }
}

#[cfg(unix)]
#[tokio::test]
async fn file_drag_integration_non_loopback_cannot_spoof_forwarded_headers() {
    for bootstrap in [true, false] {
        let server = server_on(bootstrap, "0.0.0.0").await;
        contract::file_drag_integration_non_loopback_cannot_spoof_forwarded_headers(vec![server]).await;
    }
}

#[tokio::test]
async fn file_drag_integration_default_tenant_cannot_be_changed_from_another_tenant() {
    for bootstrap in [true, false] {
        let server = server(bootstrap).await;
        contract::file_drag_integration_default_tenant_cannot_be_changed_from_another_tenant(vec![(bootstrap, server)]).await;
    }
}
