//! `AcpClient` 的会话命令门面。
//!
//! 本模块承载公共会话操作；actor 启停和请求通道由 `actor_handle` 模块维护。

use super::*;

impl AcpClient {
    /// 在指定工作目录创建新的 ACP 会话。
    pub async fn create_session(&self, cwd: impl AsRef<Path>) -> Result<SessionInfo, AcpError> {
        let cwd = cwd.as_ref().to_path_buf();
        self.send_actor_request(move |response_tx| ActorCommand::CreateSession { cwd, response_tx })
            .await
    }

    /// 加载一个已存在的 ACP 会话。
    pub async fn load_session(
        &self,
        session_id: impl Into<String>,
        cwd: impl AsRef<Path>,
    ) -> Result<SessionInfo, AcpError> {
        let session_id = session_id.into();
        let cwd = cwd.as_ref().to_path_buf();
        self.send_actor_request(move |response_tx| ActorCommand::LoadSession {
            session_id,
            cwd,
            response_tx,
        })
        .await
    }

    /// 恢复一个已存在的 ACP 会话。
    pub async fn resume_session(
        &self,
        session_id: impl Into<String>,
        cwd: impl AsRef<Path>,
    ) -> Result<SessionInfo, AcpError> {
        let session_id = session_id.into();
        let cwd = cwd.as_ref().to_path_buf();
        self.send_actor_request(move |response_tx| ActorCommand::ResumeSession {
            session_id,
            cwd,
            response_tx,
        })
        .await
    }

    /// 设置已有会话的模式。
    pub async fn set_session_mode(
        &self,
        session_id: impl Into<String>,
        cwd: impl AsRef<Path>,
        mode_id: impl Into<String>,
    ) -> Result<(), AcpError> {
        let session_id = session_id.into();
        let cwd = cwd.as_ref().to_path_buf();
        let mode_id = mode_id.into();
        self.send_actor_request(move |response_tx| ActorCommand::SetSessionMode {
            session_id,
            cwd,
            mode_id,
            response_tx,
        })
        .await
    }

    /// 设置已有会话的配置选项。
    pub async fn set_session_config_option(
        &self,
        session_id: impl Into<String>,
        cwd: impl AsRef<Path>,
        option_name: impl Into<String>,
        value_id: impl Into<String>,
    ) -> Result<acp::SetSessionConfigOptionResponse, AcpError> {
        let session_id = session_id.into();
        let cwd = cwd.as_ref().to_path_buf();
        let option_name = option_name.into();
        let value_id = value_id.into();
        self.send_actor_request(move |response_tx| ActorCommand::SetSessionConfigOption {
            session_id,
            cwd,
            option_name,
            value_id,
            response_tx,
        })
        .await
    }

    /// 设置已有会话的模型。
    pub async fn set_session_model(
        &self,
        session_id: impl Into<String>,
        cwd: impl AsRef<Path>,
        model: impl Into<String>,
    ) -> Result<(), AcpError> {
        let session_id = session_id.into();
        let cwd = cwd.as_ref().to_path_buf();
        let model = model.into();
        self.send_actor_request(move |response_tx| ActorCommand::SetSessionModel {
            session_id,
            cwd,
            model,
            response_tx,
        })
        .await
    }

    /// 在目标会话策略下执行提示词。
    pub async fn run_prompt(
        &self,
        request: PromptRequest,
        on_event: &mut impl FnMut(PromptEvent),
    ) -> Result<PromptResult, AcpError> {
        self.start().await?;
        let command_tx = self.actor_command_tx()?;
        let (event_tx, mut event_rx) = mpsc::unbounded_channel();
        let (response_tx, response_rx) = oneshot::channel();
        let command = ActorCommand::RunPrompt { request, event_tx, response_tx };
        let unavailable = "ACP client actor is unavailable";
        if command_tx.send(command).is_err() {
            self.invalidate_actor();
            return Err(AcpError::Initialize(unavailable.to_string()));
        }

        let mut events_open = true;
        tokio::pin!(response_rx);

        loop {
            tokio::select! {
                result = &mut response_rx => {
                    let result = result.map_err(|err| {
                        self.invalidate_actor();
                        AcpError::PromptJoin(err.to_string())
                    })?;
                    while let Ok(event) = event_rx.try_recv() {
                        on_event(event);
                    }
                    return result;
                }
                maybe_event = event_rx.recv(), if events_open => {
                    match maybe_event {
                        Some(event) => on_event(event),
                        None => events_open = false,
                    }
                }
            }
        }
    }
}
