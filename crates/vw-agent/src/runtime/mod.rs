//! 运行时适配器模块
//!
//! 本模块提供运行时抽象层，支持 native 和 docker 执行环境。
//! 采用 trait + 工厂模式，允许代理在不同运行时环境中执行工具和任务。
//!
//! # 核心组件
//!
//! - [`RuntimeAdapter`] - 运行时适配器 trait，定义所有运行时的统一接口
//! - [`NativeRuntime`] - 本机运行时，直接在宿主系统执行
//! - [`DockerRuntime`] - Docker 容器运行时，提供隔离环境
//!
//! # 使用示例
//!
//! ```no_run
//! use vibe_agent::app::agent::config::RuntimeConfig;
//! use vibe_agent::app::agent::runtime::create_runtime;
//!
//! let config = RuntimeConfig {
//!     kind: "native".to_string(),
//!     // ... 其他配置字段
//! };
//! let runtime = create_runtime(&config)?;
//! # Ok::<(), anyhow::Error>(())
//! ```

pub mod docker;
pub mod native;
pub mod traits;

pub use docker::DockerRuntime;
pub use native::NativeRuntime;
pub use traits::RuntimeAdapter;

use crate::app::agent::config::RuntimeConfig;

/// 根据配置创建对应的运行时实例
///
/// 本函数是运行时工厂，根据配置中的 `kind` 字段创建并返回相应的运行时适配器。
/// 返回的是 trait 对象，调用方无需关心具体实现细节。
///
/// # 参数
///
/// - `config` - 运行时配置，包含 `kind` 及各运行时特定的配置项
///
/// # 返回值
///
/// - `Ok(Box<dyn RuntimeAdapter>)` - 成功时返回对应运行时的 trait 对象
/// - `Err` - 配置无效或不支持的运行时类型时返回错误
///
/// # 支持的运行时类型
///
/// - `"native"` - 本机运行时，直接在宿主系统执行（默认推荐）
/// - `"docker"` - Docker 容器运行时，提供进程级隔离
///
/// # 错误
///
/// - 配置为 `"cloudflare"` 时返回未实现错误（保留用于未来）
/// - `kind` 为空字符串时返回配置错误
/// - `kind` 为未知值时返回不支持的错误
///
/// # 示例
///
/// ```no_run
/// use vibe_agent::app::agent::config::RuntimeConfig;
/// use vibe_agent::app::agent::runtime::create_runtime;
///
/// // 创建本机运行时
/// let config = RuntimeConfig {
///     kind: "native".to_string(),
///     // ...
/// };
/// let runtime = create_runtime(&config)?;
///
/// // 创建 Docker 运行时
/// let config = RuntimeConfig {
///     kind: "docker".to_string(),
///     // ...
/// };
/// let runtime = create_runtime(&config)?;
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn create_runtime(config: &RuntimeConfig) -> anyhow::Result<Box<dyn RuntimeAdapter>> {
    match config.kind.as_str() {
        "native" => Ok(Box::new(NativeRuntime::new())),
        "docker" => Ok(Box::new(DockerRuntime::new(config.docker.clone()))),
        "cloudflare" => anyhow::bail!(
            "runtime.kind='cloudflare' is not implemented yet. Use runtime.kind='native' for now."
        ),
        other if other.trim().is_empty() => {
            anyhow::bail!("runtime.kind cannot be empty. Supported values: native, docker")
        }
        other => {
            anyhow::bail!("Unknown runtime kind '{other}'. Supported values: native, docker")
        }
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
