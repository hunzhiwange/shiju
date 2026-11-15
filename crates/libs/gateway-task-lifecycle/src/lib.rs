//! 任务生命周期工作流领域客户端。

#[derive(Debug, Clone)]
pub struct GatewayTaskLifecycleClient {
    transport: gateway_http::GatewayTransport,
}

impl GatewayTaskLifecycleClient {
    pub fn new(transport: gateway_http::GatewayTransport) -> Self {
        Self { transport }
    }
}

mod task_lifecycle_api;
#[cfg(test)]
mod test_support;
