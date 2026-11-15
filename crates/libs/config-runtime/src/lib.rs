//! 运行时适配器配置模块
//!
//! 本模块定义了代理运行时的配置结构，支持两种运行时类型：
//! - `native`: 原生系统运行时（默认）
//! - `docker`: Docker 容器隔离运行时
//!
//! # 配置结构
//!
//! 运行时配置通过 `[runtime]` 配置段进行设置，包含：
//! - 运行时类型选择（`kind`）
//! - Docker 运行时特定配置（`[runtime.docker]`）
//! - 推理能力覆盖设置（`reasoning_enabled`、`reasoning_level`）
//!
//! # 安全性
//!
//! - Docker 运行时默认采用只读根文件系统和网络隔离
//! - 所有运行时类型均采用最小权限原则

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// 运行时适配器主配置（`[runtime]` 配置段）
///
/// 定义代理的执行环境类型及相关配置。运行时负责隔离和执行工具调用，
/// 是系统安全边界的关键组成部分。
///
/// # 示例配置
///
/// ```toml
/// [runtime]
/// kind = "docker"
/// reasoning_enabled = true
///
/// [runtime.docker]
/// image = "alpine:3.20"
/// memory_limit_mb = 512
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RuntimeConfig {
    /// 运行时类型（`native` | `docker`）
    ///
    /// - `native`: 直接在宿主机执行，性能最高但隔离性最弱
    /// - `docker`: 在 Docker 容器中执行，提供进程级隔离
    #[serde(default = "default_runtime_kind")]
    pub kind: String,

    /// Docker 运行时配置（当 `kind = "docker"` 时生效）
    ///
    /// 控制 Docker 容器的资源限制、网络配置和文件系统挂载等。
    #[serde(default)]
    pub docker: DockerRuntimeConfig,

    /// 全局推理能力覆盖开关（用于支持显式控制的 Provider）
    ///
    /// 推理能力允许模型在响应前进行内部思考链推理。
    ///
    /// - `None`: 使用 Provider 默认行为
    /// - `Some(true)`: 在支持时请求启用推理/思考功能
    /// - `Some(false)`: 在支持时禁用推理/思考功能
    #[serde(default)]
    pub reasoning_enabled: Option<bool>,

    /// 推理级别兼容性别名（已弃用，保留用于向后兼容）
    ///
    /// # 迁移说明
    ///
    /// - 推荐配置键: `provider.reasoning_level`
    /// - 兼容配置键: `runtime.reasoning_level`（已弃用）
    /// - 当两者同时设置时，Provider 级别的值优先
    #[serde(default)]
    pub reasoning_level: Option<String>,
}

/// Docker 运行时配置（`[runtime.docker]` 配置段）
///
/// 定义 Docker 容器的执行环境参数，包括镜像、资源限制和安全设置。
/// 默认配置采用最小权限原则，提供安全的隔离环境。
///
/// # 安全默认值
///
/// - 网络模式默认为 `none`（无网络访问）
/// - 根文件系统默认为只读
/// - 默认启用工作区挂载
///
/// # 示例配置
///
/// ```toml
/// [runtime.docker]
/// image = "alpine:3.20"
/// network = "bridge"
/// memory_limit_mb = 1024
/// cpu_limit = 2.0
/// read_only_rootfs = true
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DockerRuntimeConfig {
    /// 执行 Shell 命令的 Docker 镜像
    ///
    /// 镜像应包含执行工具所需的基本命令（如 `sh`、`ls` 等）。
    /// 默认使用轻量级的 Alpine Linux 镜像。
    #[serde(default = "default_docker_image")]
    pub image: String,

    /// Docker 网络模式
    ///
    /// 常用选项：
    /// - `none`: 无网络访问（最安全）
    /// - `bridge`: 使用 Docker 默认网桥
    /// - `host`: 使用宿主机网络（需谨慎使用）
    #[serde(default = "default_docker_network")]
    pub network: String,

    /// 内存限制（单位：MB）
    ///
    /// - `Some(n)`: 限制容器最大使用 n MB 内存
    /// - `None`: 不设置显式内存限制
    #[serde(default = "default_docker_memory_limit_mb")]
    pub memory_limit_mb: Option<u64>,

    /// CPU 限制
    ///
    /// - `Some(n)`: 限制容器最多使用 n 个 CPU 核心
    /// - `None`: 不设置显式 CPU 限制
    #[serde(default = "default_docker_cpu_limit")]
    pub cpu_limit: Option<f64>,

    /// 是否将根文件系统挂载为只读
    ///
    /// 启用后容器无法修改系统文件，提升安全性。
    /// 默认启用。
    #[serde(default = "default_true")]
    pub read_only_rootfs: bool,

    /// 是否将工作区挂载到容器的 `/workspace` 目录
    ///
    /// 启用后代理可以在容器内访问和修改工作区文件。
    /// 默认启用。
    #[serde(default = "default_true")]
    pub mount_workspace: bool,

    /// 工作区根目录白名单
    ///
    /// 用于验证 Docker 挂载路径的合法性。
    /// 只有在此列表中的目录才允许被挂载到容器。
    #[serde(default)]
    pub allowed_workspace_roots: Vec<String>,
}

// ============================================================================
// 默认值函数
// ============================================================================

/// 返回默认运行时类型
fn default_runtime_kind() -> String {
    "native".into()
}

/// 返回默认 Docker 镜像
fn default_docker_image() -> String {
    "alpine:3.20".into()
}

/// 返回默认 Docker 网络模式
fn default_docker_network() -> String {
    "none".into()
}

/// 返回默认 Docker 内存限制
fn default_docker_memory_limit_mb() -> Option<u64> {
    Some(512)
}

/// 返回默认 Docker CPU 限制
fn default_docker_cpu_limit() -> Option<f64> {
    Some(1.0)
}

/// 返回默认布尔值 `true`
fn default_true() -> bool {
    true
}

// ============================================================================
// 默认实现
// ============================================================================

impl Default for DockerRuntimeConfig {
    fn default() -> Self {
        Self {
            image: default_docker_image(),
            network: default_docker_network(),
            memory_limit_mb: default_docker_memory_limit_mb(),
            cpu_limit: default_docker_cpu_limit(),
            read_only_rootfs: true,
            mount_workspace: true,
            allowed_workspace_roots: Vec::new(),
        }
    }
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            kind: default_runtime_kind(),
            docker: DockerRuntimeConfig::default(),
            reasoning_enabled: None,
            reasoning_level: None,
        }
    }
}
#[cfg(test)]
#[path = "runtime_tests.rs"]
mod runtime_tests;
