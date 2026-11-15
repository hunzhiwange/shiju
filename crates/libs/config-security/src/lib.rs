//! 安全与自治策略配置模块。
//!
//! 本模块定义代理在执行工具、访问文件系统、使用沙箱、记录审计日志、触发 OTP
//! 校验等场景下的安全相关配置。
//!
//! # 主要关注点
//!
//! - 代理自治等级与审批策略
//! - 工具调用的命令、路径与风险边界
//! - 沙箱与资源限制
//! - 审计、急停、OTP 与异常系统调用监控

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

mod autonomy;
mod bash;
mod monitoring;
mod sandbox;

pub use autonomy::{
    AutonomyConfig, AutonomyLevel, IdentityConfig, NonCliNaturalLanguageApprovalMode, SecretsConfig,
    ShellRedirectPolicy,
};
pub use bash::{BashPolicyChecksConfig, BashSecurityConfig, BashValidatorChecksConfig};
pub use monitoring::{AuditConfig, EstopConfig, OtpConfig, OtpMethod, SyscallAnomalyConfig};
pub use sandbox::{ResourceLimitsConfig, SandboxBackend, SandboxConfig};

/// 顶层安全配置。
///
/// 聚合沙箱、资源限制、审计、OTP、急停与异常调用检测等子系统配置。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SecurityConfig {
    /// Bash 命令执行前安全检查配置。
    #[serde(default)]
    pub bash: BashSecurityConfig,

    /// 沙箱配置。
    #[serde(default)]
    pub sandbox: SandboxConfig,

    /// 资源限制配置。
    #[serde(default)]
    pub resources: ResourceLimitsConfig,

    /// 审计配置。
    #[serde(default)]
    pub audit: AuditConfig,

    /// OTP 配置。
    #[serde(default)]
    pub otp: OtpConfig,

    /// 急停配置。
    #[serde(default)]
    pub estop: EstopConfig,

    /// 系统调用异常检测配置。
    #[serde(default)]
    pub syscall_anomaly: SyscallAnomalyConfig,

    /// 是否启用蜜罐令牌保护。
    #[serde(default = "default_true")]
    pub canary_tokens: bool,

    /// 是否启用语义防护。
    #[serde(default)]
    pub semantic_guard: bool,

    /// 语义防护使用的集合名称。
    #[serde(default = "default_semantic_guard_collection")]
    pub semantic_guard_collection: String,

    /// 语义防护判定阈值。
    #[serde(default = "default_semantic_guard_threshold")]
    pub semantic_guard_threshold: f64,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            bash: BashSecurityConfig::default(),
            sandbox: SandboxConfig::default(),
            resources: ResourceLimitsConfig::default(),
            audit: AuditConfig::default(),
            otp: OtpConfig::default(),
            estop: EstopConfig::default(),
            syscall_anomaly: SyscallAnomalyConfig::default(),
            canary_tokens: default_true(),
            semantic_guard: false,
            semantic_guard_collection: default_semantic_guard_collection(),
            semantic_guard_threshold: default_semantic_guard_threshold(),
        }
    }
}

fn default_semantic_guard_collection() -> String {
    "semantic_guard".to_string()
}

fn default_semantic_guard_threshold() -> f64 {
    0.82
}

fn default_true() -> bool {
    true
}

#[cfg(test)]
mod tests;
