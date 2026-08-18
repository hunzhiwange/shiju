#![cfg(target_os = "windows")]

//! Windows PowerShell 工具实现。
//!
//! 本模块只在 Windows 目标启用，负责把 agent 工具调用转换为受安全策略约束的
//! PowerShell 命令执行。执行前会检查自治权限、速率限制和命令风险，避免工具绕过
//! 统一安全边界。

use super::traits::{Tool, ToolResult, ToolSpec};
use crate::app::agent::security::SecurityPolicy;
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::process::Command;

#[derive(Debug, Clone, Deserialize)]
struct Args {
    command: String,
    description: String,
    #[serde(default)]
    cwd: Option<String>,
    #[serde(default)]
    approved: bool,
}

#[derive(Clone)]
/// 在 Windows 上执行 PowerShell 命令的工具。
///
/// 工具持有共享安全策略，用于限制命令执行权限、工作目录解析和高风险命令审批。
pub struct PowerShellTool {
    security: Arc<SecurityPolicy>,
}

impl PowerShellTool {
    /// 创建新的 PowerShell 工具。
    ///
    /// # 参数
    ///
    /// - `security`: 当前 agent 会话的安全策略。
    ///
    /// # 返回值
    ///
    /// 返回绑定该安全策略的工具实例。
    pub fn new(security: Arc<SecurityPolicy>) -> Self {
        Self { security }
    }

    fn resolve_cwd(&self, raw: Option<&str>) -> anyhow::Result<PathBuf> {
        let Some(raw) = raw.map(str::trim).filter(|value| !value.is_empty()) else {
            return Ok(self.security.workspace_dir.clone());
        };
        let path = Path::new(raw);
        let resolved = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.security.workspace_dir.join(path)
        };
        Ok(resolved)
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Tool for PowerShellTool {
    fn name(&self) -> &str {
        "PowerShell"
    }

    fn description(&self) -> &str {
        "Execute a PowerShell command on Windows."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "command": {
                    "type": "string",
                    "description": "PowerShell command to execute."
                },
                "description": {
                    "type": "string",
                    "description": "Explanation of the command's purpose."
                },
                "cwd": {
                    "type": "string",
                    "description": "Optional working directory."
                },
                "approved": {
                    "type": "boolean",
                    "default": false,
                    "description": "Explicitly approve medium/high-risk commands in supervised mode."
                }
            },
            "required": ["command", "description"]
        })
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec::new(self.name(), self.description(), self.parameters_schema())
            .with_display_name("PowerShell")
            .with_aliases(vec!["powershell".to_string()])
            .with_read_only(false)
            .with_destructive(true)
            .with_concurrency_safe(false)
            .with_requires_user_interaction(false)
            .with_strict(true)
    }

    async fn execute(&self, args: Value) -> anyhow::Result<ToolResult> {
        let args: Args = serde_json::from_value(args)?;
        if !self.security.can_act() {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some("Action blocked: autonomy is read-only".to_string()),
            });
        }
        if !self.security.record_action() {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some("Action blocked: rate limit exceeded".to_string()),
            });
        }
        // PowerShell 可以修改系统状态，必须在启动进程前复用统一命令校验路径。
        self.security
            .validate_command_execution(&args.command, args.approved)
            .map_err(anyhow::Error::msg)?;

        let cwd = self.resolve_cwd(args.cwd.as_deref())?;
        let output = Command::new("powershell")
            .arg("-NoLogo")
            .arg("-NoProfile")
            .arg("-NonInteractive")
            .arg("-Command")
            .arg(&args.command)
            .current_dir(cwd)
            .output()
            .await?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Ok(ToolResult {
            success: output.status.success(),
            output: stdout,
            error: (!stderr.is_empty()).then_some(stderr),
        })
    }
}
#[cfg(test)]
mod tests;
