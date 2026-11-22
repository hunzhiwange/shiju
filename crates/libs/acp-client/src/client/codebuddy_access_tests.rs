use super::*;

#[tokio::test]
async fn acp_platform_codebuddy_contract_rejects_managed_bypass() {
    let mut client = AcpClient::new("codebuddy", AcpAgentConfig::default())
        .with_full_access(true)
        .with_permission_mode(PermissionMode::ApproveAll);
    client.guard_codebuddy_mode("session", FULL_MODE).await.unwrap();
    client.config.env.insert("VW_ACP_LAUNCH_REFERENCE".into(), "vw-session-model:fixture".into());
    assert!(client.guard_codebuddy_mode("session", FULL_MODE).await.is_err());
    assert!(client.guard_codebuddy_mode("session", "fullAccess").await.is_err());
    assert!(client.guard_codebuddy_mode("session", "dontAsk").await.is_err());
    assert!(client.check_codebuddy_host_permissions().is_err());

    client.full_access = false;
    client.permission_mode = PermissionMode::DenyAll;
    assert!(client.check_codebuddy_host_permissions().is_err());

    client.permission_mode = PermissionMode::ApproveReads;
    client.check_codebuddy_host_permissions().unwrap();
    client.guard_codebuddy_mode("session", RESTRICTED_MODE).await.unwrap();
}
