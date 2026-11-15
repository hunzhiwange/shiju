use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// 沙箱配置。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SandboxConfig {
    /// 是否启用沙箱；`None` 表示由运行时自动决定。
    #[serde(default = "default_sandbox_enabled")]
    pub enabled: Option<bool>,

    /// 沙箱后端类型。
    #[serde(default)]
    pub backend: SandboxBackend,

    /// 传递给 firejail 的额外参数。
    #[serde(default)]
    pub firejail_args: Vec<String>,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            enabled: default_sandbox_enabled(),
            backend: SandboxBackend::default(),
            firejail_args: Vec::new(),
        }
    }
}

fn default_sandbox_enabled() -> Option<bool> {
    Some(false)
}

/// 沙箱后端类型。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum SandboxBackend {
    /// 自动选择最合适的后端。
    Auto,
    /// 使用 Landlock。
    Landlock,
    /// 使用 Firejail。
    Firejail,
    /// 使用 Bubblewrap。
    Bubblewrap,
    /// 使用 Docker。
    Docker,
    /// 不使用沙箱。
    #[default]
    None,
}

/// 资源限制配置。
///
/// 用于限制单次执行可用的内存、CPU 时间和子进程数量。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ResourceLimitsConfig {
    /// 最大内存限制，单位为 MB。
    #[serde(default = "default_max_memory_mb")]
    pub max_memory_mb: u32,

    /// 最大 CPU 时间限制，单位为秒。
    #[serde(default = "default_max_cpu_time_seconds")]
    pub max_cpu_time_seconds: u64,

    /// 最多允许创建的子进程数。
    #[serde(default = "default_max_subprocesses")]
    pub max_subprocesses: u32,

    /// 是否启用内存监控。
    #[serde(default = "default_memory_monitoring_enabled")]
    pub memory_monitoring: bool,
}

fn default_max_memory_mb() -> u32 {
    512
}

fn default_max_cpu_time_seconds() -> u64 {
    60
}

fn default_max_subprocesses() -> u32 {
    10
}

fn default_memory_monitoring_enabled() -> bool {
    true
}

impl Default for ResourceLimitsConfig {
    fn default() -> Self {
        Self {
            max_memory_mb: default_max_memory_mb(),
            max_cpu_time_seconds: default_max_cpu_time_seconds(),
            max_subprocesses: default_max_subprocesses(),
            memory_monitoring: default_memory_monitoring_enabled(),
        }
    }
}
