//! 授权执行主机的外部会话读取、恢复及维护客户端。
pub use gateway_app_read::AgentAppsReadError;

#[derive(Debug, Clone)]
pub struct GatewayExternalSessionClient {
    transport: gateway_http::GatewayTransport,
}

impl GatewayExternalSessionClient {
    pub fn new(transport: gateway_http::GatewayTransport) -> Self {
        Self { transport }
    }
}

mod catalog;

mod messages;

mod stream;
#[cfg(not(target_arch = "wasm32"))]
pub use stream::ExternalSessionStream;

mod content;

mod images;

mod delete;

mod resume;

mod maintenance;

mod history_settings;

mod compression;

mod markdown;
