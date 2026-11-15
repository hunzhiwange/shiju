//! ACP actor 句柄生命周期和请求通道。

use super::*;

impl AcpClient {
    /// 启动后台 actor 线程。
    ///
    /// 如果 actor 已在运行则直接返回 `Ok(())`。启动失败会返回初始化或线程创建
    /// 相关错误，并清理已记录的 actor 状态。
    pub async fn start(&self) -> Result<(), AcpError> {
        {
            let state = self.actor_state.lock();
            if let Some(handle) = state.handle.as_ref()
                && !handle.command_tx.is_closed()
            {
                return Ok(());
            }
        }

        let (command_tx, command_rx) = mpsc::unbounded_channel();
        let (startup_tx, startup_rx) = oneshot::channel();
        let actor_client = self.clone();
        let thread = thread::Builder::new()
            .name(format!("vw-acp-{}", self.agent_name))
            .spawn(move || actor_client.run_actor_thread(command_rx, startup_tx))
            .map_err(AcpError::Spawn)?;

        {
            let mut state = self.actor_state.lock();
            state.handle = Some(AcpClientActorHandle { command_tx, thread: Some(thread) });
            state.reusable_session_id = None;
        }

        match startup_rx.await {
            Ok(Ok(())) => Ok(()),
            Ok(Err(err)) => {
                self.invalidate_actor();
                Err(err)
            }
            Err(err) => {
                self.invalidate_actor();
                Err(AcpError::Initialize(err.to_string()))
            }
        }
    }

    /// 关闭后台 actor 和当前代理进程。
    pub async fn close(&self) -> Result<(), AcpError> {
        let mut handle = {
            let mut state = self.actor_state.lock();
            state.reusable_session_id = None;
            state.handle.take()
        };
        let Some(mut handle) = handle.take() else {
            return Ok(());
        };

        let (response_tx, response_rx) = oneshot::channel();
        if handle.command_tx.send(ActorCommand::Close { response_tx }).is_ok() {
            let _ = response_rx.await;
        }

        if let Some(thread) = handle.thread.take() {
            let _ = tokio::task::spawn_blocking(move || thread.join()).await;
        }

        Ok(())
    }

    pub(super) fn actor_command_tx(&self) -> Result<mpsc::UnboundedSender<ActorCommand>, AcpError> {
        self.actor_state
            .lock()
            .handle
            .as_ref()
            .filter(|handle| !handle.command_tx.is_closed())
            .map(|handle| handle.command_tx.clone())
            .ok_or_else(|| AcpError::Initialize("ACP client actor is not running".to_string()))
    }

    pub(super) async fn send_actor_request<T, F>(&self, build: F) -> Result<T, AcpError>
    where
        T: Send + 'static,
        F: FnOnce(oneshot::Sender<Result<T, AcpError>>) -> ActorCommand,
    {
        self.start().await?;
        let command_tx = self.actor_command_tx()?;
        let (response_tx, response_rx) = oneshot::channel();
        command_tx.send(build(response_tx)).map_err(|_| {
            self.invalidate_actor();
            AcpError::Initialize("ACP client actor is unavailable".to_string())
        })?;
        response_rx.await.map_err(|err| {
            self.invalidate_actor();
            AcpError::PromptJoin(err.to_string())
        })?
    }
}
