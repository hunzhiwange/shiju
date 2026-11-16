//! 技能测试辅助工具模块
//!
//! 本模块提供了用于技能（skills）测试的通用辅助工具和实用函数。
//! 主要用于管理测试环境变量，确保测试之间的隔离性和可重复性。
//!
//! # 主要功能
//!
//! - 提供全局环境锁，防止并发测试修改环境变量导致竞态条件
//! - 提供环境变量守卫（RAII 模式），自动保存和恢复环境变量状态
//!
//! # 使用示例
//!
//! ```rust,ignore
//! use vibe_window::app::agent::skills::tests::helpers::{env_var_lock, EnvVarGuard};
//!
//! // 使用环境锁确保测试串行执行
//! let _lock = env_var_lock().lock().unwrap();
//!
//! // 临时取消设置环境变量，测试结束后自动恢复
//! let _guard = EnvVarGuard::unset("MY_ENV_VAR");
//! ```

use std::sync::{Mutex, OnceLock};

/// 获取技能测试环境锁
///
/// 返回一个静态的全局互斥锁，用于确保涉及环境变量操作的测试
/// 能够串行执行，避免并发测试导致的环境变量竞态条件。
///
/// # 返回值
///
/// 返回一个指向静态 `Mutex<()>` 的引用。该互斥锁在整个程序生命周期内存在，
/// 所有调用此函数的代码都会获得同一个互斥锁实例。
///
/// # 使用场景
///
/// 当测试需要修改或读取环境变量时，应先获取此锁以确保测试隔离：
///
/// ```rust,ignore
/// let _lock = env_var_lock().lock().unwrap();
/// // 在此作用域内，其他测试无法同时获取锁
/// // 可以安全地修改环境变量
/// ```
///
/// # 实现细节
///
/// 使用 `OnceLock` 实现线程安全的单次初始化，确保全局只有一个互斥锁实例。
pub fn env_var_lock() -> &'static Mutex<()> {
    static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    ENV_LOCK.get_or_init(|| Mutex::new(()))
}

/// 环境变量守卫
///
/// 一个 RAII（Resource Acquisition Is Initialization）风格的守卫结构体，
/// 用于临时管理环境变量。在创建时保存环境变量的原始值，
/// 在销毁时自动恢复环境变量到原始状态。
///
/// # 用途
///
/// - 在测试中临时取消设置环境变量
/// - 确保测试结束后环境变量恢复原状
/// - 防止测试污染全局环境状态
///
/// # 示例
///
/// ```rust,ignore
/// {
///     // 取消设置环境变量，原始值被保存
///     let _guard = EnvVarGuard::unset("DATABASE_URL");
///     // 在此作用域内，DATABASE_URL 未设置
///     assert!(std::env::var("DATABASE_URL").is_err());
/// } // _guard 离开作用域，自动恢复 DATABASE_URL 的原始值
/// ```
///
/// # 安全性
///
/// 此结构体使用 `unsafe` 代码块调用 `std::env::set_var` 和 `std::env::remove_var`。
/// 在 Rust 2024 版本及以后，这些函数被标记为不安全，因为修改环境变量
/// 在多线程环境中可能导致未定义行为。使用环境锁（`env_var_lock`）
/// 可以确保线程安全。
pub struct EnvVarGuard {
    /// 环境变量的键名（静态字符串）
    key: &'static str,
    /// 环境变量的原始值（如果存在）
    original: Option<String>,
}

impl EnvVarGuard {
    pub fn set(key: &'static str, value: &str) -> Self {
        let original = std::env::var(key).ok();
        unsafe { std::env::set_var(key, value) };
        Self { key, original }
    }

    /// 取消设置指定的环境变量
    ///
    /// 创建一个守卫，移除指定的环境变量并保存其原始值。
    /// 当守卫被丢弃时，环境变量将恢复到原始状态。
    ///
    /// # 参数
    ///
    /// - `key`: 环境变量的名称，必须是静态生命周期的字符串
    ///
    /// # 返回值
    ///
    /// 返回一个 `EnvVarGuard` 实例，该实例持有原始环境变量值
    ///
    /// # 示例
    ///
    /// ```rust,ignore
    /// // 临时取消设置 PATH 环境变量
    /// let guard = EnvVarGuard::unset("PATH");
    /// // PATH 现在未设置
    /// // 当 guard 被丢弃时，PATH 将恢复原值
    /// ```
    ///
    /// # 安全性
    ///
    /// 此函数使用 `unsafe` 调用 `std::env::remove_var`。
    /// 调用者应确保在单线程或已加锁的环境中使用此函数。
    pub fn unset(key: &'static str) -> Self {
        // 保存环境变量的当前值（如果存在）
        let original = std::env::var(key).ok();
        // 移除环境变量（unsafe：在多线程环境中可能不安全）
        unsafe { std::env::remove_var(key) };
        Self { key, original }
    }
}

impl Drop for EnvVarGuard {
    /// 实现析构函数，自动恢复环境变量
    ///
    /// 当 `EnvVarGuard` 实例离开作用域时，自动调用此方法
    /// 将环境变量恢复到原始状态。
    ///
    /// # 行为
    ///
    /// - 如果原始环境变量存在，则恢复其值
    /// - 如果原始环境变量不存在，则确保该变量被移除
    ///
    /// # 安全性
    ///
    /// 此方法使用 `unsafe` 调用环境变量操作函数。
    /// 依赖调用者在使用 `EnvVarGuard` 前已获取环境锁。
    fn drop(&mut self) {
        if let Some(value) = &self.original {
            // 恢复原始值（unsafe：在多线程环境中可能不安全）
            unsafe { std::env::set_var(self.key, value) };
        } else {
            // 原本不存在，确保移除（unsafe：在多线程环境中可能不安全）
            unsafe { std::env::remove_var(self.key) };
        }
    }
}
