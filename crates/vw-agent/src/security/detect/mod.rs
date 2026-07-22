//! # 安全特性自动检测模块
//!
//! 本模块负责自动检测和创建可用的安全沙箱后端。它会根据系统环境和配置自动选择
//! 最合适的沙箱实现，确保代理运行时在具备安全隔离能力的环境中执行。
//!
//! ## 核心功能
//!
//! - **自动检测**：扫描系统中可用的沙箱技术并选择最优方案
//! - **配置驱动**：支持通过配置显式指定沙箱后端类型
//! - **降级策略**：在首选后端不可用时，自动降级到应用层安全控制
//!
//! ## 支持的沙箱后端
//!
//! 1. **Landlock** - Linux 内核原生沙箱（需要 5.13+ 内核）
//! 2. **Firejail** - Linux 用户空间沙箱工具
//! 3. **Bubblewrap** - 轻量级命名空间容器（支持 Linux 和 macOS）
//! 4. **Docker** - 容器化隔离（跨平台）
//!
//! ## 使用示例
//!
//! ```rust,ignore
//! use crate::app::agent::config::SecurityConfig;
//! use crate::app::agent::security::detect::create_sandbox;
//!
//! let config = SecurityConfig::default();
//! let sandbox = create_sandbox(&config);
//!
//! // 使用沙箱执行命令
//! sandbox.execute(|| {
//!     // 受保护的代码块
//! }).unwrap();
//! ```

use super::traits::Sandbox;
use crate::app::agent::config::{SandboxBackend, SecurityConfig};
use std::sync::Arc;

fn unavailable_backend_message(backend: &SandboxBackend) -> Option<&'static str> {
    match backend {
        SandboxBackend::Landlock => {
            Some("Landlock requested but not available, falling back to application-layer")
        }
        SandboxBackend::Firejail => {
            Some("Firejail requested but not available, falling back to application-layer")
        }
        SandboxBackend::Bubblewrap => {
            Some("Bubblewrap requested but not available, falling back to application-layer")
        }
        SandboxBackend::Docker => {
            Some("Docker requested but not available, falling back to application-layer")
        }
        SandboxBackend::Auto | SandboxBackend::None => None,
    }
}

fn unavailable_backend_fallback(backend: &SandboxBackend) -> Arc<dyn Sandbox> {
    let message = unavailable_backend_message(backend)
        .expect("fallback message should exist for explicit sandbox backends");
    tracing::warn!("{}", message);
    Arc::new(super::traits::NoopSandbox)
}

fn application_layer_fallback() -> Arc<dyn Sandbox> {
    tracing::info!("No sandbox backend available, using application-layer security");
    Arc::new(super::traits::NoopSandbox)
}

/// 根据配置创建沙箱实例
///
/// 此函数是创建沙箱的主要入口点。它会根据配置中的 `backend` 字段和 `enabled` 标志
/// 决定使用哪种沙箱实现。支持显式指定后端类型或自动检测最佳可用选项。
///
/// # 参数
///
/// - `config` - 安全配置引用，包含沙箱后端类型和启用状态等信息
///
/// # 返回值
///
/// 返回一个实现了 `Sandbox` trait 的智能指针。可能的具体类型包括：
/// - 特定沙箱实现（如 `LandlockSandbox`、`DockerSandbox` 等）
/// - `NoopSandbox` - 当沙箱被禁用或所有后端都不可用时的空操作实现
///
/// # 选择逻辑
///
/// 1. 如果配置中明确禁用了沙箱（`enabled == Some(false)`）或后端为 `None`，
///    直接返回 `NoopSandbox`
/// 2. 如果指定了特定后端，尝试创建该类型的沙箱
/// 3. 如果指定的后端不可用，发出警告并降级到 `NoopSandbox`
/// 4. 如果后端为 `Auto`，调用自动检测逻辑选择最佳可用沙箱
///
/// # 示例
///
/// ```rust,ignore
/// use crate::app::agent::config::{SecurityConfig, SandboxBackend};
///
/// // 使用自动检测
/// let config = SecurityConfig {
///     sandbox: SandboxConfig {
///         backend: SandboxBackend::Auto,
///         enabled: Some(true),
///         ..Default::default()
///     },
///     ..Default::default()
/// };
/// let sandbox = create_sandbox(&config);
///
/// // 强制使用 Docker
/// let config = SecurityConfig {
///     sandbox: SandboxConfig {
///         backend: SandboxBackend::Docker,
///         ..Default::default()
///     },
///     ..Default::default()
/// };
/// let sandbox = create_sandbox(&config);
/// ```
pub fn create_sandbox(config: &SecurityConfig) -> Arc<dyn Sandbox> {
    let backend = &config.sandbox.backend;

    // 如果明确禁用了沙箱，返回空操作实现
    // 这适用于显式禁用（enabled == false）或后端类型为 None 的情况
    if matches!(backend, SandboxBackend::None) || config.sandbox.enabled == Some(false) {
        return Arc::new(super::traits::NoopSandbox);
    }

    // 根据指定的后端类型创建相应的沙箱
    match backend {
        // Landlock 后端：Linux 内核原生沙箱
        // 特点：性能高，无外部依赖，但需要较新的内核版本
        SandboxBackend::Landlock => {
            #[cfg(feature = "sandbox-landlock")]
            {
                #[cfg(target_os = "linux")]
                {
                    if let Ok(sandbox) = super::landlock::LandlockSandbox::new() {
                        return Arc::new(sandbox);
                    }
                }
            }
            unavailable_backend_fallback(backend)
        }

        // Firejail 后端：Linux 用户空间沙箱工具
        // 特点：功能丰富，需要系统安装 firejail 命令
        SandboxBackend::Firejail => {
            #[cfg(target_os = "linux")]
            {
                if let Ok(sandbox) = super::firejail::FirejailSandbox::new() {
                    return Arc::new(sandbox);
                }
            }
            unavailable_backend_fallback(backend)
        }

        // Bubblewrap 后端：轻量级命名空间容器
        // 特点：轻量快速，支持 Linux 和 macOS（通过 homebrew 安装）
        SandboxBackend::Bubblewrap => {
            #[cfg(feature = "sandbox-bubblewrap")]
            {
                #[cfg(any(target_os = "linux", target_os = "macos"))]
                {
                    if let Ok(sandbox) = super::bubblewrap::BubblewrapSandbox::new() {
                        return Arc::new(sandbox);
                    }
                }
            }
            unavailable_backend_fallback(backend)
        }

        // Docker 后端：容器化隔离
        // 特点：跨平台，隔离度最高，但资源开销较大
        SandboxBackend::Docker => {
            if let Ok(sandbox) = super::docker::DockerSandbox::new() {
                return Arc::new(sandbox);
            }
            unavailable_backend_fallback(backend)
        }

        // 自动模式：根据系统环境智能选择最佳沙箱
        // 会调用 detect_best_sandbox() 进行探测
        SandboxBackend::Auto | SandboxBackend::None => detect_best_sandbox(),
    }
}

/// 自动检测系统中可用的最佳沙箱后端
///
/// 此函数会按照性能和可用性的优先级顺序探测各种沙箱技术，
/// 返回第一个成功初始化的沙箱实现。
///
/// # 检测优先级
///
/// ## Linux 系统
///
/// 1. **Landlock** - 最高优先级，因为它是内核原生实现，性能最优且无外部依赖
/// 2. **Firejail** - 次优先级，需要用户空间工具支持
/// 3. **Docker** - 最后尝试，作为通用后备方案
///
/// ## macOS 系统
///
/// 1. **Bubblewrap** - 通过 Homebrew 安装，轻量且易用
/// 2. **Docker** - 作为后备方案
///
/// # 返回值
///
/// 返回成功探测到的第一个沙箱实例。如果所有沙箱都不可用，
/// 返回 `NoopSandbox` 作为降级方案。
///
/// # 日志输出
///
/// 成功检测到沙箱时会记录 INFO 级别日志，包括沙箱类型和相关信息。
/// 如果没有可用的沙箱后端，会记录使用应用层安全的信息。
fn detect_best_sandbox() -> Arc<dyn Sandbox> {
    // Linux 系统的检测逻辑
    #[cfg(target_os = "linux")]
    {
        // 优先尝试 Landlock
        // 原因：内核原生实现，无需外部依赖，性能最优
        #[cfg(feature = "sandbox-landlock")]
        {
            if let Ok(sandbox) = super::landlock::LandlockSandbox::probe() {
                tracing::info!("Landlock sandbox enabled (Linux kernel 5.13+)");
                return Arc::new(sandbox);
            }
        }

        // 次选尝试 Firejail
        // 原因：成熟稳定，功能丰富，但需要外部工具
        if let Ok(sandbox) = super::firejail::FirejailSandbox::probe() {
            tracing::info!("Firejail sandbox enabled");
            return Arc::new(sandbox);
        }
    }

    // macOS 系统的检测逻辑
    #[cfg(target_os = "macos")]
    {
        // 尝试 Bubblewrap
        // 原因：轻量级，可通过 Homebrew 安装
        #[cfg(feature = "sandbox-bubblewrap")]
        {
            if let Ok(sandbox) = super::bubblewrap::BubblewrapSandbox::probe() {
                tracing::info!("Bubblewrap sandbox enabled");
                return Arc::new(sandbox);
            }
        }
    }

    // Docker 作为跨平台的后备方案
    // 原因：虽然资源开销较大，但兼容性最好
    if let Ok(sandbox) = super::docker::DockerSandbox::probe() {
        tracing::info!("Docker sandbox enabled");
        return Arc::new(sandbox);
    }

    // 所有沙箱都不可用时的降级方案
    // 使用应用层安全控制，仅提供基础的安全检查
    application_layer_fallback()
}

// 单元测试模块
// 测试代码位于 tests.rs 文件中，主要测试沙箱自动检测和创建逻辑
#[cfg(test)]
#[path = "tests.rs"]
mod tests;
