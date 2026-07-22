//! 网关端点与认证模型。
//!
//! 本模块只负责描述“请求发到哪里”以及“请求如何认证”，不包含任何实际网络逻辑。

use vw_api_types::id::TenantScope;

/// 网关服务地址，负责统一描述主机与端口。
///
/// 该结构也承载可选认证信息，便于上层把网络位置与访问凭证作为一个整体传递。
#[derive(Debug, Clone)]
pub struct GatewayEndpoint {
    /// 网关主机名或 IP。
    pub host: String,
    /// 网关监听端口。
    pub port: u16,
    /// 请求需要携带的认证信息。
    pub auth: Option<GatewayAuth>,
    /// 请求所属的平台与公司租户作用域。
    tenant_scope: TenantScope,
}

/// 网关支持的认证参数集合。
///
/// 当前使用 skey 的 Bearer 认证；为空时表示匿名访问。
#[derive(Debug, Clone, Default)]
pub struct GatewayAuth {
    /// 作为 `Authorization: Bearer <skey>` 发送的网关 skey。
    pub skey: Option<String>,
}

impl GatewayEndpoint {
    /// 创建新的网关端点配置。
    ///
    /// 新建实例默认不携带认证信息，如有需要可继续调用 [`with_auth`](Self::with_auth)。
    pub fn new(host: impl Into<String>, port: u16) -> Self {
        Self { host: host.into(), port, auth: None, tenant_scope: TenantScope::default() }
    }

    /// 返回带认证信息的新端点副本。
    ///
    /// 该方法采用链式写法，便于在构建端点时一次性补齐认证配置。
    pub fn with_auth(mut self, auth: GatewayAuth) -> Self {
        self.auth = Some(auth);
        self
    }

    /// 返回切换到指定租户作用域的新端点副本。
    pub fn with_tenant_scope(mut self, tenant_scope: TenantScope) -> Self {
        self.tenant_scope = tenant_scope;
        self
    }

    /// 返回当前请求使用的租户作用域。
    pub fn tenant_scope(&self) -> &TenantScope {
        &self.tenant_scope
    }

    /// 返回规范化后的主机名，空白主机回退到本地回环地址。
    ///
    /// 这样可以避免空字符串主机在后续 URL 拼接阶段产生非法地址。
    pub fn normalized_host(&self) -> &str {
        let trimmed = self.host.trim();
        if trimmed.is_empty() { "127.0.0.1" } else { trimmed }
    }

    /// 生成 HTTP 请求使用的基础 URL。
    ///
    /// 当前统一使用 HTTP 协议，协议升级策略由网关部署侧负责。
    pub fn base_url(&self) -> String {
        format!("http://{}:{}", self.normalized_host(), self.port)
    }

    /// 生成人类可读的 `host:port` 描述字符串。
    pub fn describe(&self) -> String {
        format!("{}:{}", self.normalized_host(), self.port)
    }
}

#[cfg(test)]
#[path = "endpoint_tests.rs"]
mod endpoint_tests;
