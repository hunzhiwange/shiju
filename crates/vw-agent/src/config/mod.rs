//! 配置领域兼容导出及请求上下文组合。
use std::path::Path;
pub use agent_config::runtime::*;
pub mod schema;
pub mod traits;
#[cfg(not(target_arch = "wasm32"))]
pub mod migration;
#[cfg(not(target_arch = "wasm32"))]
pub use agent_config::store;
#[cfg(all(test, not(target_arch = "wasm32")))]
#[path = "migration_tests.rs"]
mod migration_tests;

/// 读取当前请求租户与项目的配置，每次读取都包含已保存的覆盖。
pub(crate) async fn get_current() -> Result<Config, Error> {
    let tenant = crate::session::ui_store::current_tenant_scope();
    let directory = crate::project::instance::directory();
    if directory.is_empty() {
        get_global_for_tenant(&tenant).await
    } else {
        get_project_for_tenant(&tenant, Path::new(&directory)).await
    }
}
