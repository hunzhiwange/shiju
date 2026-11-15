//! 任务池控制、工作树租约与计划执行 SSE 领域客户端。

#[derive(Debug, Clone)]
pub struct GatewayTaskPoolClient {
    transport: gateway_http::GatewayTransport,
}

impl GatewayTaskPoolClient {
    pub fn new(transport: gateway_http::GatewayTransport) -> Self {
        Self { transport }
    }

    #[cfg(test)]
    fn endpoint(&self) -> &gateway_endpoint::GatewayEndpoint {
        self.transport.endpoint()
    }
}

mod task_pool_api;
#[cfg(test)]
mod test_support;
