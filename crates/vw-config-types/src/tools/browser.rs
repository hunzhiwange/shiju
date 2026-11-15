use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

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
    /// 是否启用浏览器工具，默认启用。
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// 浏览器工具允许访问的域名，默认 `["*"]` 表示允许所有公网域名。
    #[serde(default = "default_public_domains")]
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

fn default_public_domains() -> Vec<String> {
    vec!["*".into()]
}

impl Default for BrowserConfig {
    fn default() -> Self {
        Self {
            enabled: default_true(),
            allowed_domains: default_public_domains(),
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
