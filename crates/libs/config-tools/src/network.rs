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

fn default_true() -> bool {
    true
}

fn default_public_domains() -> Vec<String> {
    vec!["*".into()]
}

/// HTTP 请求工具配置（`[http_request]` 配置段）。
///
/// 若显式将 `allowed_domains` 设为空列表，则拒绝所有 HTTP 请求。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct HttpRequestConfig {
    /// 是否启用 `http_request` 工具以进行 API 交互，默认启用。
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// HTTP 请求允许访问的域名，默认 `["*"]` 表示允许所有公网域名。
    #[serde(default = "default_public_domains")]
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
            enabled: default_true(),
            allowed_domains: default_public_domains(),
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
    /// 是否启用 `web_search_tool` 进行网页搜索，默认启用。
    #[serde(default = "default_true")]
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
            enabled: default_true(),
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
