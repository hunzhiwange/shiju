use crate::distribution::project;
use gateway_mcp_distribution::{McpBindingRequest, McpBulkRequest};
use vw_api_mcp_management::*;
use vw_gateway_client::GatewayClient;
#[derive(Debug, Clone, Default)]
pub struct BindingsState {
    pub context: Option<crate::management_context::Context>,
    pub selected: Vec<McpCatalogRecord>,
    pub running: bool,
    pub generation: u64,
    pub result: Option<Result<McpDistributionResult, String>>,
}
#[derive(Debug, Clone)]
pub enum BindingsMessage {
    Select(McpCatalogRecord, bool),
    Apply(McpAppTarget, bool),
    RetryFailed,
    Finished {
        context: crate::management_context::Context,
        generation: u64,
        result: Result<McpDistributionResult, String>,
    },
}
pub async fn apply(
    client: GatewayClient,
    records: Vec<McpCatalogRecord>,
    target: McpAppTarget,
    enabled: bool,
    token: String,
) -> Result<McpDistributionResult, String> {
    let project_id = project(&records)?;
    let response = if records.len() == 1 {
        gateway_mcp_distribution::GatewayMcpDistributionClient::new(client.transport().clone())
            .mcp_management_bind(
                &records[0].record.id,
                &McpBindingRequest {
                    target: target.clone(),
                    project_id,
                    expected_revision: records[0].record.revision,
                    enabled,
                },
                &token,
            )
            .await
    } else {
        gateway_mcp_distribution::GatewayMcpDistributionClient::new(client.transport().clone())
            .mcp_management_bulk(
                &McpBulkRequest {
                    target: target.clone(),
                    project_id,
                    records: records.iter().map(|r| r.record.clone()).collect(),
                    enabled,
                },
                &token,
            )
            .await
    }
    .map_err(|_| "绑定请求失败；请刷新目录核实期望与原生状态".to_owned())?;
    // Do not call an incomplete, duplicate or foreign response successful.
    if response.items.len() != records.len()
        || records.iter().any(|record| {
            response
                .items
                .iter()
                .filter(|item| {
                    item.record.id == record.record.id
                        && item.record.revision >= record.record.revision
                        && item.target == target
                        && item.enabled == enabled
                })
                .count()
                != 1
        })
    {
        return Err("响应不完整或目标不匹配；请刷新核实结果".into());
    }
    Ok(response)
}
impl BindingsState {
    pub fn finish(
        &mut self,
        current: &crate::management_context::Context,
        request: &crate::management_context::Context,
        generation: u64,
        result: Result<McpDistributionResult, String>,
    ) -> bool {
        if current != request
            || self.context.as_ref() != Some(request)
            || self.generation != generation
            || !self.running
        {
            return false;
        }
        self.running = false;
        self.selected.clear();
        self.result = Some(result);
        true
    }
}
