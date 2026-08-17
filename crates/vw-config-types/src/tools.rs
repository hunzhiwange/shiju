use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// HTTP 工具默认使用的桌面浏览器兼容 User-Agent。
///
/// 使用 macOS Chrome 的精简版本标识；Cookie 和 Client Hints 仍由真实浏览器会话管理。
pub const DEFAULT_USER_AGENT: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) \
AppleWebKit/537.36 (KHTML, like Gecko) Chrome/148.0.0.0 Safari/537.36";

/// 解析 HTTP 工具实际使用的 User-Agent，并升级历史默认值。
pub fn resolve_user_agent(user_agent: &str) -> String {
    let user_agent = user_agent.trim();
    if user_agent.is_empty() || user_agent.eq_ignore_ascii_case("VibeWindow/1.0") {
        DEFAULT_USER_AGENT.into()
    } else {
        user_agent.into()
    }
}

/// 多模态 OCR 图片模型配置（`[multimodal.ocr]` 配置段）。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MultimodalOcrConfig {
    /// 是否启用图片 OCR。默认关闭。
    #[serde(default)]
    pub enabled: bool,
    /// 当前模型是否支持直接接收图片。默认开启；开启时优先由当前模型处理图片。
    #[serde(default = "default_model_supports_images")]
    pub model_supports_images: bool,
    /// OCR 图片模型的完整 `provider/model` 引用。
    #[serde(default)]
    pub model: Option<String>,
}

fn default_model_supports_images() -> bool {
    true
}

impl Default for MultimodalOcrConfig {
    fn default() -> Self {
        Self { enabled: false, model_supports_images: default_model_supports_images(), model: None }
    }
}

/// 多模态图片处理配置（`[multimodal]` 配置段）。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MultimodalConfig {
    /// 每次请求允许携带的图片附件数量上限。
    #[serde(default = "default_multimodal_max_images")]
    pub max_images: usize,
    /// 在 base64 编码前允许的单张图片大小上限，单位为 MiB。
    #[serde(default = "default_multimodal_max_image_size_mb")]
    pub max_image_size_mb: usize,
    /// 是否允许抓取远程图片 URL（http/https）。默认开启。
    #[serde(default = "default_allow_remote_fetch")]
    pub allow_remote_fetch: bool,
    /// 图片 OCR 降级配置。
    #[serde(default)]
    pub ocr: MultimodalOcrConfig,
}

fn default_multimodal_max_images() -> usize {
    10
}

fn default_multimodal_max_image_size_mb() -> usize {
    5
}

fn default_allow_remote_fetch() -> bool {
    true
}

impl MultimodalConfig {
    /// 将配置值裁剪到运行时允许的安全范围内。
    pub fn effective_limits(&self) -> (usize, usize) {
        let max_images = self.max_images.clamp(1, 16);
        let max_image_size_mb = self.max_image_size_mb.clamp(1, 20);
        (max_images, max_image_size_mb)
    }
}

impl Default for MultimodalConfig {
    fn default() -> Self {
        Self {
            max_images: default_multimodal_max_images(),
            max_image_size_mb: default_multimodal_max_image_size_mb(),
            allow_remote_fetch: default_allow_remote_fetch(),
            ocr: MultimodalOcrConfig::default(),
        }
    }
}

/// `computer_use` sidecar 配置（`[browser.computer_use]` 配置段）。
///
/// 将操作系统级的鼠标、键盘与截图动作委托给本地辅助进程。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BrowserComputerUseConfig {
    /// `computer_use` 动作的辅助进程端点，用于执行系统级鼠标、键盘和截图操作。
    #[serde(default = "default_browser_computer_use_endpoint")]
    pub endpoint: String,
    /// `computer_use` 辅助进程的可选 bearer token。
    #[serde(default)]
    pub api_key: Option<String>,
    /// 单个动作请求的超时时间，单位为毫秒。
    #[serde(default = "default_browser_computer_use_timeout_ms")]
    pub timeout_ms: u64,
    /// 是否允许使用远程或公网 computer-use sidecar 端点，默认值为 `false`。
    #[serde(default)]
    pub allow_remote_endpoint: bool,
    /// 可选的窗口标题或进程白名单，会透传给辅助进程策略层。
    #[serde(default)]
    pub window_allowlist: Vec<String>,
    /// 基于坐标的操作可使用的 X 轴边界。
    #[serde(default)]
    pub max_coordinate_x: Option<i64>,
    /// 基于坐标的操作可使用的 Y 轴边界。
    #[serde(default)]
    pub max_coordinate_y: Option<i64>,
}

fn default_browser_computer_use_endpoint() -> String {
    "http://127.0.0.1:8787/v1/actions".into()
}

fn default_browser_computer_use_timeout_ms() -> u64 {
    15_000
}

impl Default for BrowserComputerUseConfig {
    fn default() -> Self {
        Self {
            endpoint: default_browser_computer_use_endpoint(),
            api_key: None,
            timeout_ms: default_browser_computer_use_timeout_ms(),
            allow_remote_endpoint: false,
            window_allowlist: Vec::new(),
            max_coordinate_x: None,
            max_coordinate_y: None,
        }
    }
}

/// 浏览器自动化配置（`[browser]` 配置段）。
///
/// 用于控制 `browser_open` 工具及浏览器自动化后端。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BrowserConfig {
    /// 是否启用 `browser_open` 工具，可在系统浏览器中打开 URL 而不抓取内容。
    #[serde(default)]
    pub enabled: bool,
    /// `browser_open` 允许访问的域名，支持精确匹配或子域匹配。
    #[serde(default)]
    pub allowed_domains: Vec<String>,
    /// `browser_open` 工具使用的打开模式或浏览器。
    ///
    /// 支持值包括：
    /// - `default`、`new_window`、`new_tab`：通过系统浏览器集成打开
    /// - `disable`：禁用 `browser_open` 工具注册
    /// - 兼容旧浏览器名，如 `brave`、`chrome`、`firefox`
    #[serde(default = "default_browser_open")]
    pub browser_open: String,
    /// 浏览器会话名称，用于 agent-browser 自动化。
    #[serde(default)]
    pub session_name: Option<String>,
    /// 浏览器自动化后端：`agent_browser`、`rust_native`、`computer_use`、`auto`。
    #[serde(default = "default_browser_backend")]
    pub backend: String,
    /// rust-native 后端是否启用无头模式。
    #[serde(default = "default_true")]
    pub native_headless: bool,
    /// rust-native 后端的 WebDriver 端点 URL，例如 `http://127.0.0.1:9515`。
    #[serde(default = "default_browser_webdriver_url")]
    pub native_webdriver_url: String,
    /// rust-native 后端可选的 Chrome 或 Chromium 可执行文件路径。
    #[serde(default)]
    pub native_chrome_path: Option<String>,
    /// `computer_use` 辅助进程配置。
    #[serde(default)]
    pub computer_use: BrowserComputerUseConfig,
}

fn default_browser_backend() -> String {
    "agent_browser".into()
}

fn default_browser_open() -> String {
    "default".into()
}

fn default_browser_webdriver_url() -> String {
    "http://127.0.0.1:9515".into()
}

fn default_true() -> bool {
    true
}

impl Default for BrowserConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            allowed_domains: Vec::new(),
            browser_open: default_browser_open(),
            session_name: None,
            backend: default_browser_backend(),
            native_headless: default_true(),
            native_webdriver_url: default_browser_webdriver_url(),
            native_chrome_path: None,
            computer_use: BrowserComputerUseConfig::default(),
        }
    }
}

/// HTTP 请求工具配置（`[http_request]` 配置段）。
///
/// 默认拒绝策略：若 `allowed_domains` 为空，则拒绝所有 HTTP 请求。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct HttpRequestConfig {
    /// 是否启用 `http_request` 工具以进行 API 交互。
    #[serde(default)]
    pub enabled: bool,
    /// HTTP 请求允许访问的域名，支持精确匹配或子域匹配。
    #[serde(default)]
    pub allowed_domains: Vec<String>,
    /// 最大响应体大小，单位为字节；默认值为 `1MB`，`0` 表示不限制。
    #[serde(default = "default_http_max_response_size")]
    pub max_response_size: usize,
    /// 请求超时时间，单位为秒，默认值为 `30`。
    #[serde(default = "default_http_timeout_secs")]
    pub timeout_secs: u64,
    /// HTTP 请求使用的 User-Agent 字符串，可由环境变量 `VIBEWINDOW_HTTP_REQUEST_USER_AGENT` 覆盖。
    #[serde(default = "default_user_agent")]
    pub user_agent: String,
}

impl Default for HttpRequestConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            allowed_domains: vec![],
            max_response_size: default_http_max_response_size(),
            timeout_secs: default_http_timeout_secs(),
            user_agent: default_user_agent(),
        }
    }
}

fn default_http_max_response_size() -> usize {
    1_000_000 // 1MB
}

fn default_http_timeout_secs() -> u64 {
    30
}

/// 网页抓取工具配置（`[web_fetch]` 配置段）。
///
/// 用于抓取网页内容并将 HTML 转为纯文本供 LLM 使用。
/// 域名过滤规则如下：`allowed_domains` 控制允许访问的主机（使用 `["*"]`
/// 表示允许所有公网主机）；`blocked_domains` 的优先级高于 `allowed_domains`。
/// 若 `allowed_domains` 为空，则拒绝所有请求（默认拒绝）。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WebFetchConfig {
    /// 是否启用 `web_fetch` 工具以抓取网页内容。
    #[serde(default)]
    pub enabled: bool,
    /// 提供方：`fast_html2md`、`nanohtml2text`、`firecrawl` 或 `tavily`。
    #[serde(default = "default_web_fetch_provider")]
    pub provider: String,
    /// 提供方可选 API Key；当 provider 为 `firecrawl` 或 `tavily` 时必填。
    /// 多个 Key 可用逗号分隔，以进行轮询负载均衡。
    #[serde(default)]
    pub api_key: Option<String>,
    /// 提供方可选 API URL 覆盖值，用于自托管服务。
    #[serde(default)]
    pub api_url: Option<String>,
    /// web_fetch 允许访问的域名，支持精确匹配或子域匹配；`["*"]` 表示所有公网主机。
    #[serde(default)]
    pub allowed_domains: Vec<String>,
    /// 禁止访问的域名，支持精确匹配或子域匹配，且始终优先于 `allowed_domains`。
    #[serde(default)]
    pub blocked_domains: Vec<String>,
    /// 最大响应体大小，单位为字节；默认值为 `500KB`，纯文本通常远小于原始 HTML。
    #[serde(default = "default_web_fetch_max_response_size")]
    pub max_response_size: usize,
    /// 请求超时时间，单位为秒，默认值为 `30`。
    #[serde(default = "default_web_fetch_timeout_secs")]
    pub timeout_secs: u64,
    /// 抓取请求使用的 User-Agent 字符串，可由环境变量 `VIBEWINDOW_WEB_FETCH_USER_AGENT` 覆盖。
    #[serde(default = "default_user_agent")]
    pub user_agent: String,
}

fn default_web_fetch_max_response_size() -> usize {
    500_000 // 500KB
}

fn default_web_fetch_provider() -> String {
    "fast_html2md".into()
}

fn default_web_fetch_timeout_secs() -> u64 {
    30
}

impl Default for WebFetchConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            provider: default_web_fetch_provider(),
            api_key: None,
            api_url: None,
            allowed_domains: vec!["*".into()],
            blocked_domains: vec![],
            max_response_size: default_web_fetch_max_response_size(),
            timeout_secs: default_web_fetch_timeout_secs(),
            user_agent: default_user_agent(),
        }
    }
}

/// 网页搜索工具配置（`[web_search]` 配置段）。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WebSearchConfig {
    /// 是否启用 `web_search_tool` 进行网页搜索。
    #[serde(default)]
    pub enabled: bool,
    /// 搜索提供方：`duckduckgo`（免费且无需 API Key）、`brave`、`serper`、`google`、
    /// `bing`、`firecrawl` 或 `tavily`。
    #[serde(default = "default_web_search_provider")]
    pub provider: String,
    /// 通用 provider API Key，可用于 serper、google、bing、firecrawl、tavily，
    /// 也可作为 brave 的回退 Key。
    /// 多个 Key 可用逗号分隔，以进行轮询负载均衡。
    #[serde(default)]
    pub api_key: Option<String>,
    /// provider 可选 API URL 覆盖值，用于兼容网关或自定义端点。
    #[serde(default)]
    pub api_url: Option<String>,
    /// Brave Search API Key；当 provider 为 `brave` 时需要设置。
    #[serde(default)]
    pub brave_api_key: Option<String>,
    /// 每次搜索返回的最大结果数，范围为 `1-10`。
    #[serde(default = "default_web_search_max_results")]
    pub max_results: usize,
    /// 请求超时时间，单位为秒。
    #[serde(default = "default_web_search_timeout_secs")]
    pub timeout_secs: u64,
    /// 搜索请求使用的 User-Agent 字符串，可由环境变量 `VIBEWINDOW_WEB_SEARCH_USER_AGENT` 覆盖。
    #[serde(default = "default_user_agent")]
    pub user_agent: String,
}

fn default_web_search_provider() -> String {
    "duckduckgo".into()
}

fn default_web_search_max_results() -> usize {
    5
}

fn default_web_search_timeout_secs() -> u64 {
    15
}

impl Default for WebSearchConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            provider: default_web_search_provider(),
            api_key: None,
            api_url: None,
            brave_api_key: None,
            max_results: default_web_search_max_results(),
            timeout_secs: default_web_search_timeout_secs(),
            user_agent: default_user_agent(),
        }
    }
}

fn default_user_agent() -> String {
    resolve_user_agent("")
}

/// 委托代理内置必选工具主 ID 目录。
///
/// 匹配时还需覆盖既有别名（见 [`is_builtin_required_tool_id`]）；本切片仅列主 ID，
/// 供 UI 展示与白名单合并使用。
pub const BUILTIN_REQUIRED_TOOL_IDS: &[&str] = &[
    "file_read",
    "file_write",
    "ls",
    "glob",
    "grep",
    "rustcodegraph",
    "apply_patch",
    "bash",
    "http_request",
    "image_info",
    "screenshot",
    "AgentTool",
    "skill",
    "schedule",
    "memory_forget",
    "memory_recall",
    "memory_store",
    "batch",
    "AskUserQuestion",
    "Browser",
    "BrowserOpen",
    "Config",
    "TodoRead",
    "TodoWrite",
    "WebSearch",
    "file_edit",
    "image_asset_read",
    "tool_search",
    "Sleep",
    "McpStatus",
];

/// 主 ID 之外、仍视为同一内置必选工具的既有别名。
const BUILTIN_REQUIRED_TOOL_ALIASES: &[&str] = &[
    "shell",
    "question",
    "browser",
    "browser_open",
    "todoread",
    "todowrite",
    "web_search",
    "websearch",
    "web_search_tool",
    "edit",
    "sleep",
    "mcp_status",
];

/// 判断工具 ID 是否属于内置必选目录（含主 ID 与既有别名）。
pub fn is_builtin_required_tool_id(tool_id: &str) -> bool {
    BUILTIN_REQUIRED_TOOL_IDS.contains(&tool_id) || BUILTIN_REQUIRED_TOOL_ALIASES.contains(&tool_id)
}

/// 判断工具 ID 是否为 MCP 动态工具（形如 `mcp__{server}__{tool}`）。
///
/// 要求以 `mcp__` 为前缀，且 `__` 分隔后至少两段非空（server 与 tool）。
pub fn is_mcp_dynamic_tool_id(tool_id: &str) -> bool {
    let Some(rest) = tool_id.strip_prefix("mcp__") else {
        return false;
    };
    let mut parts = rest.split("__");
    match (parts.next(), parts.next()) {
        (Some(server), Some(tool)) => {
            !server.is_empty() && !tool.is_empty() && parts.all(|part| !part.is_empty())
        }
        _ => false,
    }
}

#[cfg(test)]
#[path = "tools_tests.rs"]
mod tools_tests;
