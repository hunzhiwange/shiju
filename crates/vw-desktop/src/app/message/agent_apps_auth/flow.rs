//! Drive desktop status reads and cancellation through the scoped gateway SDK.

use super::{Event, message};
use crate::app::{
    Message,
    state::agent_apps_auth::{Connection, LoginChallenge, LoginTarget, Scope, State},
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
        agent_apps_auth::{
            AuthErrorCode, AuthFlow, AuthFlowId, AuthPasteRequest, AuthService, AuthStartResponse,
        },
    },
};

#[derive(Clone)]
pub enum Update {
    Started(LoginChallenge),
    Pending(TimestampMs),
    Approved,
    Failed(&'static str),
    PasteFinished(Result<(), &'static str>),
    CancelFailed,
}

pub(super) fn start(
    state: &mut State,
    scope: Scope,
    client: GatewayClient,
    connection: Connection,
    target: LoginTarget,
) -> Task<Message> {
    let copy_link = state.copy_login_link;
    let (generation, cancelled, pastes) = state.begin_login(target);
    Task::run(
        iced::stream::channel(4, async move |updates| {
            run(client, connection, target, copy_link, cancelled, pastes, updates).await;
        }),
        move |update| message(Event::LoginUpdated { scope: scope.clone(), generation, update }),
    )
}

pub(in crate::app) async fn run(
    client: GatewayClient,
    connection: Connection,
    target: LoginTarget,
    copy_link: bool,
    mut cancelled: oneshot::Receiver<()>,
    mut pastes: tokio::sync::mpsc::Receiver<AuthPasteRequest>,
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
    let request = target.request(copy_link);
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
        retry_after_seconds,
        user_code,
        verification_url,
        can_paste,
    } = match client.agent_apps_auth_start(host, service, &credentials, &request).await {
        Ok(response) => response,
        Err(error) => {
            let _ = updates.send(Update::Failed(error_label(error))).await;
            return;
        }
    };

    let can_paste = service == AuthService::ClaudeOfficial && can_paste == Some(true);
    if let Some(challenge) = challenge(target, user_code, &verification_url, expires_at, can_paste)
    {
        let _ = updates.send(Update::Started(challenge)).await;
        let poll_delay = tokio::time::sleep(Duration::from_secs(retry_after_seconds.max(1)));
        tokio::pin!(poll_delay);
        loop {
            tokio::select! {
                biased;
                _ = &mut cancelled => break,
                _ = &mut poll_delay => {}
                input = pastes.recv(), if can_paste => {
                    let Some(input) = input else { break };
                    let result = tokio::select! {
                        biased;
                        _ = &mut cancelled => break,
                        result = client.agent_apps_auth_paste(host, &credentials, &flow_id, &input) => result,
                    };
                    let _ = updates.send(Update::PasteFinished(result.map_err(paste_error_label))).await;
                    continue;
                }
            }
            let result = tokio::select! {
                biased;
                _ = &mut cancelled => break,
                result = client.agent_apps_auth_poll(host, service, &credentials, &flow_id) => result,
            };
            match result {
                Ok(AuthFlow::Pending { flow_id: id, expires_at, retry_after_seconds: after })
                    if id == flow_id =>
                {
                    poll_delay.set(tokio::time::sleep(Duration::from_secs(after.max(1))));
                    let _ = updates.send(Update::Pending(expires_at)).await;
                }
                Ok(snapshot) if matches_flow(&snapshot, &flow_id) => {
                    let update = match snapshot {
                        AuthFlow::Approved { account, .. } if account.service == service => {
                            Update::Approved
                        }
                        AuthFlow::Failed { error, .. } => Update::Failed(code_label(error.code)),
                        AuthFlow::Expired { .. } => Update::Failed("授权已到期，请重试"),
                        AuthFlow::Cancelled { .. } => Update::Failed("授权登录已取消"),
                        _ => Update::Failed("网关返回了无效的授权状态"),
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
        let reason = if target.is_device() {
            "网关返回了无效的设备码或验证地址"
        } else {
            "网关返回了无效的授权链接"
        };
        let _ = updates.send(Update::Failed(reason)).await;
    }

    if client.agent_apps_auth_cancel(host, service, &credentials, &flow_id).await.is_err() {
        let _ = updates.send(Update::CancelFailed).await;
    }
}

fn challenge(
    target: LoginTarget,
    user_code: Option<String>,
    verification_url: &str,
    expires_at: TimestampMs,
    can_paste: bool,
) -> Option<LoginChallenge> {
    if target.is_device() != user_code.is_some()
        || user_code
            .as_ref()
            .is_some_and(|code| code.trim().is_empty() || code.chars().any(char::is_control))
        || verification_url.chars().any(char::is_control)
    {
        return None;
    }
    let verification_url = reqwest::Url::parse(verification_url).ok()?;
    if verification_url.scheme() != "https"
        || !verification_url.has_host()
        || !verification_url.username().is_empty()
        || verification_url.password().is_some()
    {
        return None;
    }
    let region = match target {
        LoginTarget::Kimi(region) => Some(region),
        _ => None,
    };
    Some(LoginChallenge { user_code, verification_url, region, expires_at, can_paste })
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
        AgentAppsAuthError::Decode => "网关返回了无效的授权响应",
        AgentAppsAuthError::Http { .. } => "网关拒绝了授权登录",
    }
}

fn code_label(code: AuthErrorCode) -> &'static str {
    use AuthErrorCode::*;
    match code {
        InvalidInput => "授权登录参数无效",
        Unauthenticated => "管理凭据或主体令牌无效",
        Forbidden => "当前主体没有此服务的登录权限",
        Unsupported => "执行主机不支持此服务的登录方式",
        CallbackUnreachable => "当前浏览器无法访问执行主机的授权回调",
        ExecutionChannelUnavailable => "执行主机没有可用的授权通道",
        ExecutionHostUnavailable => "授权执行主机不可用",
        _ => "授权登录失败，请重试",
    }
}

fn paste_error_label(error: AgentAppsAuthError) -> &'static str {
    match error {
        AgentAppsAuthError::Http { code: Some(AuthErrorCode::InvalidInput), .. } => {
            "执行主机拒绝了回填，请检查授权码或回调地址"
        }
        AgentAppsAuthError::Http {
            code: Some(AuthErrorCode::Conflict | AuthErrorCode::NotFound),
            ..
        } => "回填与当前 Claude 授权不匹配，或流程已结束",
        _ => "无法提交 Claude 授权回填，请重试",
    }
}
