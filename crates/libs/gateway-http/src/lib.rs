//! 共享网关 HTTP 传输与 JSON 请求。

use gateway_endpoint::GatewayEndpoint;

mod http;
mod request_helpers;
pub use http::*;
#[cfg(not(target_arch = "wasm32"))]
pub use request_helpers::get_json_blocking;

#[derive(Debug, Clone)]
pub struct GatewayTransport {
    endpoint: GatewayEndpoint,
    client: reqwest::Client,
    #[cfg(not(target_arch = "wasm32"))]
    dynamic_host_client: reqwest::Client,
}

impl GatewayTransport {
    /// 使用给定端点创建客户端，并在原生平台上配置默认请求超时。
    ///
    /// # 错误
    ///
    /// 当底层 `reqwest::Client` 构建失败时，返回字符串化后的错误信息。
    pub fn new(endpoint: GatewayEndpoint) -> Result<Self, String> {
        #[cfg(not(target_arch = "wasm32"))]
        let builder = || {
            let builder = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(REQUEST_TIMEOUT_SECS));
            if endpoint.is_loopback() { builder.no_proxy() } else { builder }
        };
        #[cfg(target_arch = "wasm32")]
        let builder = reqwest::Client::builder;
        #[cfg(not(target_arch = "wasm32"))]
        let dynamic_host_client = builder()
            .redirect(reqwest::redirect::Policy::none())
            .retry(reqwest::retry::never())
            .build()
            .map_err(|err| err.to_string())?;
        let client = builder().build().map_err(|err| err.to_string())?;
        Ok(Self {
            endpoint,
            client,
            #[cfg(not(target_arch = "wasm32"))]
            dynamic_host_client,
        })
    }

    // Browser fetch cannot enforce the native no-redirect/no-retry transport policy.
    pub fn native_transport(&self) -> Result<&reqwest::Client, String> {
        #[cfg(target_arch = "wasm32")]
        {
            Err("authorized application requests require native transport".into())
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            Ok(&self.dynamic_host_client)
        }
    }

    /// 返回当前客户端绑定的网关端点。
    ///
    /// 可用于日志、诊断或生成派生请求配置。
    pub fn endpoint(&self) -> &GatewayEndpoint {
        &self.endpoint
    }

    /// 复用禁止重定向与重试的原生宿主连接池。
    #[cfg(not(target_arch = "wasm32"))]
    pub fn dynamic_host_client(&self) -> &reqwest::Client {
        &self.dynamic_host_client
    }

    /// 复用普通请求连接池。
    pub fn client(&self) -> &reqwest::Client {
        &self.client
    }
}

#[cfg(test)]
mod test_support;

mod agent_apps_error;
pub use agent_apps_error::AgentAppsReadError;
