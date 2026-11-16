use session_info::Info;
pub async fn delete(
    client: Result<vw_gateway_client::GatewayClient, String>,
    id: String,
) -> Result<(), String> {
    gateway_session::GatewaySessionClient::new(client?.transport().clone())
        .session_delete(&id, None)
        .await
}
pub async fn copy(
    client: Result<vw_gateway_client::GatewayClient, String>,
    id: String,
) -> Result<Info, String> {
    gateway_session::GatewaySessionClient::new(client?.transport().clone())
        .session_fork::<Info>(&id, None, &None)
        .await
        .map_err(|e| e.to_string())
}

/// Read-only navigation projection; the shell owns session removal and runtime teardown.
pub fn next_after_delete(
    rows: &[Info],
    active: Option<&str>,
    deleted: &str,
) -> Option<Option<String>> {
    (active == Some(deleted)).then(|| {
        rows.iter()
            .find(|row| row.id != deleted && row.is_user_navigable())
            .map(|row| row.id.clone())
    })
}
