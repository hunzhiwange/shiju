//! 项目管理模块
//!
//! 本模块提供项目的完整生命周期管理，包括：
//! - 项目信息存储与检索（基于 Git worktree 或本地目录）
//! - 版本控制系统（VCS）检测与集成
//! - 项目图标自动发现
//! - 沙箱目录管理
//! - 项目实例上下文管理
//! - 项目状态管理

use crate::app::agent::snapshot;
#[cfg(test)]
use crate::app::agent::storage;
use crate::app::agent::util::log;
use serde_json::{Map, Value};
use std::collections::HashSet;
use std::future::Future;
use std::path::Path;
use std::pin::Pin;
use std::sync::{LazyLock, Mutex};
use {
    vw_api_transport::id::ProjectId,
    vw_api_transport::id::ProjectScope,
    vw_api_transport::id::TenantScope,
};

#[cfg(not(target_arch = "wasm32"))]
use crate::global_database::GlobalDatabase;

mod core;
mod icon_discovery;
pub mod instance;
#[cfg(not(target_arch = "wasm32"))]
mod migration;
pub mod state;
#[cfg(not(target_arch = "wasm32"))]
pub mod store;
pub mod vcs;

/// 模块专用日志记录器
static LOGGER: LazyLock<log::Logger> = LazyLock::new(|| {
    let mut tags = Map::new();
    tags.insert("service".to_string(), Value::String("project".to_string()));
    log::create(Some(tags))
});

static BOOTSTRAPPED_WORKTREES: LazyLock<Mutex<HashSet<String>>> =
    LazyLock::new(|| Mutex::new(HashSet::new()));

#[cfg(not(target_arch = "wasm32"))]
static RUNTIME_STORE: LazyLock<Mutex<Option<store::ProjectStore>>> =
    LazyLock::new(|| Mutex::new(None));

/// 装箱的 Future 类型别名
pub type BoxFuture<T> = Pin<Box<dyn Future<Output = T> + Send + 'static>>;

/// 项目相关事件定义模块
pub mod event {
    use crate::app::agent::bus;

    pub const UPDATED: bus::Definition = bus::Definition { r#type: "project.updated" };
}

pub use core::{
    add_sandbox, from_directory, get, list, remove_sandbox, sandboxes, set_initialized, update,
};
pub use icon_discovery::discover;
#[cfg(not(target_arch = "wasm32"))]
pub use migration::{MigrationOutcome, migrate_project_registry};
pub use project_types::{
    Commands, CommandsUpdate, Icon, IconUpdate, Info, TimeInfo, UpdateInput, Vcs,
};

pub use project_types::Error;

pub fn default_tenant_scope() -> TenantScope {
    TenantScope::default()
}

pub fn project_scope_for_default_tenant(
    project_id: impl Into<ProjectId>,
) -> Result<ProjectScope, Error> {
    Ok(ProjectScope::new(default_tenant_scope(), project_id.into())?)
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn runtime_store() -> Result<store::ProjectStore, Error> {
    let mut cached = RUNTIME_STORE.lock().unwrap_or_else(|error| error.into_inner());
    if let Some(store) = cached.as_ref() {
        return Ok(store.clone());
    }

    #[cfg(test)]
    let store = store::ProjectStore::new(GlobalDatabase::new(
        std::env::var_os("VW_ACP_MOUNT_RECOVERY_DB_ROOT")
            .map(|root| std::path::PathBuf::from(root).join("global.sqlite3"))
            .unwrap_or_else(|| {
                std::env::temp_dir()
                    .join(format!("vw-agent-test-global-store-{}.sqlite3", std::process::id()))
            }),
    )?)?;

    #[cfg(not(test))]
    let store = {
        let home = &crate::app::agent::global::paths().home;
        let store = store::ProjectStore::new(GlobalDatabase::for_home(home)?)?;
        migrate_project_registry(home, &store)?;
        store
    };
    *cached = Some(store.clone());
    Ok(store)
}

#[cfg(test)]
pub(crate) fn store_local_test_project(info: &Info) -> Result<(), Error> {
    let store = runtime_store()?;
    let scope = project_scope_for_default_tenant(info.id.as_str())?;
    if store.get(&scope)?.is_some() {
        store.update(&scope, info)?;
    } else {
        store.create(scope.tenant_scope(), info)?;
    }
    Ok(())
}

#[cfg(test)]
pub(crate) fn read_local_test_project(project_id: &str) -> Result<Option<Info>, Error> {
    let store = runtime_store()?;
    Ok(store.get(&project_scope_for_default_tenant(project_id)?)?)
}

#[cfg(test)]
pub(crate) fn remove_local_test_project(project_id: &str) -> Result<(), Error> {
    let store = runtime_store()?;
    store.delete(&project_scope_for_default_tenant(project_id)?)?;
    Ok(())
}

/// 引导项目实例
pub async fn instance_bootstrap(worktree: impl AsRef<Path>) {
    let worktree = worktree.as_ref().to_path_buf();
    let key = worktree.to_string_lossy().to_string();
    let should_init = {
        let mut bootstrapped = BOOTSTRAPPED_WORKTREES.lock().unwrap_or_else(|e| e.into_inner());
        bootstrapped.insert(key)
    };
    if should_init {
        snapshot::init(&worktree);
    }
}

fn extra<const N: usize>(pairs: [(&'static str, Value); N]) -> Map<String, Value> {
    let mut m = Map::new();
    for (k, v) in pairs {
        m.insert(k.to_string(), v);
    }
    m
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
