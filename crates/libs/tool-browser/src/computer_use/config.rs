/// 计算机使用边车服务配置
///
/// 定义了与计算机使用服务交互所需的所有配置参数，包括端点地址、
/// 超时时间、安全策略、坐标限制等。
///
/// # 安全性
///
/// - `allow_remote_endpoint` 为 false 时，仅允许连接本地私有地址
/// - 当允许远程端点时，必须使用 HTTPS 协议
/// - `window_allowlist` 限制可操作的窗口列表
/// - `max_coordinate_x/y` 限制鼠标操作的最大坐标范围
///
/// # 示例
///
/// ```rust,ignore
/// let config = ComputerUseConfig {
///     endpoint: "http://127.0.0.1:8787/v1/actions".to_string(),
///     timeout_ms: 15_000,
///     allow_remote_endpoint: false,
///     ..Default::default()
/// };
/// ```
#[derive(Clone)]
pub struct ComputerUseConfig {
    /// 计算机使用服务的 HTTP 端点地址
    /// 必须是有效的 HTTP 或 HTTPS URL
    pub endpoint: String,

    /// 可选的 API 密钥，用于向边车服务进行身份验证
    /// 如果提供，将作为 Bearer Token 发送
    pub api_key: Option<String>,

    /// 请求超时时间（毫秒）
    /// 必须大于 0，用于防止长时间挂起的请求
    pub timeout_ms: u64,

    /// 是否允许连接远程端点
    /// false: 仅允许连接本地私有地址（默认）
    /// true: 允许连接公网地址，但必须使用 HTTPS
    pub allow_remote_endpoint: bool,

    /// 允许操作的窗口标题白名单
    /// 为空表示允许所有窗口
    pub window_allowlist: Vec<String>,

    /// X 坐标最大值限制
    /// 用于限制鼠标移动和点击的水平范围
    pub max_coordinate_x: Option<i64>,

    /// Y 坐标最大值限制
    /// 用于限制鼠标移动和点击的垂直范围
    pub max_coordinate_y: Option<i64>,
}

/// 为 ComputerUseConfig 实现自定义 Debug trait
///
/// 出于安全考虑，在调试输出中排除敏感字段 `api_key`，
/// 防止在日志或调试信息中泄露认证凭证。
impl std::fmt::Debug for ComputerUseConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ComputerUseConfig")
            .field("endpoint", &self.endpoint)
            .field("timeout_ms", &self.timeout_ms)
            .field("allow_remote_endpoint", &self.allow_remote_endpoint)
            .field("window_allowlist", &self.window_allowlist)
            .field("max_coordinate_x", &self.max_coordinate_x)
            .field("max_coordinate_y", &self.max_coordinate_y)
            .finish_non_exhaustive()
    }
}

/// 为 ComputerUseConfig 提供默认配置
///
/// 默认配置提供安全的初始值：
/// - 端点地址：本地回环地址 `http://127.0.0.1:8787/v1/actions`
/// - 超时时间：15 秒
/// - 不允许远程端点
/// - 空的窗口白名单
/// - 无坐标限制
impl Default for ComputerUseConfig {
    fn default() -> Self {
        Self {
            endpoint: "http://127.0.0.1:8787/v1/actions".into(),
            api_key: None,
            timeout_ms: 15_000,
            allow_remote_endpoint: false,
            window_allowlist: Vec::new(),
            max_coordinate_x: None,
            max_coordinate_y: None,
        }
    }
}
#[cfg(test)]
#[path = "config_tests.rs"]
mod config_tests;
