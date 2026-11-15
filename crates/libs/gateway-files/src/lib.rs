//! 文件与目录的网关领域客户端。

mod file_api;
pub use file_api::{DirectoryFileReadResponse, DirectoryFileWriteResponse};

/// 显式复用网关传输及其连接池。
#[derive(Debug, Clone)]
pub struct GatewayFilesClient {
    transport: gateway_http::GatewayTransport,
}

impl GatewayFilesClient {
    pub fn new(transport: gateway_http::GatewayTransport) -> Self {
        Self { transport }
    }
}

#[cfg(test)]
mod test_support;
