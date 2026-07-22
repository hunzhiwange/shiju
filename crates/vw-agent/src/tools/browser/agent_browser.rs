//! 浏览器代理命令执行模块
//!
//! 本模块提供了通过外部 `agent-browser` CLI 工具执行浏览器操作的封装。
//! 它是浏览器工具的底层实现之一，通过命令行接口与外部浏览器代理程序通信。
//!
//! # 主要功能
//!
//! - **命令可用性检测**：检查 `agent-browser` CLI 是否已安装并可用
//! - **命令执行封装**：提供类型安全的命令执行接口，自动处理 JSON 响应解析
//! - **浏览器操作映射**：将内部的 `BrowserAction` 枚举转换为 CLI 命令参数
//!
//! # 架构说明
//!
//! ```text
//! BrowserAction (内部枚举)
//!      ↓
//! execute_agent_browser_action() (映射转换)
//!      ↓
//! run_command() (CLI 命令构建)
//!      ↓
//! agent-browser CLI (外部工具)
//!      ↓
//! AgentBrowserResponse (JSON 响应解析)
//!      ↓
//! ToolResult (统一返回格式)
//! ```

use super::BrowserTool;
use crate::app::agent::shell::tokio_command;
use crate::app::agent::tools::browser::actions::BrowserAction;
use crate::app::agent::tools::traits::ToolResult;
use serde::Deserialize;
use serde_json::{Value, json};
use std::process::Stdio;
use tracing::debug;

/// agent-browser CLI 工具的 JSON 响应结构
///
/// 此结构体用于反序列化 `agent-browser --json` 命令的输出。
/// 所有通过 CLI 执行的命令都会返回此格式的 JSON 响应。
///
/// # 字段说明
///
/// * `success` - 命令执行是否成功
/// * `data` - 成功时返回的数据（JSON 格式），失败时可能为 `None`
/// * `error` - 失败时的错误信息，成功时为 `None`
///
/// # 示例响应
///
/// 成功响应：
/// ```json
/// {
///     "success": true,
///     "data": {"url": "https://example.com", "title": "Example"},
///     "error": null
/// }
/// ```
///
/// 失败响应：
/// ```json
/// {
///     "success": false,
///     "data": null,
///     "error": "Element not found: #nonexistent"
/// }
/// ```
#[derive(Debug, Deserialize)]
pub(crate) struct AgentBrowserResponse {
    /// 命令执行是否成功
    pub(crate) success: bool,
    /// 返回的数据内容，JSON 格式，可能为空
    pub(crate) data: Option<Value>,
    /// 错误信息，仅在失败时有值
    pub(crate) error: Option<String>,
}

fn response_from_output(status_success: bool, stdout: &str, stderr: &str) -> AgentBrowserResponse {
    if let Ok(response) = serde_json::from_str::<AgentBrowserResponse>(stdout) {
        return response;
    }

    if status_success {
        AgentBrowserResponse {
            success: true,
            data: Some(json!({ "output": stdout.trim() })),
            error: None,
        }
    } else {
        AgentBrowserResponse { success: false, data: None, error: Some(stderr.trim().to_string()) }
    }
}

impl BrowserTool {
    /// 检查 `agent-browser` CLI 工具是否已安装并可用
    ///
    /// 通过执行 `agent-browser --version` 命令来验证工具是否在系统 PATH 中可用。
    /// 此方法不会检查具体版本，仅确认工具可执行。
    ///
    /// # 返回值
    ///
    /// - `true` - 工具已安装且可正常执行
    /// - `false` - 工具未安装、不在 PATH 中或执行失败
    ///
    /// # 示例
    ///
    /// ```ignore
    /// if BrowserTool::is_agent_browser_available().await {
    ///     println!("agent-browser 已安装");
    /// } else {
    ///     println!("请先安装 agent-browser 工具");
    /// }
    /// ```
    ///
    /// # 实现细节
    ///
    /// - 使用 `Stdio::null()` 丢弃版本输出
    /// - 任何执行错误（如命令未找到）都会返回 `false`
    pub async fn is_agent_browser_available() -> bool {
        tokio_command("agent-browser")
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await
            .map(|s| s.success())
            .unwrap_or(false)
    }

    /// 执行 agent-browser CLI 命令并返回结构化响应
    ///
    /// 此方法是所有 agent-browser 命令执行的统一入口，负责：
    /// 1. 构建完整的命令行（包括会话配置和 JSON 输出标志）
    /// 2. 执行命令并捕获标准输出和标准错误
    /// 3. 解析 JSON 响应，对非 JSON 输出进行兼容处理
    ///
    /// # 参数
    ///
    /// * `args` - 命令行参数数组（不包括 `agent-browser` 本身和 `--json` 标志）
    ///
    /// # 返回值
    ///
    /// 成功返回 `Ok(AgentBrowserResponse)`，包含解析后的响应数据。
    /// 失败返回 `Err(anyhow::Error)`，通常表示命令执行层面的错误。
    ///
    /// # 示例
    ///
    /// ```ignore
    /// // 执行打开 URL 的命令
    /// let response = self.run_command(&["open", "https://example.com"]).await?;
    /// if response.success {
    ///     println!("页面已打开: {:?}", response.data);
    /// } else {
    ///     eprintln!("打开失败: {:?}", response.error);
    /// }
    /// ```
    ///
    /// # 会话支持
    ///
    /// 如果 `self.session_name` 已配置，会自动添加 `--session` 参数，
    /// 使得多个命令可以在同一个浏览器会话中执行。
    ///
    /// # 错误处理
    ///
    /// - 优先尝试解析 JSON 响应
    /// - 对于非 JSON 输出的旧版本 CLI，会根据命令退出码构造兼容响应
    /// - 标准错误输出会通过 debug 日志记录
    pub(crate) async fn run_command(&self, args: &[&str]) -> anyhow::Result<AgentBrowserResponse> {
        let mut cmd = tokio_command("agent-browser");

        // 如果配置了会话名称，添加 --session 参数以支持会话持久化
        if let Some(ref session) = self.session_name {
            cmd.arg("--session").arg(session);
        }

        // 添加 --json 标志以获取机器可读的 JSON 输出
        cmd.args(args).arg("--json");

        debug!("Running: agent-browser {} --json", args.join(" "));

        // 执行命令并捕获输出
        let output = cmd.stdout(Stdio::piped()).stderr(Stdio::piped()).output().await?;

        // 将字节数据转换为字符串以便处理
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        // 记录标准错误输出（用于调试）
        if !stderr.is_empty() {
            debug!("agent-browser stderr: {}", stderr);
        }

        Ok(response_from_output(output.status.success(), &stdout, &stderr))
    }

    /// 执行浏览器操作并将其转换为工具结果
    ///
    /// 此方法将内部的 `BrowserAction` 枚举映射为对应的 `agent-browser` CLI 命令，
    /// 执行命令并将响应转换为统一的 `ToolResult` 格式。
    ///
    /// # 参数
    ///
    /// * `action` - 要执行的浏览器操作，参见 [`BrowserAction`] 枚举
    ///
    /// # 返回值
    ///
    /// 返回 `anyhow::Result<ToolResult>`，其中：
    /// - `Ok(ToolResult)` 包含操作结果数据
    /// - `Err` 表示命令执行或参数验证失败
    ///
    /// # 支持的操作
    ///
    /// | 操作 | CLI 命令 | 说明 |
    /// |------|----------|------|
    /// | `Open` | `open <url>` | 打开指定 URL |
    /// | `Snapshot` | `snapshot [-i] [-c] [-d N]` | 获取页面快照 |
    /// | `Click` | `click <selector>` | 点击元素 |
    /// | `Fill` | `fill <selector> <value>` | 填充表单 |
    /// | `Type` | `type <selector> <text>` | 模拟键盘输入 |
    /// | `GetText` | `get text <selector>` | 获取元素文本 |
    /// | `GetTitle` | `get title` | 获取页面标题 |
    /// | `GetUrl` | `get url` | 获取当前 URL |
    /// | `Screenshot` | `screenshot [path] [--full]` | 截取屏幕 |
    /// | `Wait` | `wait [selector\|ms\|--text]` | 等待条件 |
    /// | `Press` | `press <key>` | 按键 |
    /// | `Hover` | `hover <selector>` | 鼠标悬停 |
    /// | `Scroll` | `scroll <direction> [pixels]` | 滚动页面 |
    /// | `IsVisible` | `is visible <selector>` | 检查可见性 |
    /// | `Close` | `close` | 关闭浏览器 |
    /// | `Find` | `find <by> <value> <action> [fill_value]` | 查找并执行 |
    ///
    /// # 安全性
    ///
    /// 对于 `Open` 操作，会先调用 `validate_url()` 进行 URL 验证，
    /// 防止打开不安全或未经授权的 URL。
    #[allow(clippy::too_many_lines)]
    pub(crate) async fn execute_agent_browser_action(
        &self,
        action: BrowserAction,
    ) -> anyhow::Result<ToolResult> {
        match action {
            // 打开指定 URL，会先进行 URL 安全验证
            BrowserAction::Open { url } => {
                self.validate_url(&url)?;
                let resp = self.run_command(&["open", &url]).await?;
                self.to_result(resp)
            }

            // 获取页面 DOM 快照，支持多种过滤和格式化选项
            BrowserAction::Snapshot { interactive_only, compact, depth } => {
                let mut args = vec!["snapshot"];
                // 仅返回可交互元素（如按钮、输入框等）
                if interactive_only {
                    args.push("-i");
                }
                // 返回紧凑格式，减少冗余信息
                if compact {
                    args.push("-c");
                }
                // 设置 DOM 遍历深度限制
                let depth_str;
                if let Some(d) = depth {
                    args.push("-d");
                    depth_str = d.to_string();
                    args.push(&depth_str);
                }
                let resp = self.run_command(&args).await?;
                self.to_result(resp)
            }

            // 点击指定选择器的元素
            BrowserAction::Click { selector } => {
                let resp = self.run_command(&["click", &selector]).await?;
                self.to_result(resp)
            }

            // 在指定元素中填充值（适用于 input、textarea 等）
            BrowserAction::Fill { selector, value } => {
                let resp = self.run_command(&["fill", &selector, &value]).await?;
                self.to_result(resp)
            }

            // 模拟键盘逐字符输入文本
            BrowserAction::Type { selector, text } => {
                let resp = self.run_command(&["type", &selector, &text]).await?;
                self.to_result(resp)
            }

            // 获取指定元素的文本内容
            BrowserAction::GetText { selector } => {
                let resp = self.run_command(&["get", "text", &selector]).await?;
                self.to_result(resp)
            }

            // 获取当前页面的标题
            BrowserAction::GetTitle => {
                let resp = self.run_command(&["get", "title"]).await?;
                self.to_result(resp)
            }

            // 获取当前页面的 URL
            BrowserAction::GetUrl => {
                let resp = self.run_command(&["get", "url"]).await?;
                self.to_result(resp)
            }

            // 截取当前页面或整个页面的屏幕截图
            BrowserAction::Screenshot { path, full_page } => {
                let mut args = vec!["screenshot"];
                // 可选的保存路径
                if let Some(ref p) = path {
                    args.push(p);
                }
                // 是否截取整个页面（包括滚动区域）
                if full_page {
                    args.push("--full");
                }
                let resp = self.run_command(&args).await?;
                self.to_result(resp)
            }

            // 等待指定条件：元素出现、时间流逝或文本出现
            BrowserAction::Wait { selector, ms, text } => {
                let mut args = vec!["wait"];
                let ms_str;
                // 三种等待模式，按优先级选择：选择器 > 毫秒数 > 文本
                if let Some(sel) = selector.as_ref() {
                    args.push(sel);
                } else if let Some(millis) = ms {
                    ms_str = millis.to_string();
                    args.push(&ms_str);
                } else if let Some(ref t) = text {
                    args.push("--text");
                    args.push(t);
                }
                let resp = self.run_command(&args).await?;
                self.to_result(resp)
            }

            // 模拟按下键盘按键（如 Enter、Tab、Escape 等）
            BrowserAction::Press { key } => {
                let resp = self.run_command(&["press", &key]).await?;
                self.to_result(resp)
            }

            // 将鼠标悬停在指定元素上
            BrowserAction::Hover { selector } => {
                let resp = self.run_command(&["hover", &selector]).await?;
                self.to_result(resp)
            }

            // 按指定方向和像素数滚动页面
            BrowserAction::Scroll { direction, pixels } => {
                let mut args = vec!["scroll", &direction];
                let px_str;
                // 可选的滚动像素数
                if let Some(px) = pixels {
                    px_str = px.to_string();
                    args.push(&px_str);
                }
                let resp = self.run_command(&args).await?;
                self.to_result(resp)
            }

            // 检查指定元素是否在视口中可见
            BrowserAction::IsVisible { selector } => {
                let resp = self.run_command(&["is", "visible", &selector]).await?;
                self.to_result(resp)
            }

            // 关闭当前浏览器会话
            BrowserAction::Close => {
                let resp = self.run_command(&["close"]).await?;
                self.to_result(resp)
            }

            // 综合查找操作：按指定方式查找元素并执行动作
            BrowserAction::Find { by, value, action, fill_value } => {
                let mut args = vec!["find", &by, &value, &action];
                // 如果动作是填充操作，提供填充值
                if let Some(ref fv) = fill_value {
                    args.push(fv);
                }
                let resp = self.run_command(&args).await?;
                self.to_result(resp)
            }
        }
    }
}
#[cfg(test)]
#[path = "agent_browser_tests.rs"]
mod agent_browser_tests;
