use super::GatewayClient;
use vw_api_tools::ListToolSpecsResponse;

impl GatewayClient {
    /// 获取网关当前暴露的工具名称列表。
    pub async fn tools_list(&self) -> Result<Vec<String>, String> {
        let response: ListToolSpecsResponse = self.get_json("/v1/tools", &[]).await?;
        Ok(response.items.into_iter().map(|item| item.id.0).collect())
    }
}

#[cfg(test)]
#[path = "tools_api_tests.rs"]
mod tools_api_tests;
