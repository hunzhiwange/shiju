// 覆盖会话 API 公共错误映射、目录解析与租户实例上下文。

use super::*;
use crate::app::agent::gateway::instance::InstanceQuery;
use crate::app::agent::project;
use axum::http::{HeaderMap, HeaderValue};
use std::sync::Once;
use tempfile::TempDir;

static TEST_HOME: Once = Once::new();
static SHARED_TEST_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

fn ensure_test_home() {
    TEST_HOME.call_once(|| {
        let dir =
            std::env::temp_dir().join(format!("vw-session-shared-tests-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("test home should be created");
        unsafe {
            std::env::set_var("VIBEWINDOW_TEST_HOME", dir);
        }
    });
}

// 查询参数优先于目录请求头，空值和非 UTF-8 请求头均不能建立隐式作用域。
// 错误用例固定失败发生在持久化之前，避免跨会话或跨作用域修改已有数据。
#[test]
fn resolve_scope_from_query_returns_scope_for_directory_query() {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let query = InstanceQuery { directory: Some(temp_dir.path().to_string_lossy().to_string()) };
    let headers = HeaderMap::new();

    assert!(
        resolve_scope_from_query(&vw_api_transport::TenantScope::default(), &query, &headers)
            .is_some()
    );
}

#[test]
fn resolve_scope_from_query_returns_scope_for_directory_header() {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let query = InstanceQuery { directory: None };
    let mut headers = HeaderMap::new();
    let path = temp_dir.path().to_string_lossy();
    let value = HeaderValue::from_str(path.as_ref()).expect("header value");
    headers.insert("x-directory", value);

    assert!(
        resolve_scope_from_query(&vw_api_transport::TenantScope::default(), &query, &headers)
            .is_some()
    );
}

#[tokio::test]
async fn with_tenant_instance_establishes_directory_context_and_returns_value() {
    let _guard = SHARED_TEST_LOCK.lock().await;
    ensure_test_home();
    project::instance::dispose_all().await;

    let project_dir = TempDir::new().expect("project dir");
    let expected = project_dir.path().to_string_lossy().to_string();
    let actual = with_tenant_instance(TenantScope::default(), expected.clone(), || {
        Box::pin(async { Ok(project::instance::directory()) })
    })
    .await
    .expect("tenant instance should execute the callback");

    assert_eq!(actual, expected);
    project::instance::dispose_all().await;
}
