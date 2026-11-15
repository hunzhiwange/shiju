use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Bash 策略检查配置。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct BashPolicyChecksConfig {
    /// 是否启用策略检查。
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// 是否检查只读约束。
    #[serde(default = "default_true")]
    pub read_only: bool,
    /// 是否检查危险 Shell 结构。
    #[serde(default = "default_true")]
    pub unsafe_shell_patterns: bool,
    /// 是否检查命令规则。
    #[serde(default = "default_true")]
    pub command_rules: bool,
    /// 是否检查工作区路径约束。
    #[serde(default = "default_true")]
    pub workspace_paths: bool,
    /// 是否检查禁止路径。
    #[serde(default = "default_true")]
    pub forbidden_paths: bool,
    /// 是否检查风险等级。
    #[serde(default = "default_true")]
    pub risk: bool,
    /// 是否检查重定向。
    #[serde(default = "default_true")]
    pub redirects: bool,
}

impl Default for BashPolicyChecksConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            read_only: true,
            unsafe_shell_patterns: true,
            command_rules: true,
            workspace_paths: true,
            forbidden_paths: true,
            risk: true,
            redirects: true,
        }
    }
}

/// Bash Shell validator 检查配置。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct BashValidatorChecksConfig {
    /// 是否启用 Shell validator 检查。
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// 是否检查空命令。
    #[serde(default = "default_true")]
    pub empty_command: bool,
    /// 是否检查 heredoc。
    #[serde(default = "default_true")]
    pub heredoc: bool,
    /// 是否检查 Git commit 命令。
    #[serde(default = "default_true")]
    pub git_commit: bool,
    /// 是否检查 jq 命令。
    #[serde(default = "default_true")]
    pub jq: bool,
    /// 是否检查 Shell 元字符。
    #[serde(default = "default_true")]
    pub metacharacters: bool,
    /// 是否检查危险变量。
    #[serde(default = "default_true")]
    pub dangerous_vars: bool,
    /// 是否检查命令替换。
    #[serde(default = "default_true")]
    pub substitution: bool,
    /// 是否检查重定向。
    #[serde(default = "default_true")]
    pub redirection: bool,
    /// 是否检查换行注入。
    #[serde(default = "default_true")]
    pub newline_injection: bool,
    /// 是否检查 IFS 注入。
    #[serde(default = "default_true")]
    pub ifs_injection: bool,
    /// 是否检查进程环境访问。
    #[serde(default = "default_true")]
    pub proc_environ: bool,
    /// 是否检查格式异常的命令。
    #[serde(default = "default_true")]
    pub malformed: bool,
    /// 是否检查混淆参数。
    #[serde(default = "default_true")]
    pub obfuscated_flags: bool,
    /// 是否检查反斜杠。
    #[serde(default = "default_true")]
    pub backslash: bool,
    /// 是否检查花括号扩展。
    #[serde(default = "default_true")]
    pub brace_expansion: bool,
    /// 是否检查 Unicode 空白字符。
    #[serde(default = "default_true")]
    pub unicode_whitespace: bool,
    /// 是否检查哈希注释。
    #[serde(default = "default_true")]
    pub hash_comment: bool,
    /// 是否检查注释与引号状态不同步。
    #[serde(default = "default_true")]
    pub comment_quote_desync: bool,
    /// 是否检查引号内换行符。
    #[serde(default)]
    pub quoted_newline: bool,
    /// 是否检查 zsh 危险结构。
    #[serde(default = "default_true")]
    pub zsh_dangerous: bool,
    /// 是否检查控制字符。
    #[serde(default = "default_true")]
    pub control_chars: bool,
}

impl Default for BashValidatorChecksConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            empty_command: true,
            heredoc: true,
            git_commit: true,
            jq: true,
            metacharacters: true,
            dangerous_vars: true,
            substitution: true,
            redirection: true,
            newline_injection: true,
            ifs_injection: true,
            proc_environ: true,
            malformed: true,
            obfuscated_flags: true,
            backslash: true,
            brace_expansion: true,
            unicode_whitespace: true,
            hash_comment: true,
            comment_quote_desync: true,
            quoted_newline: false,
            zsh_dangerous: true,
            control_chars: true,
        }
    }
}

/// Bash 命令执行前安全检查配置。
///
/// 关闭后跳过命令白名单、路径、风险、重定向与 Shell validator 检查。沙箱、超时、
/// 输出限制和系统调用异常检测仍由各自独立配置控制。
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct BashSecurityConfig {
    /// 是否启用 Bash 命令执行前安全检查。
    #[serde(default)]
    pub enabled: bool,
    /// 策略检查配置。
    #[serde(default)]
    pub policy: BashPolicyChecksConfig,
    /// Shell validator 检查配置。
    #[serde(default)]
    pub validators: BashValidatorChecksConfig,
}

fn default_true() -> bool {
    true
}

#[cfg(test)]
#[path = "bash_tests.rs"]
mod tests;
