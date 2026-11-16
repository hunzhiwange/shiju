// 覆盖命令、代理和技能列表等只读辅助路由。

use super::*;
use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use tower::ServiceExt;

async fn route_status(method: Method, uri: &str) -> StatusCode {
    router::<()>()
        .layer(Extension(vw_api_transport::TenantScope::default()))
        .oneshot(
            Request::builder()
                .method(method)
                .uri(uri)
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("route should respond")
        .status()
}

// 这些只读目录接口固定公开字段，且不依赖遗留项目文件是否可解析。
#[test]
fn router_builds_with_unit_state() {
    let _ = router::<()>();
}

#[tokio::test]
async fn router_dispatches_all_get_routes() {
    let temp = tempfile::tempdir().expect("tempdir");
    let directory = temp.path().to_string_lossy();

    assert_eq!(route_status(Method::GET, "/doc").await, StatusCode::NOT_IMPLEMENTED);
    for path in ["command", "agent", "skill"] {
        let uri = format!("/{path}?directory={directory}");
        assert_eq!(route_status(Method::GET, &uri).await, StatusCode::OK, "{path}");
    }
}

#[tokio::test]
async fn router_rejects_non_get_methods_and_unknown_paths() {
    for path in ["/doc", "/command", "/agent", "/skill"] {
        assert_eq!(
            route_status(Method::POST, path).await,
            StatusCode::METHOD_NOT_ALLOWED,
            "{path}"
        );
    }
    assert_eq!(route_status(Method::GET, "/missing").await, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn command_list_returns_named_command_metadata() {
    gateway_misc::test_contract::command_list_returns_named_command_metadata::<Host>().await;
}

#[tokio::test]
async fn agent_list_returns_keyed_agent_metadata() {
    gateway_misc::test_contract::agent_list_returns_keyed_agent_metadata::<Host>(crate::session::ui_store::current_tenant_scope()).await;
}

#[tokio::test]
async fn skill_list_returns_named_and_located_skill_metadata() {
    gateway_misc::test_contract::skill_list_returns_named_and_located_skill_metadata::<Host>().await;
}

#[tokio::test]
async fn list_handlers_ignore_legacy_invalid_project_json() {
    gateway_misc::test_contract::list_handlers_ignore_legacy_invalid_project_json::<Host>(crate::session::ui_store::current_tenant_scope(), crate::global::paths().data.clone()).await;
}

#[tokio::test]
async fn agent_profile_api_catalog_includes_disabled_only_when_requested() {
    let _env_lock = crate::test_support::global_async_env_lock().lock().await;
    gateway_misc::test_contract::agent_profile_api_catalog_includes_disabled_only_when_requested::<Host, _>(crate::session::ui_store::current_tenant_scope(), |directory| async move {
            crate::gateway::instance::with_instance(directory, || {
                Box::pin(async { Ok(crate::agent::get("acp-snow").await.is_none()) })
            }).await.unwrap()
        }).await;
}
