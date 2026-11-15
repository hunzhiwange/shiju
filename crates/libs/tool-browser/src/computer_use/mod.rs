//! 计算机使用工具模块
//!
//! 本模块实现了与计算机使用边车服务（Computer-Use Sidecar）的交互功能，
//! 提供浏览器自动化操作能力，包括鼠标控制、键盘输入、屏幕截图等功能。
//!
//! # 核心功能
//!
//! - **端点验证**: 确保计算机使用服务端点的安全性和可达性
//! - **URL 验证**: 验证导航 URL 是否符合安全策略和域名白名单
//! - **坐标验证**: 限制鼠标操作坐标范围，防止越界操作
//! - **路径验证**: 验证文件输出路径是否符合安全策略
//! - **动作执行**: 将验证后的动作发送到计算机使用服务执行
//!
//! # 安全机制
//!
//! - 禁止访问本地私有主机地址
//! - 强制使用 HTTPS 协议（当允许远程端点时）
//! - 文件输出路径必须通过安全策略检查
//! - 坐标范围受配置限制
//! - 键盘输入长度和字符受限制
//!
//! # 架构
//!
//! ```text
//! ComputerUseClient
//!     ├── validation   -> 端点、URL、坐标、路径与动作参数校验
//!     ├── execution    -> 请求构建、发送与响应解析
//!     ├── config       -> 对外配置结构
//!     └── response     -> 边车响应结构
//! ```

use agent_security::SecurityPolicy;
use std::sync::Arc;

mod config;
mod execution;
mod response;
mod validation;

pub use config::ComputerUseConfig;

/// 计算机使用客户端
///
/// 负责与计算机使用边车服务进行交互，提供浏览器自动化功能。
/// 该客户端封装了所有的安全验证逻辑，确保操作符合安全策略。
///
/// # 职责
///
/// 1. **端点验证**: 验证服务端点的 URL 格式和安全性
/// 2. **URL 验证**: 检查浏览器导航 URL 是否在允许的域名列表中
/// 3. **坐标验证**: 限制鼠标操作的坐标范围
/// 4. **路径验证**: 确保文件输出路径符合安全策略
/// 5. **动作执行**: 将验证后的动作发送到边车服务
///
/// # 安全性
///
/// - 所有 URL 都会检查是否为私有地址
/// - 所有文件路径都会通过安全策略验证
/// - 鼠标坐标受配置限制
/// - 键盘输入有长度和字符限制
///
/// # 线程安全
///
/// 该结构体实现了 `Clone`，内部使用 `Arc<SecurityPolicy>` 共享安全策略，
/// 可以在多线程环境中安全使用。
#[derive(Clone)]
pub(crate) struct ComputerUseClient {
    /// 安全策略引用，用于路径验证和访问控制
    security: Arc<SecurityPolicy>,

    /// 允许访问的域名列表
    /// 仅这些域名的 URL 可以在浏览器中打开
    allowed_domains: Vec<String>,

    /// 会话名称，用于在边车服务中标识不同的会话
    session_name: Option<String>,

    /// 计算机使用服务配置
    config: ComputerUseConfig,
}

impl ComputerUseClient {
    /// 创建新的计算机使用客户端实例
    pub(crate) fn new(
        security: Arc<SecurityPolicy>,
        allowed_domains: Vec<String>,
        session_name: Option<String>,
        config: ComputerUseConfig,
    ) -> Self {
        Self { security, allowed_domains, session_name, config }
    }
}
#[cfg(test)]
mod tests;
