//! # 安全模块 (Security Module)
//!
//! 本模块提供 VibeWindow 代理运行时的核心安全防护机制，涵盖多层防御策略和隔离机制。
//!
//! ## 模块职责
//!
//! - **审计与日志**：记录安全相关事件和操作轨迹
//! - **沙箱隔离**：通过多种沙箱技术隔离代理执行环境
//! - **密钥管理**：安全存储和处理敏感信息
//! - **输入防护**：防御提示注入和恶意输入
//! - **访问控制**：配对验证和自主级别控制
//! - **异常检测**：系统调用监控和泄漏检测
//!
//! ## 子模块概览
//!
//! | 模块 | 功能描述 |
//! |------|----------|
//! | `audit` | 安全审计事件记录 |
//! | `bubblewrap` | Bubblewrap 沙箱实现（可选） |
//! | `canary_guard` | 金丝雀令牌防护 |
//! | `detect` | 沙箱创建和检测工具 |
//! | `docker` | Docker 容器隔离 |
//! | `domain_matcher` | 域名匹配器（防御提示注入） |
//! | `estop` | 紧急停止机制 |
//! | `firejail` | Firejail 沙箱（仅 Linux） |
//! | `landlock` | Landlock LSM 沙箱（可选） |
//! | `leak_detector` | 敏感信息泄漏检测 |
//! | `otp` | 一次性密码验证 |
//! | `pairing` | 设备配对防护 |
//! | `policy` | 安全策略定义 |
//! | `prompt_guard` | 提示防护器 |
//! | `secrets` | 密钥安全存储 |
//! | `semantic_guard` | 语义防护器 |
//! | `syscall_anomaly` | 系统调用异常检测 |
//! | `traits` | 沙箱 trait 定义 |
//!
//! ## 设计原则
//!
//! 1. **默认拒绝**：所有访问边界采用默认拒绝策略
//! 2. **最小权限**：限制网络/文件系统/命令行范围
//! 3. **快速失败**：对不安全状态优先显式错误
//! 4. **可审计性**：所有安全相关操作可追踪

/// 安全审计模块：记录安全相关事件（如认证失败、策略违规等）
pub mod audit;

/// Bubblewrap 沙箱模块：使用 Flatpak 的 Bubblewrap 实现轻量级命名空间隔离
/// 需启用 `sandbox-bubblewrap` 特性
#[cfg(feature = "sandbox-bubblewrap")]
pub mod bubblewrap;

/// 金丝雀防护模块：通过嵌入金丝雀令牌检测未授权的数据访问或泄漏
pub mod canary_guard;

/// 沙箱检测模块：提供沙箱创建和检测工具函数
pub mod detect;

/// Docker 隔离模块：通过 Docker 容器提供进程和资源隔离
pub mod docker;

/// 域名匹配模块：域名匹配器，用于提示注入防御
/// 贡献自 RustyClaw 项目（MIT 许可证）
pub mod domain_matcher;

/// 紧急停止模块：提供紧急停止和恢复机制，用于危急情况下的快速干预
pub mod estop;

/// Tenant-scoped gateway skey metadata store in the shared database.
pub mod gateway_skey_store;

/// Firejail 沙箱模块：使用 Firejail 实现细粒度的进程隔离
/// 仅在 Linux 系统上可用
#[cfg(target_os = "linux")]
pub mod firejail;

/// Landlock 沙箱模块：使用 Linux Landlock LSM 实现文件系统访问控制
/// 需启用 `sandbox-landlock` 特性
#[cfg(feature = "sandbox-landlock")]
pub mod landlock;

/// 泄漏检测模块：检测日志、输出中是否包含敏感信息（如密钥、令牌）
pub mod leak_detector;

/// OTP 验证模块：一次性密码验证器，用于双因素认证
pub mod otp;

/// 配对防护模块：管理设备配对流程，防止未授权设备连接
pub mod pairing;

/// 安全策略模块：定义自主级别、Shell 重定向策略等安全配置
pub mod policy;

/// 提示防护模块：提示注入防御器，检测和过滤恶意提示
pub mod prompt_guard;

/// 密钥存储模块：安全存储和管理敏感配置（如 API 密钥）
pub mod secrets;

/// 语义防护模块：基于语义分析的安全防护器
pub mod semantic_guard;

/// 系统调用异常检测模块：监控系统调用模式，检测异常行为
pub mod syscall_anomaly;

/// Trait 定义模块：定义沙箱和相关安全组件的核心接口
pub mod traits;

// ============================================================================
// 公开 API 重导出
// ============================================================================

/// 重导出审计相关类型
#[allow(unused_imports)]
pub use audit::{AuditEvent, AuditEventType, AuditLogger};

/// 重导出金丝雀防护器
pub use canary_guard::CanaryGuard;

/// 重导出沙箱创建函数（根据平台和特性自动选择合适的沙箱实现）
#[allow(unused_imports)]
pub use detect::create_sandbox;

/// 重导出域名匹配器
pub use domain_matcher::DomainMatcher;

/// 重导出紧急停止相关类型
#[allow(unused_imports)]
pub use estop::{EstopLevel, EstopManager, EstopState, ResumeSelector};

/// 重导出 OTP 验证器
#[allow(unused_imports)]
pub use otp::OtpValidator;

/// 重导出配对防护器
#[allow(unused_imports)]
pub use pairing::PairingGuard;

/// 重导出安全策略相关类型
pub use policy::{AutonomyLevel, SecurityPolicy, ShellRedirectPolicy};

/// 重导出密钥存储
#[allow(unused_imports)]
pub use secrets::SecretStore;

/// 重导出系统调用异常检测相关类型
#[allow(unused_imports)]
pub use syscall_anomaly::{SyscallAnomalyAlert, SyscallAnomalyDetector, SyscallAnomalyKind};

/// 重导出沙箱 trait 和空操作实现（用于测试或无沙箱环境）
#[allow(unused_imports)]
pub use traits::{NoopSandbox, Sandbox};

/// 重导出泄漏检测相关类型（提示注入防御组件）
#[allow(unused_imports)]
pub use leak_detector::{LeakDetector, LeakResult};

/// 重导出提示防护相关类型（提示注入防御组件）
#[allow(unused_imports)]
pub use prompt_guard::{GuardAction, GuardResult, PromptGuard};

/// 重导出语义防护器及其启动状态
pub use semantic_guard::{GuardCorpusUpdateReport, SemanticGuard, SemanticGuardStartupStatus};

// ============================================================================
// 工具函数
// ============================================================================

/// 脱敏敏感值，用于安全日志记录
///
/// 该函数通过仅保留前 4 个字符并添加 `"***"` 后缀来脱敏敏感信息，
/// 确保日志中不会泄露完整的密钥、令牌等敏感数据。
///
/// # 设计目的
///
/// 1. **日志安全**：防止敏感信息出现在日志文件中
/// 2. **可追溯性**：保留前缀字符便于调试时识别具体是哪个密钥
/// 3. **静态分析友好**：故意中断数据流污点链，避免静态分析工具误报
///
/// # 参数
///
/// * `value` - 需要脱敏的敏感字符串
///
/// # 返回值
///
/// 返回脱敏后的字符串：
/// - 如果输入长度 ≤ 4，返回 `"***"`
/// - 如果输入长度 > 4，返回前 4 个字符 + `"***"`
///
/// # 示例
///
/// ```rust
/// use vibe_agent::security::redact;
///
/// // 脱敏 API 密钥
/// let api_key = "sk-1234567890abcdef";
/// let redacted = redact(api_key);
/// assert_eq!(redacted, "sk-1***");
///
/// // 短字符串处理
/// let short = "abc";
/// assert_eq!(redact(short), "***");
///
/// // 空字符串处理
/// assert_eq!(redact(""), "***");
/// ```
///
/// # 安全说明
///
/// 此函数专门设计用于日志输出场景。在生产环境中，所有可能包含
/// 敏感信息的变量在记录到日志前都应通过此函数处理。
pub fn redact(value: &str) -> String {
    // 长度判断：短字符串直接返回固定脱敏值
    if value.len() <= 4 {
        "***".to_string()
    } else {
        // 长字符串保留前 4 字符作为标识，后跟脱敏标记
        format!("{}***", &value[..4])
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
