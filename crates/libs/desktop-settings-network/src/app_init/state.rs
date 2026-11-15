#[derive(Debug, Clone)]
pub struct WebSearchSettingsState {
    pub enabled: bool,
    pub provider: String,
    pub api_key_input: String,
    pub api_url_input: String,
    pub brave_api_key_input: String,
    pub max_results_input: String,
    pub timeout_secs_input: String,
    pub user_agent: String,
    pub show_help_modal: bool,
    pub save_error: Option<String>,
}

impl Default for WebSearchSettingsState {
    fn default() -> Self {
        let config = config_tools::WebSearchConfig::default();
        Self {
            enabled: config.enabled,
            provider: config.provider,
            api_key_input: String::new(),
            api_url_input: String::new(),
            brave_api_key_input: String::new(),
            max_results_input: config.max_results.to_string(),
            timeout_secs_input: config.timeout_secs.to_string(),
            user_agent: config.user_agent,
            show_help_modal: false,
            save_error: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct HttpRequestSettingsState {
    pub enabled: bool,
    pub allowed_domains: Vec<String>,
    pub new_allowed_domain_input: String,
    pub max_response_size: u32,
    pub timeout_secs: u32,
    pub user_agent: String,
    pub save_error: Option<String>,
}

impl Default for HttpRequestSettingsState {
    fn default() -> Self {
        let config = config_tools::HttpRequestConfig::default();
        Self {
            enabled: config.enabled,
            allowed_domains: config.allowed_domains,
            new_allowed_domain_input: String::new(),
            max_response_size: 1_000_000,
            timeout_secs: 30,
            user_agent: config.user_agent,
            save_error: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ProxySettingsState {
    /// 是否启用代理
    pub enabled: bool,
    /// HTTP 代理地址
    pub http_proxy: String,
    /// HTTPS 代理地址
    pub https_proxy: String,
    /// 全局代理地址
    pub all_proxy: String,
    /// 不使用代理的地址输入
    pub no_proxy_input: String,
    /// 代理作用域
    pub scope: config_proxy::ProxyScope,
    /// 服务列表输入
    pub services_input: String,
    /// 是否显示帮助对话框
    pub show_help_modal: bool,
    /// 保存错误信息
    pub save_error: Option<String>,
}

impl Default for ProxySettingsState {
    fn default() -> Self {
        Self {
            enabled: false,
            http_proxy: String::new(),
            https_proxy: String::new(),
            all_proxy: String::new(),
            no_proxy_input: String::new(),
            scope: config_proxy::ProxyScope::Vibewindow,
            services_input: String::new(),
            show_help_modal: false,
            save_error: None,
        }
    }
}

/// 隧道设置面板状态
///
/// 管理网关公网暴露的隧道 provider 及其参数。
#[derive(Debug, Clone)]
pub struct TunnelSettingsState {
    /// 当前选择的隧道提供商
    pub provider: String,
    /// Cloudflare Tunnel token
    pub cloudflare_token: String,
    /// Tailscale 是否启用 funnel
    pub tailscale_funnel: bool,
    /// Tailscale 主机名
    pub tailscale_hostname: String,
    /// ngrok auth token
    pub ngrok_auth_token: String,
    /// ngrok 自定义域名
    pub ngrok_domain: String,
    /// 自定义隧道启动命令
    pub custom_start_command: String,
    /// 自定义隧道健康检查 URL
    pub custom_health_url: String,
    /// 自定义隧道公网 URL 匹配模式
    pub custom_url_pattern: String,
    /// 保存错误信息
    pub save_error: Option<String>,
}

impl Default for TunnelSettingsState {
    fn default() -> Self {
        Self {
            provider: "none".to_string(),
            cloudflare_token: String::new(),
            tailscale_funnel: false,
            tailscale_hostname: String::new(),
            ngrok_auth_token: String::new(),
            ngrok_domain: String::new(),
            custom_start_command: String::new(),
            custom_health_url: String::new(),
            custom_url_pattern: String::new(),
            save_error: None,
        }
    }
}
