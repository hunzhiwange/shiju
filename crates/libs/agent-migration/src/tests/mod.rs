//! 迁移模块的测试套件
//!
//! 本模块提供了配置迁移功能的集成测试和单元测试基础设施。
//! 包含以下测试模块：
//!
//! - `backup`: 备份功能测试
//! - `migration`: 迁移逻辑测试
//! - `parse`: 配置解析测试
//! - `sqlite`: SQLite 后端测试
//! - `workspace`: 工作空间管理测试
//!
//! 本模块还提供了测试辅助工具，包括测试配置生成器和
//! 环境变量隔离机制，确保测试的独立性和可重复性。

mod backup;
mod migration;
mod parse;
mod sqlite;
mod workspace;

use agent_config::runtime::{Config, MemoryConfig};
use std::path::Path;

/// 创建用于测试的配置实例
///
/// 生成一个最小化的测试配置，使用指定的临时工作空间目录。
/// 配置默认使用 SQLite 作为内存后端，适合隔离的测试环境。
///
/// # 参数
///
/// * `workspace` - 测试用的工作空间路径，通常指向临时目录
///
/// # 返回值
///
/// 返回一个配置好的 `Config` 实例，其中：
/// - `workspace_dir` 设置为提供的路径
/// - `config_path` 设置为工作空间下的 `vibewindow.json`
/// - `memory.backend` 设置为 `"sqlite"`
/// - 其他字段使用默认值
///
/// # 示例
///
/// ```ignore
/// use std::path::PathBuf;
/// let temp_dir = tempfile::tempdir().unwrap();
/// let config = test_config(temp_dir.path());
/// assert_eq!(config.memory.backend, "sqlite");
/// ```
fn test_config(workspace: &Path) -> Config {
    Config {
        workspace_dir: workspace.to_path_buf(),
        config_path: workspace.join("vibewindow.json"),
        memory: MemoryConfig { backend: "sqlite".to_string(), ..MemoryConfig::default() },
        ..Config::default()
    }
}

/// 环境变量保护器
///
/// RAII 风格的环境变量管理器，用于在测试中临时修改环境变量，
/// 并在作用域结束时自动恢复原始值。这确保了测试之间不会
/// 因为环境变量变更而相互干扰。
///
/// # 生命周期
///
/// 1. 创建时保存当前环境变量的值
/// 2. 设置新的环境变量值（或删除该变量）
/// 3. 析构时自动恢复原始值
///
/// # 线程安全性
///
/// 使用此结构体时，建议配合 [`env_lock`] 函数获取全局环境变量锁，
/// 以防止多个测试并发修改环境变量导致竞态条件。
///
/// # 示例
///
/// ```ignore
/// {
///     let _guard = EnvGuard::set("MY_VAR", Some("test_value"));
///     // 在此作用域内，MY_VAR 的值为 "test_value"
/// }
/// // 离开作用域后，MY_VAR 自动恢复为原始值
/// ```
pub(super) struct EnvGuard {
    /// 环境变量名称
    key: &'static str,
    /// 修改前的原始值（如果变量不存在则为 None）
    previous: Option<std::ffi::OsString>,
}

impl EnvGuard {
    /// 设置环境变量并返回保护器
    ///
    /// 临时设置指定的环境变量，并返回一个 `EnvGuard` 实例。
    /// 当该实例被丢弃时，环境变量将自动恢复为原始值。
    ///
    /// # 参数
    ///
    /// * `key` - 环境变量名称（必须是静态生命周期的字符串）
    /// * `value` - 新的变量值；`Some(v)` 表示设置该值，`None` 表示删除该变量
    ///
    /// # 返回值
    ///
    /// 返回一个 `EnvGuard` 实例，持有原始值的所有权以便后续恢复
    ///
    /// # 注意事项
    ///
    /// 此方法会立即修改进程的环境变量。在并发测试中，
    /// 应确保在调用此方法前已获取 [`env_lock`]。
    pub(super) fn set(key: &'static str, value: Option<&str>) -> Self {
        // 保存当前环境变量的值（如果存在）
        let previous = std::env::var_os(key);
        // 根据参数设置新值或删除变量
        match value {
            Some(v) => unsafe { std::env::set_var(key, v) },
            None => unsafe { std::env::remove_var(key) },
        }
        Self { key, previous }
    }
}

impl Drop for EnvGuard {
    /// 析构时自动恢复环境变量
    ///
    /// 当 `EnvGuard` 实例离开作用域时，自动将环境变量
    /// 恢复为创建时保存的原始值，确保测试隔离性。
    ///
    /// # 恢复逻辑
    ///
    /// - 如果原始值存在，则恢复该值
    /// - 如果原始值不存在（变量之前未定义），则删除该变量
    fn drop(&mut self) {
        if let Some(value) = &self.previous {
            // 恢复原始值
            unsafe { std::env::set_var(self.key, value) };
        } else {
            // 变量原本不存在，删除当前设置
            unsafe { std::env::remove_var(self.key) };
        }
    }
}

/// 获取全局环境变量互斥锁
///
/// 返回一个静态互斥锁的守卫，用于同步对环境变量的访问。
/// 由于 `std::env::set_var` 和 `std::env::remove_var` 不是线程安全的，
/// 在并发测试中使用此锁可以防止竞态条件。
///
/// # 返回值
///
/// 返回一个 `MutexGuard`，当它被丢弃时自动释放锁
///
/// # 用法模式
///
/// ```ignore
/// {
///     let _lock = env_lock();
///     let _guard = EnvGuard::set("MY_VAR", Some("value"));
///     // 在此作用域内，环境变量的修改是线程安全的
/// }
/// // 锁和守卫都已释放
/// ```
///
/// # 实现细节
///
/// 使用 `OnceLock` 实现懒初始化的静态互斥锁，
/// 确保整个测试过程中使用同一个锁实例。
fn env_lock() -> std::sync::MutexGuard<'static, ()> {
    use std::sync::{Mutex, OnceLock};
    // 使用 OnceLock 确保互斥锁只初始化一次
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    // 获取或初始化互斥锁，然后锁定并返回守卫
    LOCK.get_or_init(|| Mutex::new(())).lock().unwrap()
}
