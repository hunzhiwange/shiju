//! Drive desktop status reads and cancellation through the scoped gateway SDK.

use super::{Event, message};
use crate::app::{
    Message,
    state::agent_apps_auth::{Connection, DeviceChallenge, DeviceTarget, Scope, State},
};
use iced::{
    Task,
    futures::{SinkExt, channel::mpsc},
};
use std::time::Duration;
use tokio::sync::oneshot;
use vw_gateway_client::{
    AgentAppsAuthCredentials, AgentAppsAuthError, GatewayClient,
    vw_api_types::{
        TimestampMs,
        agent_apps_auth::{AuthErrorCode, AuthFlow, AuthFlowId, AuthStartResponse},
    },
};

#[derive(Clone)]
pub enum Update {
    Started(DeviceChallenge),
    Pending(TimestampMs),
    Approved,
    Failed(&'static str),
    CancelFailed,
}

pub(super) fn start(
    state: &mut State,
    scope: Scope,
    client: GatewayClient,
    connection: Connection,
    target: DeviceTarget,
) -> Task<Message> {
    let (generation, cancelled) = state.begin_device_login(target);
    Task::run(
        iced::stream::channel(4, async move |updates| {
            run(client, connection, target, cancelled, updates).await;
        }),
        move |update| message(Event::DeviceUpdated { scope: scope.clone(), generation, update }),
    )
}

pub(in crate::app) async fn run(
    client: GatewayClient,
    connection: Connection,
    target: DeviceTarget,
    mut cancelled: oneshot::Receiver<()>,
    mut updates: mpsc::Sender<Update>,
) {
    if !client.endpoint().is_loopback() || !connection.valid() {
        let _ = updates.send(Update::Failed("授权中心需要有效的本机回环管理连接")).await;
        return;
    }
    if cancelled.try_recv() != Err(oneshot::error::TryRecvError::Empty) {
        return;
    }
    let service = target.service();
    let request = target.request();
    let host = connection.host.trim();
    let credentials = AgentAppsAuthCredentials {
        username: connection.username.trim(),
        password: &connection.password,
        principal_token: connection.token.trim(),
    };
    // Wait for the start result even after the modal closes so a late flow ID can be cancelled.
    let AuthStartResponse::Pending {
        flow_id,
        expires_at,
        mut retry_after_seconds,
        user_code,
        verification_url,
        ..
    } = match client.agent_apps_auth_start(host, service, &credentials, &request).await {
        Ok(response) => response,
        Err(error) => {
            let _ = updates.send(Update::Failed(error_label(error))).await;
            return;
        }
    };

    if let Some(challenge) = challenge(user_code, &verification_url, request.region, expires_at) {
        let _ = updates.send(Update::Started(challenge)).await;
        loop {
            let result = tokio::select! {
                biased;
                _ = &mut cancelled => break,
                result = async {
                    tokio::time::sleep(Duration::from_secs(retry_after_seconds.max(1))).await;
                    client.agent_apps_auth_poll(host, service, &credentials, &flow_id).await
                } => result,
            };
            match result {
                Ok(AuthFlow::Pending { flow_id: id, expires_at, retry_after_seconds: after })
                    if id == flow_id =>
                {
                    retry_after_seconds = after;
                    let _ = updates.send(Update::Pending(expires_at)).await;
                }
                Ok(snapshot) if matches_flow(&snapshot, &flow_id) => {
                    let update = match snapshot {
                        AuthFlow::Approved { account, .. } if account.service == service => {
                            Update::Approved
                        }
                        AuthFlow::Failed { error, .. } => Update::Failed(code_label(error.code)),
                        AuthFlow::Expired { .. } => Update::Failed("设备码已到期，请重试"),
                        AuthFlow::Cancelled { .. } => Update::Failed("设备码登录已取消"),
                        _ => Update::Failed("网关返回了无效的设备授权状态"),
                    };
                    let _ = updates.send(update).await;
                    return;
                }
                Ok(_) => {
                    let _ = updates.send(Update::Failed("网关返回了其他流程的授权状态")).await;
                    break;
                }
                Err(error) => {
                    let _ = updates.send(Update::Failed(error_label(error))).await;
                    break;
                }
            }
        }
    } else {
        let _ = updates.send(Update::Failed("网关返回了无效的设备码或验证地址")).await;
    }

    if client.agent_apps_auth_cancel(host, service, &credentials, &flow_id).await.is_err() {
        let _ = updates.send(Update::CancelFailed).await;
    }
}

fn challenge(
    user_code: Option<String>,
    verification_url: &str,
    region: Option<vw_gateway_client::vw_api_types::agent_apps_auth::AuthRegion>,
    expires_at: TimestampMs,
) -> Option<DeviceChallenge> {
    let user_code =
        user_code.filter(|code| !code.trim().is_empty() && !code.chars().any(char::is_control))?;
    let verification_url = reqwest::Url::parse(verification_url).ok()?;
    if verification_url.scheme() != "https"
        || !verification_url.has_host()
        || !verification_url.username().is_empty()
        || verification_url.password().is_some()
    {
        return None;
    }
    Some(DeviceChallenge { user_code, verification_url, region, expires_at })
}

fn matches_flow(snapshot: &AuthFlow, expected: &AuthFlowId) -> bool {
    let id = match snapshot {
        AuthFlow::Pending { flow_id, .. }
        | AuthFlow::Approved { flow_id, .. }
        | AuthFlow::Failed { flow_id, .. }
        | AuthFlow::Expired { flow_id, .. }
        | AuthFlow::Cancelled { flow_id, .. } => flow_id,
    };
    id == expected
}

fn error_label(error: AgentAppsAuthError) -> &'static str {
    match error {
        AgentAppsAuthError::Request(code) | AgentAppsAuthError::Http { code: Some(code), .. } => {
            code_label(code)
        }
        AgentAppsAuthError::Http { status: 401 | 403, .. } => "管理认证被拒绝，请检查连接凭据",
        AgentAppsAuthError::InvalidPrincipalToken => "主体令牌格式无效",
        AgentAppsAuthError::Transport => "无法连接管理网关，请重试",
        AgentAppsAuthError::Decode => "网关返回了无效的设备授权响应",
        AgentAppsAuthError::Http { .. } => "网关拒绝了设备码登录",
    }
}

fn code_label(code: AuthErrorCode) -> &'static str {
    use AuthErrorCode::*;
    match code {
        InvalidInput => "设备码登录参数无效",
        Unauthenticated => "管理凭据或主体令牌无效",
        Forbidden => "当前主体没有此服务的设备登录权限",
        Unsupported => "执行主机不支持此服务的设备码登录",
        ExecutionChannelUnavailable => "执行主机没有可用的授权通道",
        ExecutionHostUnavailable => "授权执行主机不可用",
        _ => "设备码登录失败，请重试",
    }
}
