use super::BrowserTool;
use super::actions::{
    BrowserAction, is_computer_use_only_action, is_supported_browser_action, parse_browser_action,
};
use super::agent_browser::AgentBrowserResponse;
use super::backend::{BrowserBackendKind, ResolvedBackend, unavailable_action_for_backend_error};
use super::computer_use::ComputerUseClient;
#[cfg(feature = "browser-native")]
use super::helpers::is_recoverable_rust_native_error;
use super::helpers::normalize_domains;
use crate::app::agent::security::SecurityPolicy;
use crate::app::agent::tools::traits::{
    Tool, ToolCallResult, ToolCallTelemetry, ToolRenderHint, ToolResult, ToolSpec,
};
#[cfg(feature = "browser-native")]
use anyhow::Context;
use async_trait::async_trait;
use serde_json::{Value, json};
use std::sync::Arc;
use vw_api_types::tools::ToolResultContentDto;

impl BrowserTool {
    pub fn new(
        security: Arc<SecurityPolicy>,
        allowed_domains: Vec<String>,
        session_name: Option<String>,
    ) -> Self {
        Self::new_with_backend(
            security,
            allowed_domains,
            session_name,
            "agent_browser".into(),
            true,
            "http://127.0.0.1:9515".into(),
            None,
            super::ComputerUseConfig::default(),
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new_with_backend(
        security: Arc<SecurityPolicy>,
        allowed_domains: Vec<String>,
        session_name: Option<String>,
        backend: String,
        native_headless: bool,
        native_webdriver_url: String,
        native_chrome_path: Option<String>,
        computer_use: super::ComputerUseConfig,
    ) -> Self {
        Self {
            security,
            allowed_domains: normalize_domains(allowed_domains),
            session_name,
            backend,
            native_headless,
            native_webdriver_url,
            native_chrome_path,
            computer_use,
            #[cfg(feature = "browser-native")]
            native_state: tokio::sync::Mutex::new(
                super::native_backend::NativeBrowserState::default(),
            ),
        }
    }

    /// Backward-compatible alias.
    pub async fn is_available() -> bool {
        Self::is_agent_browser_available().await
    }

    pub(crate) fn configured_backend(&self) -> anyhow::Result<BrowserBackendKind> {
        BrowserBackendKind::parse(&self.backend)
    }

    fn rust_native_compiled() -> bool {
        cfg!(feature = "browser-native")
    }

    fn rust_native_available(&self) -> bool {
        #[cfg(feature = "browser-native")]
        {
            super::native_backend::NativeBrowserState::is_available(
                self.native_headless,
                &self.native_webdriver_url,
                self.native_chrome_path.as_deref(),
            )
        }
        #[cfg(not(feature = "browser-native"))]
        {
            false
        }
    }

    fn computer_use_client(&self) -> ComputerUseClient {
        ComputerUseClient::new(
            self.security.clone(),
            self.allowed_domains.clone(),
            self.session_name.clone(),
            self.computer_use.clone(),
        )
    }

    pub(crate) fn computer_use_endpoint_url(&self) -> anyhow::Result<reqwest::Url> {
        self.computer_use_client().endpoint_url()
    }

    fn computer_use_available(&self) -> anyhow::Result<bool> {
        self.computer_use_client().available()
    }

    async fn resolve_backend(&self) -> anyhow::Result<ResolvedBackend> {
        let configured = self.configured_backend()?;

        match configured {
            BrowserBackendKind::AgentBrowser => {
                if Self::is_agent_browser_available().await {
                    Ok(ResolvedBackend::AgentBrowser)
                } else {
                    anyhow::bail!(
                        "browser.backend='{}' but agent-browser CLI is unavailable. Install with: npm install -g agent-browser",
                        configured.as_str()
                    )
                }
            }
            BrowserBackendKind::RustNative => {
                if !Self::rust_native_compiled() {
                    anyhow::bail!(
                        "browser.backend='rust_native' requires build feature 'browser-native'"
                    );
                }
                if !self.rust_native_available() {
                    anyhow::bail!(
                        "Rust-native browser backend is enabled but WebDriver endpoint is unreachable. Set browser.native_webdriver_url and start a compatible driver"
                    );
                }
                Ok(ResolvedBackend::RustNative)
            }
            BrowserBackendKind::ComputerUse => {
                if !self.computer_use_available()? {
                    anyhow::bail!(
                        "browser.backend='computer_use' but sidecar endpoint is unreachable. Check browser.computer_use.endpoint and sidecar status"
                    );
                }
                Ok(ResolvedBackend::ComputerUse)
            }
            BrowserBackendKind::Auto => {
                if Self::rust_native_compiled() && self.rust_native_available() {
                    return Ok(ResolvedBackend::RustNative);
                }
                if Self::is_agent_browser_available().await {
                    return Ok(ResolvedBackend::AgentBrowser);
                }

                let computer_use_err = match self.computer_use_available() {
                    Ok(true) => return Ok(ResolvedBackend::ComputerUse),
                    Ok(false) => None,
                    Err(err) => Some(err.to_string()),
                };

                if Self::rust_native_compiled() {
                    if let Some(err) = computer_use_err {
                        anyhow::bail!(
                            "browser.backend='auto' found no usable backend (agent-browser missing, rust-native unavailable, computer-use invalid: {err})"
                        );
                    }
                    anyhow::bail!(
                        "browser.backend='auto' found no usable backend (agent-browser missing, rust-native unavailable, computer-use sidecar unreachable)"
                    )
                }

                if let Some(err) = computer_use_err {
                    anyhow::bail!(
                        "browser.backend='auto' needs agent-browser CLI, browser-native, or valid computer-use sidecar (error: {err})"
                    );
                }

                anyhow::bail!(
                    "browser.backend='auto' needs agent-browser CLI, browser-native, or computer-use sidecar"
                )
            }
        }
    }

    /// Validate URL against allowlist
    pub(crate) fn validate_url(&self, url: &str) -> anyhow::Result<()> {
        self.computer_use_client().validate_url(url)
    }

    async fn execute_computer_use_action(
        &self,
        action: &str,
        args: &Value,
    ) -> anyhow::Result<ToolResult> {
        self.computer_use_client().execute_action(action, args).await
    }

    #[allow(clippy::unused_async)]
    async fn execute_rust_native_action(
        &self,
        action: BrowserAction,
    ) -> anyhow::Result<ToolResult> {
        #[cfg(feature = "browser-native")]
        {
            let mut state = self.native_state.lock().await;

            let first_attempt = state
                .execute_action(
                    action.clone(),
                    self.native_headless,
                    &self.native_webdriver_url,
                    self.native_chrome_path.as_deref(),
                )
                .await;

            let output = match first_attempt {
                Ok(output) => output,
                Err(err) => {
                    if !is_recoverable_rust_native_error(&err) {
                        return Err(err);
                    }

                    state.reset_session().await;
                    state
                        .execute_action(
                            action,
                            self.native_headless,
                            &self.native_webdriver_url,
                            self.native_chrome_path.as_deref(),
                        )
                        .await
                        .with_context(|| "rust_native backend retry after session reset failed")?
                }
            };

            Ok(ToolResult {
                success: true,
                output: serde_json::to_string_pretty(&output).unwrap_or_default(),
                error: None,
            })
        }

        #[cfg(not(feature = "browser-native"))]
        {
            let _ = action;
            anyhow::bail!(
                "Rust-native browser backend is not compiled. Rebuild with --features browser-native"
            )
        }
    }

    async fn execute_action(
        &self,
        action: BrowserAction,
        backend: ResolvedBackend,
    ) -> anyhow::Result<ToolResult> {
        match backend {
            ResolvedBackend::AgentBrowser => self.execute_agent_browser_action(action).await,
            ResolvedBackend::RustNative => self.execute_rust_native_action(action).await,
            ResolvedBackend::ComputerUse => anyhow::bail!(
                "Internal error: computer_use backend must be handled before BrowserAction parsing"
            ),
        }
    }

    #[allow(clippy::unnecessary_wraps, clippy::unused_self)]
    pub(crate) fn to_result(&self, resp: AgentBrowserResponse) -> anyhow::Result<ToolResult> {
        if resp.success {
            let output = resp
                .data
                .map(|d| serde_json::to_string_pretty(&d).unwrap_or_default())
                .unwrap_or_default();
            Ok(ToolResult { success: true, output, error: None })
        } else {
            Ok(ToolResult { success: false, output: String::new(), error: resp.error })
        }
    }

    fn sanitize_payload(value: &Value) -> Value {
        match value {
            Value::Array(items) => Value::Array(items.iter().map(Self::sanitize_payload).collect()),
            Value::Object(map) => {
                let mut sanitized = serde_json::Map::new();
                for (key, item) in map {
                    let key_lower = key.to_ascii_lowercase();
                    if key_lower.contains("base64") {
                        if let Some(text) = item.as_str() {
                            sanitized.insert(
                                key.clone(),
                                Value::String(format!("<base64 {} chars>", text.chars().count())),
                            );
                            continue;
                        }
                    }
                    sanitized.insert(key.clone(), Self::sanitize_payload(item));
                }
                Value::Object(sanitized)
            }
            _ => value.clone(),
        }
    }

    fn summarize_action(action: &str, payload: &Value) -> String {
        match action {
            "open" => payload
                .get("url")
                .and_then(Value::as_str)
                .map(|url| format!("Opened {url}"))
                .unwrap_or_else(|| "Opened page".to_string()),
            "snapshot" => "Captured page snapshot".to_string(),
            "click" => payload
                .get("selector")
                .and_then(Value::as_str)
                .map(|selector| format!("Clicked {selector}"))
                .unwrap_or_else(|| "Clicked element".to_string()),
            "fill" => payload
                .get("selector")
                .and_then(Value::as_str)
                .map(|selector| format!("Filled {selector}"))
                .unwrap_or_else(|| "Filled field".to_string()),
            "type" => payload
                .get("selector")
                .and_then(Value::as_str)
                .map(|selector| format!("Typed into {selector}"))
                .unwrap_or_else(|| "Typed text".to_string()),
            "get_text" => payload
                .get("selector")
                .and_then(Value::as_str)
                .map(|selector| format!("Read text from {selector}"))
                .unwrap_or_else(|| "Read element text".to_string()),
            "get_title" => payload
                .get("title")
                .and_then(Value::as_str)
                .map(|title| format!("Read title: {title}"))
                .unwrap_or_else(|| "Read page title".to_string()),
            "get_url" => payload
                .get("url")
                .and_then(Value::as_str)
                .map(|url| format!("Read URL: {url}"))
                .unwrap_or_else(|| "Read current URL".to_string()),
            "screenshot" | "screen_capture" => payload
                .get("path")
                .and_then(Value::as_str)
                .map(|path| format!("Captured screenshot to {path}"))
                .unwrap_or_else(|| "Captured screenshot".to_string()),
            "wait" => "Wait completed".to_string(),
            "press" | "key_press" => payload
                .get("key")
                .and_then(Value::as_str)
                .map(|key| format!("Pressed {key}"))
                .unwrap_or_else(|| "Pressed key".to_string()),
            "hover" => payload
                .get("selector")
                .and_then(Value::as_str)
                .map(|selector| format!("Hovered {selector}"))
                .unwrap_or_else(|| "Hovered element".to_string()),
            "scroll" => payload
                .get("direction")
                .and_then(Value::as_str)
                .map(|direction| format!("Scrolled {direction}"))
                .unwrap_or_else(|| "Scrolled page".to_string()),
            "is_visible" => payload
                .get("selector")
                .and_then(Value::as_str)
                .map(|selector| format!("Checked visibility for {selector}"))
                .unwrap_or_else(|| "Checked element visibility".to_string()),
            "close" => "Closed browser".to_string(),
            "find" => payload
                .get("value")
                .and_then(Value::as_str)
                .map(|value| format!("Resolved element for {value}"))
                .unwrap_or_else(|| "Resolved page element".to_string()),
            _ => payload
                .get("backend")
                .and_then(Value::as_str)
                .map(|backend| format!("Ran {action} via {backend}"))
                .unwrap_or_else(|| format!("Ran {action}")),
        }
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Tool for BrowserTool {
    fn name(&self) -> &str {
        "browser"
    }

    fn description(&self) -> &str {
        concat!(
            "Web/浏览器自动化，支持可插拔后端（agent-browser、rust-native、computer_use）。",
            "支持 DOM 操作以及通过 computer-use 旁路实现的可选 OS 级操作（mouse_move、mouse_click、mouse_drag、",
            "key_type、key_press、screen_capture）。使用 'snapshot' 将可交互元素映射到引用（@e1、@e2）。",
            "对 open 操作强制执行 browser.allowed_domains。\n",
            "【重要防错指南】:\n",
            "1. 执行 'find' 操作时，必须提供 'value' 和 'find_action' 参数。\n",
            "2. 执行 click/fill/type/get_text/hover/is_visible 时，必须提供非空字符串 'selector'（优先用 snapshot 返回的 @e1、@e2；也可 CSS 或 text=...）。",
            "get_text 不会读取整页，缺少 selector 会直接失败。\n",
            "3. 如果执行操作（如 click/type）超时或报错元素被遮挡/不可交互，请务必先运行 'snapshot' 重新获取当前页面最新的 DOM 状态和元素引用，不要盲目重试。"
        )
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["open", "snapshot", "click", "fill", "type", "get_text",
                             "get_title", "get_url", "screenshot", "wait", "press",
                             "hover", "scroll", "is_visible", "close", "find",
                             "mouse_move", "mouse_click", "mouse_drag", "key_type",
                             "key_press", "screen_capture"],
                    "description": concat!(
                        "要执行的浏览器操作（OS 级操作需要 backend=computer_use）。",
                        "click/fill/type/get_text/hover/is_visible 必须同时提供 selector；",
                        "get_text 只读取指定元素文本，不能省略 selector。"
                    )
                },
                "url": {
                    "type": "string",
                    "description": "要导航到的 URL（用于 'open' 操作）"
                },
                "selector": {
                    "type": "string",
                    "description": concat!(
                        "元素选择器：@ref（例如 @e1）、CSS（#id、.class）或 text=...。",
                        "click、fill、type、get_text、hover、is_visible 必须提供此参数；",
                        "get_text 缺少 selector 会报错，不会回退为整页文本。"
                    )
                },
                "value": {
                    "type": "string",
                    "description": "要填充或输入的值 (注意：find 操作必须提供此参数)"
                },
                "text": {
                    "type": "string",
                    "description": "要输入或等待的文本 (注意：type 操作必须提供此参数)"
                },
                "key": {
                    "type": "string",
                    "description": "要按下的键（Enter、Tab、Escape 等）"
                },
                "x": {
                    "type": "integer",
                    "description": "屏幕 X 坐标（computer_use：mouse_move/mouse_click）"
                },
                "y": {
                    "type": "integer",
                    "description": "屏幕 Y 坐标（computer_use：mouse_move/mouse_click）"
                },
                "from_x": {
                    "type": "integer",
                    "description": "拖动源 X 坐标（computer_use：mouse_drag）"
                },
                "from_y": {
                    "type": "integer",
                    "description": "拖动源 Y 坐标（computer_use：mouse_drag）"
                },
                "to_x": {
                    "type": "integer",
                    "description": "拖动目标 X 坐标（computer_use：mouse_drag）"
                },
                "to_y": {
                    "type": "integer",
                    "description": "拖动目标 Y 坐标（computer_use：mouse_drag）"
                },
                "button": {
                    "type": "string",
                    "enum": ["left", "right", "middle"],
                    "description": "computer_use mouse_click 的鼠标按钮"
                },
                "direction": {
                    "type": "string",
                    "enum": ["up", "down", "left", "right"],
                    "description": "滚动方向"
                },
                "pixels": {
                    "type": "integer",
                    "description": "滚动像素数"
                },
                "interactive_only": {
                    "type": "boolean",
                    "description": "用于 snapshot：仅显示可交互元素"
                },
                "compact": {
                    "type": "boolean",
                    "description": "用于 snapshot：移除空的结构元素"
                },
                "depth": {
                    "type": "integer",
                    "description": "For snapshot: limit tree depth"
                },
                "full_page": {
                    "type": "boolean",
                    "description": "For screenshot: capture full page"
                },
                "path": {
                    "type": "string",
                    "description": "File path for screenshot"
                },
                "ms": {
                    "type": "integer",
                    "description": "Milliseconds to wait"
                },
                "by": {
                    "type": "string",
                    "enum": ["role", "text", "label", "placeholder", "testid"],
                    "description": "For find: semantic locator type"
                },
                "find_action": {
                    "type": "string",
                    "enum": ["click", "fill", "text", "hover", "check"],
                    "description": "For find: action to perform on found element"
                },
                "fill_value": {
                    "type": "string",
                    "description": "For find with fill action: value to fill"
                }
            },
            "required": ["action"]
        })
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec::new(
            crate::app::agent::tools::BROWSER_TOOL_ID,
            self.description(),
            self.parameters_schema(),
        )
        .with_display_name(crate::app::agent::tools::BROWSER_TOOL_ID)
        .with_aliases(vec![crate::app::agent::tools::BROWSER_TOOL_ALIAS.to_string()])
        .with_read_only(false)
        .with_destructive(false)
        .with_concurrency_safe(false)
        .with_requires_user_interaction(false)
        .with_strict(true)
    }

    async fn call(&self, input: Value) -> anyhow::Result<ToolCallResult> {
        let action = input.get("action").and_then(Value::as_str).unwrap_or("browser").to_string();
        let legacy = self.execute(input).await?;

        if !legacy.success {
            let mut result = ToolCallResult::from_legacy_result(legacy);
            result.render_hint = Some(ToolRenderHint {
                title: Some(crate::app::agent::tools::BROWSER_TOOL_ID.to_string()),
                kind: Some("browser".to_string()),
                summary: Some(format!("Failed to run {action}")),
                metadata: json!({ "action": action }),
            });
            return Ok(result);
        }

        let parsed = serde_json::from_str::<Value>(&legacy.output)
            .unwrap_or_else(|_| Value::String(legacy.output.clone()));
        let sanitized = Self::sanitize_payload(&parsed);
        let backend =
            sanitized.get("backend").and_then(Value::as_str).unwrap_or("browser").to_string();
        let summary = Self::summarize_action(&action, &sanitized);
        let data = json!({
            "action": action.clone(),
            "backend": backend.clone(),
            "result": sanitized.clone(),
        });

        Ok(ToolCallResult {
            data: data.clone(),
            model_result: Value::String(legacy.output),
            content_blocks: vec![ToolResultContentDto::Json { value: data.clone() }],
            render_hint: Some(ToolRenderHint {
                title: Some(crate::app::agent::tools::BROWSER_TOOL_ID.to_string()),
                kind: Some("browser".to_string()),
                summary: Some(summary),
                metadata: json!({
                    "action": action,
                    "backend": backend,
                }),
            }),
            telemetry: Some(ToolCallTelemetry { success: true, ..ToolCallTelemetry::default() }),
            ..ToolCallResult::default()
        })
    }

    async fn execute(&self, args: Value) -> anyhow::Result<ToolResult> {
        // Security checks
        if !self.security.can_act() {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some("Action blocked: autonomy is read-only".into()),
            });
        }

        if !self.security.record_action() {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some("Action blocked: rate limit exceeded".into()),
            });
        }

        let backend = match self.resolve_backend().await {
            Ok(selected) => selected,
            Err(error) => {
                return Ok(ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some(error.to_string()),
                });
            }
        };

        // Parse action from args
        let action_str = args
            .get("action")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'action' parameter"))?;

        if !is_supported_browser_action(action_str) {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("Unknown action: {action_str}")),
            });
        }

        if backend == ResolvedBackend::ComputerUse {
            return self.execute_computer_use_action(action_str, &args).await;
        }

        if is_computer_use_only_action(action_str) {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(unavailable_action_for_backend_error(action_str, backend)),
            });
        }

        let action = match parse_browser_action(action_str, &args) {
            Ok(a) => a,
            Err(e) => {
                return Ok(ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some(e.to_string()),
                });
            }
        };

        if let BrowserAction::Screenshot { path: Some(path), .. } = &action {
            if let Err(err) = self.computer_use_client().validate_output_path("path", path) {
                return Ok(ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some(err.to_string()),
                });
            }
        }

        self.execute_action(action, backend).await
    }
}
#[cfg(test)]
#[path = "tool_tests.rs"]
mod tool_tests;
