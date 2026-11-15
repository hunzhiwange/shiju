//! Gateway 服务启动选项模块
//!
//! 本模块提供配置网关服务启动行为的选项结构体。
//! 这些选项控制网络绑定、端口、CORS 策略等关键服务参数。

/// 网关服务启动选项
///
/// 定义了网关服务启动时的配置参数，包括监听地址、端口号和 CORS 策略。
///
/// # 示例
///
/// ```
/// use gateway_types::ServeOptions;
///
/// // 使用默认配置
/// let opts = ServeOptions::default();
/// assert_eq!(opts.port, 4099);
///
/// // 自定义配置
/// let custom_opts = ServeOptions {
///     hostname: "0.0.0.0".to_string(),
///     port: 8080,
///     cors: vec!["http://localhost:3000".to_string()],
/// };
/// ```
#[derive(Debug, Clone)]
pub struct ServeOptions {
    /// 服务监听的主机名或 IP 地址
    ///
    /// - `"127.0.0.1"` 表示仅本机访问（默认）
    /// - `"0.0.0.0"` 表示监听所有网络接口（对外暴露）
    /// - 也可以使用具体的主机名或 IP 地址
    pub hostname: String,

    /// 服务监听的端口号
    ///
    /// 默认为 4099。确保该端口未被其他服务占用。
    pub port: u16,

    /// 允许跨域资源共享（CORS）的源列表
    ///
    /// 包含允许跨域访问此服务的源地址（origin）列表。
    /// 每个源应为完整的 URL（包括协议、主机和端口，如果有）。
    ///
    /// - 空列表表示不允许任何跨域请求
    /// - 例如：`["http://localhost:3000", "https://example.com"]`
    pub cors: Vec<String>,
}

impl Default for ServeOptions {
    /// 返回 `ServeOptions` 的默认配置
    ///
    /// # 默认值
    ///
    /// - `hostname`: `"127.0.0.1"`（仅本机访问）
    /// - `port`: `4099`
    /// - `cors`: 空列表（不允许跨域请求）
    ///
    /// # 示例
    ///
    /// ```
    /// use gateway_types::ServeOptions;
    ///
    /// let opts = ServeOptions::default();
    /// assert_eq!(opts.hostname, "127.0.0.1");
    /// assert_eq!(opts.port, 4099);
    /// assert!(opts.cors.is_empty());
    /// ```
    fn default() -> Self {
        Self { hostname: "127.0.0.1".to_string(), port: 4099, cors: Vec::new() }
    }
}

#[cfg(test)]
#[path = "options_tests.rs"]
mod options_tests;
