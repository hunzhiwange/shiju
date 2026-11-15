//! Agent Scheduler 任务调度领域客户端。

#[derive(Debug, Clone)]
pub struct GatewaySchedulerClient {
    transport: gateway_http::GatewayTransport,
}

impl GatewaySchedulerClient {
    pub fn new(transport: gateway_http::GatewayTransport) -> Self {
        Self { transport }
    }
}

mod agent_scheduler_api;
#[cfg(test)]
mod test_support;
#[cfg(test)]
mod agent_scheduler_api_tests;
