//! 代理配置检测模块
//!
//! 本模块提供了用于检测系统是否配置了 HTTP/HTTPS 代理的功能。
//! 通过检查标准环境变量来判断代理配置状态，支持大小写不敏感的代理环境变量。
//!
//! # 支持的环境变量
//!
//! - `HTTP_PROXY` / `http_proxy` - HTTP 代理配置
//! - `HTTPS_PROXY` / `https_proxy` - HTTPS 代理配置
//!
//! # 示例
//!
//! ```rust
//! use agent_context::proxied::proxied;
//!
//! if proxied() {
//!     println!("系统已配置代理");
//! } else {
//!     println!("系统未配置代理");
//! }
//! ```

/// 检测系统是否配置了 HTTP/HTTPS 代理
///
/// 该函数通过检查标准代理环境变量来判断系统是否配置了代理。
/// 支持以下环境变量（大小写不敏感）：
/// - `HTTP_PROXY` / `http_proxy` - HTTP 代理配置
/// - `HTTPS_PROXY` / `https_proxy` - HTTPS 代理配置
///
/// # 返回值
///
/// - `true` - 至少一个代理环境变量被设置
/// - `false` - 所有代理环境变量都未设置
///
/// # 示例
///
/// ```rust
/// use agent_context::proxied::proxied;
///
/// // 检查是否配置了代理
/// if proxied() {
///     println!("检测到代理配置");
/// } else {
///     println!("未检测到代理配置");
/// }
/// ```
///
/// # 注意事项
///
/// - 该函数仅检查环境变量是否存在，不验证代理配置的有效性
/// - 不检查环境变量的值是否为有效的代理地址
/// - 支持大小写不敏感的环境变量名称
pub fn proxied() -> bool {
    // 检查 HTTP_PROXY 环境变量（大写形式）
    std::env::var_os("HTTP_PROXY").is_some()
        // 检查 HTTPS_PROXY 环境变量（大写形式）
        || std::env::var_os("HTTPS_PROXY").is_some()
        // 检查 http_proxy 环境变量（小写形式）
        || std::env::var_os("http_proxy").is_some()
        // 检查 https_proxy 环境变量（小写形式）
        || std::env::var_os("https_proxy").is_some()
}
