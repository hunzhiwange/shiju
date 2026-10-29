#![cfg(target_os = "windows")]

//! Windows PowerShell 工具实现。
//!
//! 本模块只在 Windows 目标启用，负责把 agent 工具调用转换为受安全策略约束的
//! PowerShell 命令执行。复用 ShellTool 的权限、环境、进程会话和输出解码；
//! 仍保留 PowerShell 工具 ID、cwd 参数及只读模式下的拒绝行为。

use super::shell::ShellTool;
use super::traits::{Tool, ToolCallResult, ToolResult, ToolSpec};
use crate::app::agent::runtime::{RuntimeAdapter, native::NativeRuntime};
use crate::app::agent::security::SecurityPolicy;
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};
use std::sync::Arc;

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
    runtime: Arc<NativeRuntime>,
    shell: Arc<ShellTool>,
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
        let runtime = Arc::new(NativeRuntime::for_powershell());
        let shell = Arc::new(ShellTool::new(security.clone(), runtime.clone()));
        Self { security, runtime, shell }
    }

    fn shell_input(&self, input: Value) -> anyhow::Result<Value> {
        let args: Args = serde_json::from_value(input)?;
        let mut input = json!({
            "command": args.command,
            "description": args.description,
            "approved": args.approved,
        });
        if let Some(cwd) = args.cwd {
            input["workdir"] = json!(cwd);
        }
        Ok(input)
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Tool for PowerShellTool {
    fn name(&self) -> &str {
        "PowerShell"
    }

    fn description(&self) -> &str {
        "Execute a PowerShell command on Windows using managed process sessions. Continue running results with bash_wait, or stop them with bash_cancel."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "command": {
                    "type": "string",
                    "description": self.shell.parameters_schema()["properties"]["command"]["description"]
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
        let schema = self.parameters_schema();
        let instructions =
            schema["properties"]["command"]["description"].as_str().unwrap_or_default();
        ToolSpec::new(self.name(), format!("{}\n\n{instructions}", self.description()), schema)
            .with_display_name("PowerShell")
            .with_aliases(vec!["powershell".to_string()])
            .with_read_only(false)
            .with_destructive(true)
            .with_concurrency_safe(false)
            .with_requires_user_interaction(false)
            .with_strict(true)
    }

    async fn check_permissions(&self, input: &Value) -> anyhow::Result<()> {
        if !self.security.can_act() {
            anyhow::bail!("Action blocked: autonomy is read-only");
        }
        self.shell.check_permissions(&self.shell_input(input.clone())?).await
    }

    async fn call(&self, input: Value) -> anyhow::Result<ToolCallResult> {
        if !self.security.can_act() {
            return Ok(ToolCallResult::from_legacy_result(ToolResult {
                success: false,
                output: String::new(),
                error: Some("Action blocked: autonomy is read-only".to_string()),
            }));
        }
        if !self.runtime.has_shell_access() {
            anyhow::bail!("PowerShell is unavailable: install PowerShell or add it to PATH");
        }
        // Share authorization, filtered environment, owned process lifetime and
        // streaming decoding with shell calls; record the action only once.
        let mut result = self.shell.call(self.shell_input(input)?).await?;
        if let Some(hint) = &mut result.render_hint {
            hint.title = Some("PowerShell".into());
        }
        Ok(result)
    }

    async fn execute(&self, input: Value) -> anyhow::Result<ToolResult> {
        let result = self.call(input).await?;
        Ok(ToolResult {
            success: result.is_success(),
            output: result.data["output"].as_str().unwrap_or_default().to_string(),
            error: result.data["error"].as_str().map(str::to_owned),
        })
    }
}
#[cfg(test)]
mod tests;
