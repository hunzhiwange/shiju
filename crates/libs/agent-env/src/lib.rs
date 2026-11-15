//! 可注入的环境变量上下文。
//!
//! 本模块为运行时代码提供环境变量快照，而不是在每个调用点直接读取进程环境。
//! 这样测试或局部执行上下文可以注入确定性的变量集合，同时默认路径仍使用启动时
//! 捕获的进程环境。

use agent_context::context;
use std::collections::HashMap;
use std::sync::LazyLock;
use std::sync::Mutex;

/// 环境变量键值快照。
type Vars = HashMap<String, String>;

/// 捕获当前进程环境变量。
///
/// 返回值：
/// 返回一个拥有所有键值的字符串 Map；非 UTF-8 键值会以 lossy 方式转换，保持
/// 调用方接口简单且可序列化。
fn snapshot() -> Vars {
    std::env::vars_os()
        .map(|(k, v)| (k.to_string_lossy().to_string(), v.to_string_lossy().to_string()))
        .collect()
}

static ENV_CONTEXT: LazyLock<context::Context<Mutex<Vars>>> =
    LazyLock::new(|| context::create("env"));
static DEFAULT: LazyLock<Mutex<Vars>> = LazyLock::new(|| Mutex::new(snapshot()));

/// 使用当前上下文中的环境变量集合执行闭包。
///
/// 参数：
/// - `f`：接收环境变量互斥锁的闭包。
///
/// 返回值：
/// 返回闭包执行结果。若当前没有注入上下文，则回退到默认快照。
fn with_env<R>(f: impl FnOnce(&Mutex<Vars>) -> R) -> R {
    if let Ok(v) = ENV_CONTEXT.use_value() {
        return f(&v);
    }
    f(&DEFAULT)
}

/// 在新的环境变量快照上下文中执行闭包。
///
/// 参数：
/// - `f`：在隔离环境上下文内执行的闭包。
///
/// 返回值：
/// 返回闭包执行结果。
pub fn provide<R>(f: impl FnOnce() -> R) -> R {
    ENV_CONTEXT.provide(Mutex::new(snapshot()), f)
}

/// 使用指定变量集合创建环境上下文并执行闭包。
///
/// 参数：
/// - `vars`：要注入的环境变量集合。
/// - `f`：在该环境上下文内执行的闭包。
///
/// 返回值：
/// 返回闭包执行结果。该函数主要用于测试和需要确定性环境的局部运行。
pub fn provide_with<R>(vars: Vars, f: impl FnOnce() -> R) -> R {
    ENV_CONTEXT.provide(Mutex::new(vars), f)
}

/// 读取环境变量。
///
/// 参数：
/// - `key`：变量名。
///
/// 返回值：
/// 变量存在时返回其值；不存在或环境锁被污染时返回 `None`。
pub fn get(key: &str) -> Option<String> {
    with_env(|m| m.lock().ok().and_then(|env| env.get(key).cloned()))
}

/// 获取当前环境上下文的完整变量快照。
///
/// 返回值：
/// 返回当前变量集合的克隆；环境锁被污染时返回空集合，避免把锁状态泄露给调用方。
pub fn all() -> Vars {
    with_env(|m| m.lock().ok().map(|env| env.clone()).unwrap_or_default())
}

/// 设置或覆盖环境变量。
///
/// 参数：
/// - `key`：变量名。
/// - `value`：变量值。
///
/// 错误处理：
/// 若环境锁被污染，写入会被忽略；该模块把环境上下文作为辅助状态，不让锁错误
/// 扩散到业务路径。
pub fn set(key: impl Into<String>, value: impl Into<String>) {
    let key = key.into();
    let value = value.into();
    with_env(|m| {
        if let Ok(mut env) = m.lock() {
            env.insert(key, value);
        }
    });
}

/// 删除环境变量。
///
/// 参数：
/// - `key`：要删除的变量名。
///
/// 错误处理：
/// 若环境锁被污染，删除会被忽略。
pub fn remove(key: &str) {
    with_env(|m| {
        if let Ok(mut env) = m.lock() {
            env.remove(key);
        }
    });
}

#[cfg(test)]
mod tests;
