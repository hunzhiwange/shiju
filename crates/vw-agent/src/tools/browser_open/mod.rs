//! 浏览器打开工具
//!
//! 在浏览器中打开批准的 HTTPS URL（不执行抓取或 DOM 自动化）。
//! 仅用于简单的 URL 打开操作。

use super::traits::{
    Tool, ToolCallResult, ToolCallTelemetry, ToolRenderHint, ToolResult, ToolSpec,
};
use super::url_validation::{
    DomainPolicy, UrlSchemePolicy, normalize_allowed_domains, validate_url,
};
use crate::app::agent::security::SecurityPolicy;
use async_trait::async_trait;
use serde_json::{Value, json};
use std::sync::Arc;
use vw_api_types::tools::ToolResultContentDto;

/// 浏览器打开工具
///
/// 在系统默认浏览器或指定浏览器中打开已批准的 HTTPS URL。
/// 该工具仅用于简单的 URL 打开操作，不进行页面抓取或 DOM 自动化。
///
/// # 安全约束
///
/// - 仅允许打开白名单域名中的 HTTPS URL
/// - 禁止访问本地/私有主机地址
/// - 受安全策略的自主操作和速率限制约束
///
/// # 示例
///
/// ```ignore
/// use std::sync::Arc;
/// use crate::app::agent::security::SecurityPolicy;
/// use crate::app::agent::tools::browser_open::BrowserOpenTool;
///
/// let security = Arc::new(SecurityPolicy::default());
/// let allowed_domains = vec!["example.com".to_string()];
/// let tool = BrowserOpenTool::new(security, allowed_domains, "chrome".to_string());
/// ```
pub struct BrowserOpenTool {
    /// 安全策略引用，用于检查操作权限和速率限制
    security: Arc<SecurityPolicy>,
    /// 允许打开的域名白名单（已标准化处理）
    allowed_domains: Vec<String>,
    /// 浏览器名称（如 "chrome"、"firefox"、"brave"、"default"）
    browser_name: String,
}

impl BrowserOpenTool {
    /// 创建新的浏览器打开工具实例
    ///
    /// # 参数
    ///
    /// - `security`: 安全策略的共享引用，用于权限检查
    /// - `allowed_domains`: 允许打开的域名列表，将被标准化处理（转小写、去空格）
    /// - `browser_name`: 浏览器名称，支持 "default"、"chrome"、"firefox"、"brave"
    ///
    /// # 返回值
    ///
    /// 返回配置好的 `BrowserOpenTool` 实例
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let tool = BrowserOpenTool::new(
    ///     security,
    ///     vec!["example.com".to_string(), "docs.rs".to_string()],
    ///     "firefox".to_string()
    /// );
    /// ```
    pub fn new(
        security: Arc<SecurityPolicy>,
        allowed_domains: Vec<String>,
        browser_name: String,
    ) -> Self {
        Self { security, allowed_domains: normalize_allowed_domains(allowed_domains), browser_name }
    }

    /// 验证 URL 是否符合安全策略
    ///
    /// 检查 URL 是否满足以下条件：
    /// - 使用 HTTPS 协议
    /// - 域名在允许的白名单中
    /// - 不是本地或私有地址
    ///
    /// # 参数
    ///
    /// - `raw_url`: 待验证的原始 URL 字符串
    ///
    /// # 返回值
    ///
    /// - `Ok(String)`: 验证通过的 URL
    /// - `Err`: 验证失败的错误信息
    fn validate_url(&self, raw_url: &str) -> anyhow::Result<String> {
        validate_url(
            raw_url,
            &DomainPolicy {
                allowed_domains: &self.allowed_domains,
                blocked_domains: &[],
                allowed_field_name: "browser.allowed_domains",
                blocked_field_name: None,
                empty_allowed_message: "Browser tool is enabled but no allowed_domains are configured. Add browser.allowed_domains in system settings",
                scheme_policy: UrlSchemePolicy::HttpsOnly,
                ipv6_error_context: "browser_open",
            },
        )
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Tool for BrowserOpenTool {
    /// 返回工具名称
    ///
    /// # 返回值
    ///
    /// 固定返回 "browser_open"
    fn name(&self) -> &str {
        "browser_open"
    }

    /// 返回工具描述
    ///
    /// 描述工具的功能、用途和安全约束
    fn description(&self) -> &str {
        "Open an approved HTTPS URL in the system browser. This tool only opens URLs and cannot interact with pages (click, type, or extract text); use the `browser` tool for DOM operations. Security constraints: allowlisted domains only, no local/private hosts, and no page fetching."
    }

    /// 返回工具参数的 JSON Schema
    ///
    /// 定义工具接受的参数结构：
    /// - `url`: 必需的字符串参数，指定要打开的 HTTPS URL
    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "url": {
                    "type": "string",
                    "description": "HTTPS URL to open in the browser"
                }
            },
            "required": ["url"]
        })
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec::new(
            crate::app::agent::tools::BROWSER_OPEN_TOOL_ID,
            self.description(),
            self.parameters_schema(),
        )
        .with_display_name(crate::app::agent::tools::BROWSER_OPEN_TOOL_ID)
        .with_aliases(vec![crate::app::agent::tools::BROWSER_OPEN_TOOL_ALIAS.to_string()])
        .with_read_only(false)
        .with_destructive(false)
        .with_concurrency_safe(false)
        .with_requires_user_interaction(false)
        .with_strict(true)
    }

    async fn call(&self, input: Value) -> anyhow::Result<ToolCallResult> {
        let legacy = self.execute(input.clone()).await?;
        let browser = display_browser_name(&self.browser_name).to_string();

        if !legacy.success {
            let mut result = ToolCallResult::from_legacy_result(legacy);
            result.render_hint = Some(ToolRenderHint {
                title: Some(crate::app::agent::tools::BROWSER_OPEN_TOOL_ID.to_string()),
                kind: Some("browser_open".to_string()),
                summary: Some(format!("Failed to open URL in {browser}")),
                metadata: json!({ "browser": browser }),
            });
            return Ok(result);
        }

        let requested_url =
            input.get("url").and_then(Value::as_str).map(str::trim).unwrap_or_default();
        let url = if requested_url.is_empty() {
            String::new()
        } else {
            self.validate_url(requested_url).unwrap_or_else(|_| requested_url.to_string())
        };

        let data = json!({
            "url": url.clone(),
            "browser": browser.clone(),
        });

        Ok(ToolCallResult {
            data: data.clone(),
            model_result: Value::String(legacy.output),
            content_blocks: vec![ToolResultContentDto::Json { value: data.clone() }],
            render_hint: Some(ToolRenderHint {
                title: Some(crate::app::agent::tools::BROWSER_OPEN_TOOL_ID.to_string()),
                kind: Some("browser_open".to_string()),
                summary: Some(if url.is_empty() {
                    format!("Opened URL in {browser}")
                } else {
                    format!("Opened {url}")
                }),
                metadata: json!({
                    "browser": browser,
                    "url": url,
                }),
            }),
            telemetry: Some(ToolCallTelemetry { success: true, ..ToolCallTelemetry::default() }),
            ..ToolCallResult::default()
        })
    }

    /// 执行浏览器打开操作
    ///
    /// # 参数
    ///
    /// - `args`: JSON 格式的参数，包含 "url" 字段
    ///
    /// # 返回值
    ///
    /// 返回 `ToolResult`，包含：
    /// - `success`: 操作是否成功
    /// - `output`: 成功时的输出信息
    /// - `error`: 失败时的错误信息
    ///
    /// # 执行流程
    ///
    /// 1. 提取并验证 URL 参数
    /// 2. 检查安全策略是否允许操作
    /// 3. 记录操作并检查速率限制
    /// 4. 验证 URL 符合域名白名单和安全约束
    /// 5. 调用系统命令打开浏览器
    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        // 提取 URL 参数
        let url = args
            .get("url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'url' parameter"))?;

        // 检查自主操作权限（只读模式下禁止操作）
        if !self.security.can_act() {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some("Action blocked: autonomy is read-only".into()),
            });
        }

        // 记录操作并检查速率限制
        if !self.security.record_action() {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some("Action blocked: rate limit exceeded".into()),
            });
        }

        // 验证 URL 安全性
        let url = match self.validate_url(url) {
            Ok(v) => v,
            Err(e) => {
                return Ok(ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some(e.to_string()),
                });
            }
        };

        // 在浏览器中打开 URL
        match open_in_browser(&url, &self.browser_name).await {
            Ok(()) => Ok(ToolResult {
                success: true,
                output: format!("Opened in {}: {}", display_browser_name(&self.browser_name), url),
                error: None,
            }),
            Err(e) => Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!(
                    "Failed to open {}: {}",
                    display_browser_name(&self.browser_name),
                    e
                )),
            }),
        }
    }
}

/// 获取浏览器的显示名称（用户友好的字符串）
///
/// 将内部浏览器标识符转换为用户友好的显示名称。
///
/// # 参数
///
/// - `browser_name`: 内部浏览器标识符
///
/// # 返回值
///
/// 返回用户友好的浏览器显示名称：
/// - "brave" -> "Brave"
/// - "chrome" -> "Chrome"
/// - "firefox" -> "Firefox"
/// - "default" 或 "" -> "system default browser"
/// - 其他值 -> 原样返回
fn display_browser_name(browser_name: &str) -> &str {
    match browser_name {
        "brave" => "Brave",
        "chrome" => "Chrome",
        "firefox" => "Firefox",
        "default" | "" => "system default browser",
        _ => browser_name,
    }
}

/// 在指定浏览器中打开 URL
///
/// 根据浏览器名称调用相应的打开函数。会对浏览器名称进行标准化处理
///（转小写、替换连字符为下划线）。
///
/// # 参数
///
/// - `url`: 要打开的 URL
/// - `browser_name`: 浏览器名称（支持 "default"、"brave"、"chrome"、"firefox"）
///
/// # 返回值
///
/// - `Ok(())`: 成功打开
/// - `Err`: 打开失败或浏览器不支持
async fn open_in_browser(url: &str, browser_name: &str) -> anyhow::Result<()> {
    // 标准化浏览器名称：转小写并替换连字符
    let normalized = browser_name.to_ascii_lowercase().replace('-', "_");

    // 根据标准化后的名称选择对应的打开方式
    match normalized.as_str() {
        "default" | "new_window" | "new_tab" | "" => open_system_default(url).await,
        "brave" => open_brave(url).await,
        "chrome" => open_chrome(url).await,
        "firefox" => open_firefox(url).await,
        _ => anyhow::bail!(
            "Unsupported browser '{browser_name}'. Use: default, new_window, new_tab, brave, chrome, or firefox"
        ),
    }
}

/// 使用系统默认浏览器打开 URL
///
/// 根据操作系统使用不同的命令打开 URL：
/// - macOS: 使用 `open` 命令
/// - Linux: 尝试 `xdg-open`、`gio open`、`gnome-open`、`kde-open`
/// - Windows: 使用 `cmd /C start` 命令
///
/// # 参数
///
/// - `url`: 要打开的 URL
///
/// # 返回值
///
/// - `Ok(())`: 成功打开
/// - `Err`: 打开失败或不支持的操作系统
async fn open_system_default(url: &str) -> anyhow::Result<()> {
    // macOS: 使用 open 命令
    #[cfg(target_os = "macos")]
    {
        let status = tokio::process::Command::new("open").arg(url).status().await?;

        if status.success() {
            return Ok(());
        }
        anyhow::bail!("Failed to open URL with system default browser");
    }

    // Linux: 尝试多种打开命令，依次回退
    #[cfg(target_os = "linux")]
    {
        // 定义可能的打开命令，按优先级排序
        let commands = ["xdg-open", "gio open", "gnome-open", "kde-open"];
        let mut last_error = String::new();

        // 依次尝试每个命令
        for cmd in commands {
            match tokio::process::Command::new(cmd).arg(url).status().await {
                Ok(status) if status.success() => return Ok(()),
                Ok(status) => {
                    last_error = format!("{cmd} exited with status {status}");
                }
                Err(e) => {
                    last_error = format!("{cmd} not runnable: {e}");
                }
            }
        }
        anyhow::bail!("{last_error}");
    }

    // Windows: 使用 cmd start 命令
    #[cfg(target_os = "windows")]
    {
        let status =
            tokio::process::Command::new("cmd").args(["/C", "start", "", url]).status().await?;

        if status.success() {
            return Ok(());
        }
        anyhow::bail!("cmd start exited with non-zero status");
    }

    // 不支持的操作系统
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        let _ = url;
        anyhow::bail!("browser_open is not supported on this OS");
    }
}

/// 使用 Brave 浏览器打开 URL
///
/// 根据操作系统使用不同的方式打开 Brave 浏览器：
/// - macOS: 尝试 "Brave Browser" 和 "Brave" 应用名称
/// - Linux: 尝试 "brave-browser" 和 "brave" 命令
/// - Windows: 使用 `cmd /C start brave` 命令
///
/// # 参数
///
/// - `url`: 要打开的 URL
///
/// # 返回值
///
/// - `Ok(())`: 成功打开
/// - `Err`: 浏览器未找到或打开失败
async fn open_brave(url: &str) -> anyhow::Result<()> {
    // macOS: 尝试不同的应用名称
    #[cfg(target_os = "macos")]
    {
        for app in ["Brave Browser", "Brave"] {
            let status =
                tokio::process::Command::new("open").arg("-a").arg(app).arg(url).status().await;

            if let Ok(s) = status {
                if s.success() {
                    return Ok(());
                }
            }
        }
        anyhow::bail!(
            "Brave Browser was not found (tried macOS app names 'Brave Browser' and 'Brave')"
        );
    }

    // Linux: 尝试不同的命令名称
    #[cfg(target_os = "linux")]
    {
        let mut last_error = String::new();
        for cmd in ["brave-browser", "brave"] {
            match tokio::process::Command::new(cmd).arg(url).status().await {
                Ok(status) if status.success() => return Ok(()),
                Ok(status) => {
                    last_error = format!("{cmd} exited with status {status}");
                }
                Err(e) => {
                    last_error = format!("{cmd} not runnable: {e}");
                }
            }
        }
        anyhow::bail!("{last_error}");
    }

    // Windows: 使用 cmd start 命令
    #[cfg(target_os = "windows")]
    {
        let status = tokio::process::Command::new("cmd")
            .args(["/C", "start", "", "brave", url])
            .status()
            .await?;

        if status.success() {
            return Ok(());
        }

        anyhow::bail!("cmd start brave exited with status {status}");
    }

    // 不支持的操作系统
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        let _ = url;
        anyhow::bail!("browser_open is not supported on this OS");
    }
}

/// 使用 Chrome 浏览器打开 URL
///
/// 根据操作系统使用不同的方式打开 Chrome 浏览器：
/// - macOS: 尝试 "Google Chrome" 和 "Chrome" 应用名称
/// - Linux: 尝试 "google-chrome"、"chrome"、"chromium"、"chromium-browser" 命令
/// - Windows: 使用 `cmd /C start chrome` 命令
///
/// # 参数
///
/// - `url`: 要打开的 URL
///
/// # 返回值
///
/// - `Ok(())`: 成功打开
/// - `Err`: 浏览器未找到或打开失败
async fn open_chrome(url: &str) -> anyhow::Result<()> {
    // macOS: 尝试不同的应用名称
    #[cfg(target_os = "macos")]
    {
        for app in ["Google Chrome", "Chrome"] {
            let status =
                tokio::process::Command::new("open").arg("-a").arg(app).arg(url).status().await;

            if let Ok(s) = status {
                if s.success() {
                    return Ok(());
                }
            }
        }
        anyhow::bail!(
            "Google Chrome was not found (tried macOS app names 'Google Chrome' and 'Chrome')"
        );
    }

    // Linux: 尝试不同的命令名称（包括 Chromium 变体）
    #[cfg(target_os = "linux")]
    {
        let mut last_error = String::new();
        for cmd in ["google-chrome", "chrome", "chromium", "chromium-browser"] {
            match tokio::process::Command::new(cmd).arg(url).status().await {
                Ok(status) if status.success() => return Ok(()),
                Ok(status) => {
                    last_error = format!("{cmd} exited with status {status}");
                }
                Err(e) => {
                    last_error = format!("{cmd} not runnable: {e}");
                }
            }
        }
        anyhow::bail!("{last_error}");
    }

    // Windows: 使用 cmd start 命令
    #[cfg(target_os = "windows")]
    {
        let status = tokio::process::Command::new("cmd")
            .args(["/C", "start", "", "chrome", url])
            .status()
            .await?;

        if status.success() {
            return Ok(());
        }

        anyhow::bail!("cmd start chrome exited with status {status}");
    }

    // 不支持的操作系统
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        let _ = url;
        anyhow::bail!("browser_open is not supported on this OS");
    }
}

/// 使用 Firefox 浏览器打开 URL
///
/// 根据操作系统使用不同的方式打开 Firefox 浏览器：
/// - macOS: 使用 "Firefox" 应用名称
/// - Linux: 尝试 "firefox" 和 "firefox-esr"（Extended Support Release）命令
/// - Windows: 使用 `cmd /C start firefox` 命令
///
/// # 参数
///
/// - `url`: 要打开的 URL
///
/// # 返回值
///
/// - `Ok(())`: 成功打开
/// - `Err`: 浏览器未找到或打开失败
async fn open_firefox(url: &str) -> anyhow::Result<()> {
    // macOS: 使用 Firefox 应用
    #[cfg(target_os = "macos")]
    {
        let status =
            tokio::process::Command::new("open").arg("-a").arg("Firefox").arg(url).status().await?;

        if status.success() {
            return Ok(());
        }

        anyhow::bail!("Firefox was not found");
    }

    // Linux: 尝试标准版和 ESR 版
    #[cfg(target_os = "linux")]
    {
        let mut last_error = String::new();
        for cmd in ["firefox", "firefox-esr"] {
            match tokio::process::Command::new(cmd).arg(url).status().await {
                Ok(status) if status.success() => return Ok(()),
                Ok(status) => {
                    last_error = format!("{cmd} exited with status {status}");
                }
                Err(e) => {
                    last_error = format!("{cmd} not runnable: {e}");
                }
            }
        }
        anyhow::bail!("{last_error}");
    }

    // Windows: 使用 cmd start 命令
    #[cfg(target_os = "windows")]
    {
        let status = tokio::process::Command::new("cmd")
            .args(["/C", "start", "", "firefox", url])
            .status()
            .await?;

        if status.success() {
            return Ok(());
        }

        anyhow::bail!("cmd start firefox exited with status {status}");
    }

    // 不支持的操作系统
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        let _ = url;
        anyhow::bail!("browser_open is not supported on this OS");
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
