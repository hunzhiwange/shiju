use gateway_app_auth::{AgentAppsAuthCredentials, AgentAppsAuthReadError};
use desktop_settings_account::types::ServiceAccounts;
use vw_gateway_client::GatewayClient;
use vw_api_app_auth::{AuthService, AuthErrorCode};
pub const SERVICES: [AuthService; 7] = [
    AuthService::ClaudeOfficial,
    AuthService::CodexOauth,
    AuthService::GithubCopilot,
    AuthService::XaiOauth,
    AuthService::KimiOauth,
    AuthService::AgyOauth,
    AuthService::DshAccount,
];

pub async fn load(
    client: &GatewayClient,
    host: &str,
    credentials: &AgentAppsAuthCredentials<'_>,
) -> Vec<ServiceAccounts> {
    iced::futures::future::join_all(
        SERVICES.map(|service| load_service(client, host, credentials, service)),
    )
    .await
}

pub async fn load_service(
    client: &GatewayClient,
    host: &str,
    credentials: &AgentAppsAuthCredentials<'_>,
    service: AuthService,
) -> ServiceAccounts {
    let accounts = if client.endpoint().is_loopback() {
        gateway_app_auth::GatewayAppAuthClient::new(client.transport().clone())
            .agent_apps_auth_accounts(host, service, credentials)
            .await
            .map_err(read_error_label)
            .and_then(|accounts| {
                if accounts.iter().any(|account| account.service != service) {
                    Err("授权账号响应与所选服务不符")
                } else {
                    Ok(accounts)
                }
            })
    } else {
        Err("授权中心需要本机回环管理连接")
    };
    ServiceAccounts { service, accounts }
}

pub fn read_error_label(error: AgentAppsAuthReadError) -> &'static str {
    use AuthErrorCode::*;
    match error {
        AgentAppsAuthReadError::Http { error: Some(error), .. } => match error.code {
            InvalidInput => "执行主机或授权查询参数无效",
            Unauthenticated => "管理凭据或主体令牌无效",
            Forbidden => "当前主体没有此服务的账号读取权限",
            NotFound => "找不到所选执行主机或授权资源",
            Unsupported => "执行主机不支持此服务的账号读取",
            ExecutionChannelUnavailable => "执行主机没有可用的授权通道",
            ExecutionHostUnavailable => "授权执行主机不可用",
            KeychainUnavailable => "执行主机的钥匙串不可用",
            StorageUnavailable => "执行主机的授权存储不可用",
            AccountInUse => "账号正在被执行主机使用，请稍后重试",
            _ => "此服务的账号状态暂不可用",
        },
        AgentAppsAuthReadError::Http { status: 401 | 403, .. } => "管理认证被拒绝，请检查连接凭据",
        AgentAppsAuthReadError::Http { .. } => "网关拒绝了账号查询，请检查管理连接",
        AgentAppsAuthReadError::InvalidPrincipalToken => "主体令牌格式无效",
        AgentAppsAuthReadError::Decode => "网关返回了无效的授权账号响应",
        AgentAppsAuthReadError::Transport => "无法连接管理网关，请重试",
    }
}
