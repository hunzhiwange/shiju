use session_info::Info;
pub async fn delete(client: Result<vw_gateway_client::GatewayClient, String>, id: String) -> Result<(), String> {
    gateway_session::GatewaySessionClient::new(client?.transport().clone()).session_delete(&id, None).await
}
pub async fn copy(client: Result<vw_gateway_client::GatewayClient, String>, id: String) -> Result<Info, String> {
    gateway_session::GatewaySessionClient::new(client?.transport().clone()).session_fork::<Info>(&id, None, &None).await.map_err(|e| e.to_string())
}
