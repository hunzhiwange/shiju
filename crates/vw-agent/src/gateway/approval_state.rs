//! Gateway 实例级审批状态缓存。
//!
//! 审批策略从配置构建后会绑定到当前项目实例目录，避免多个工作区共享同一个
//! `ApprovalManager` 而导致权限决策串扰。测试或配置刷新时可以按目录清理缓存。

use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex};

use crate::app::agent::approval::ApprovalManager;
use crate::app::agent::config;
use crate::app::agent::project::instance;

static APPROVAL_MANAGERS: LazyLock<Mutex<HashMap<String, Arc<ApprovalManager>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// 返回当前项目实例对应的审批管理器。
///
/// 该函数使用当前实例目录作为缓存 key。首次访问时会读取配置并构造
/// `ApprovalManager`，后续请求复用同一实例，以保持审批状态在同一工作区内连续。
///
/// # 返回值
///
/// 返回可共享的审批管理器引用。
pub(crate) async fn approval_manager_for_current_instance() -> Arc<ApprovalManager> {
    let directory = normalize_directory_key(&instance::directory());

    // 快速路径只读取缓存；锁中毒时继续走重建路径，避免一次 panic 永久阻断审批。
    if let Ok(lock) = APPROVAL_MANAGERS.lock()
        && let Some(existing) = lock.get(&directory)
    {
        return existing.clone();
    }

    let config = config::get().await;
    let manager = Arc::new(ApprovalManager::from_config(&config.autonomy));

    // 插入时再次检查 entry，防止并发请求为同一目录创建出多个长期实例。
    let mut lock = APPROVAL_MANAGERS.lock().unwrap_or_else(|error| error.into_inner());
    lock.entry(directory).or_insert_with(|| manager.clone()).clone()
}

/// 清理指定目录的审批管理器缓存。
///
/// # 参数
///
/// * `directory` - 项目实例目录，必须与创建审批管理器时使用的目录字符串一致。
///
/// # 错误处理
///
/// 该函数用于测试和配置刷新路径；如果缓存锁不可用，会直接跳过清理。
pub(crate) fn clear_approval_manager_for_directory(directory: &str) {
    if let Ok(mut lock) = APPROVAL_MANAGERS.lock() {
        lock.remove(&normalize_directory_key(directory));
    }
}

fn normalize_directory_key(directory: &str) -> String {
    std::fs::canonicalize(directory)
        .unwrap_or_else(|_| std::path::PathBuf::from(directory))
        .to_string_lossy()
        .into_owned()
}

#[cfg(test)]
#[path = "approval_state_tests.rs"]
mod approval_state_tests;
