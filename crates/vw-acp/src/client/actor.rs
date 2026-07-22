//! ACP 客户端后台 actor 运行时。
//!
//! 该模块把代理进程、ACP 连接和会话请求串行化到独立线程中，避免上层
//! `AcpClient` 在多个异步调用之间直接共享非线程安全的 ACP 连接状态。

use super::*;

impl AcpClient {
    /// 在专用线程内启动单线程 Tokio runtime 并运行 actor 循环。
    ///
    /// `startup_tx` 用于把 runtime 初始化结果传回调用方；初始化失败时会清理
    /// actor 状态，避免后续请求误以为后台线程可用。
    pub(super) fn run_actor_thread(
        self,
        command_rx: mpsc::UnboundedReceiver<ActorCommand>,
        startup_tx: oneshot::Sender<Result<(), AcpError>>,
    ) {
        let runtime = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
            Ok(runtime) => runtime,
            Err(err) => {
                let _ = startup_tx.send(Err(AcpError::Initialize(err.to_string())));
                self.invalidate_actor();
                return;
            }
        };

        runtime.block_on(async move {
            let local_set = tokio::task::LocalSet::new();
            local_set
                .run_until(async move {
                    let _ = startup_tx.send(Ok(()));
                    self.actor_loop(command_rx).await;
                })
                .await;
        });
    }

    async fn actor_loop(&self, mut command_rx: mpsc::UnboundedReceiver<ActorCommand>) {
        let mut runtime = None::<ActorRuntime>;
        loop {
            let command = if runtime.is_some() {
                tokio::select! {
                    maybe_command = command_rx.recv() => maybe_command,
                    _ = tokio::time::sleep(self.actor_idle_timeout) => {
                        if let Some(active_runtime) = runtime.take() {
                            self.shutdown_actor_runtime(active_runtime, Some("idle_timeout"), false)
                                .await;
                        }
                        continue;
                    }
                    _ = async {
                        if let Some(active_runtime) = runtime.as_mut() {
                            let _ = (&mut active_runtime.io_closed_rx).await;
                        }
                    } => {
                        if runtime.as_ref().is_some_and(|active_runtime| active_runtime.io_task.is_finished())
                            && let Some(active_runtime) = runtime.take()
                        {
                            self.shutdown_actor_runtime(active_runtime, Some("io_closed"), false)
                                .await;
                        }
                        continue;
                    }
                }
            } else if Arc::strong_count(&self.actor_state) <= 1 {
                match command_rx.try_recv() {
                    Ok(command) => Some(command),
                    Err(mpsc::error::TryRecvError::Empty)
                    | Err(mpsc::error::TryRecvError::Disconnected) => {
                        self.store_reusable_session(None);
                        break;
                    }
                }
            } else {
                command_rx.recv().await
            };
            let Some(command) = command else {
                break;
            };
            match command {
                ActorCommand::CreateSession { cwd, response_tx } => {
                    let _ = response_tx.send(self.actor_create_session(&mut runtime, cwd).await);
                }
                ActorCommand::LoadSession { session_id, cwd, response_tx } => {
                    let _ = response_tx
                        .send(self.actor_load_session(&mut runtime, session_id, cwd).await);
                }
                ActorCommand::ResumeSession { session_id, cwd, response_tx } => {
                    let _ = response_tx
                        .send(self.actor_resume_session(&mut runtime, session_id, cwd).await);
                }
                ActorCommand::SetSessionMode { session_id, cwd, mode_id, response_tx } => {
                    let _ = response_tx.send(
                        self.actor_set_session_mode(&mut runtime, session_id, cwd, mode_id).await,
                    );
                }
                ActorCommand::SetSessionConfigOption {
                    session_id,
                    cwd,
                    option_name,
                    value_id,
                    response_tx,
                } => {
                    let _ = response_tx.send(
                        self.actor_set_session_config_option(
                            &mut runtime,
                            session_id,
                            cwd,
                            option_name,
                            value_id,
                        )
                        .await,
                    );
                }
                ActorCommand::SetSessionModel { session_id, cwd, model, response_tx } => {
                    let _ = response_tx.send(
                        self.actor_set_session_model(&mut runtime, session_id, cwd, model).await,
                    );
                }
                ActorCommand::RunPrompt { request, event_tx, response_tx } => {
                    let _ = response_tx
                        .send(self.actor_run_prompt(&mut runtime, request, event_tx).await);
                }
                ActorCommand::Close { response_tx } => {
                    if let Some(active_runtime) = runtime.take() {
                        self.shutdown_actor_runtime(active_runtime, Some("client_close"), false)
                            .await;
                    } else {
                        self.store_reusable_session(None);
                    }
                    let _ = response_tx.send(());
                    return;
                }
            }
        }

        if let Some(active_runtime) = runtime.take() {
            self.shutdown_actor_runtime(active_runtime, Some("actor_channel_closed"), false).await;
        } else {
            self.store_reusable_session(None);
        }
    }

    async fn actor_run_prompt(
        &self,
        runtime: &mut Option<ActorRuntime>,
        request: PromptRequest,
        event_tx: mpsc::UnboundedSender<PromptEvent>,
    ) -> Result<PromptResult, AcpError> {
        *self.permission_stats.lock() = PermissionStats::default();
        self.prepare_actor_runtime(runtime, request.cwd.clone()).await?;
        let runtime = runtime
            .as_mut()
            .ok_or_else(|| AcpError::Initialize("ACP actor runtime is missing".to_string()))?;

        self.run_actor_prompt(runtime, request, event_tx).await
    }
}

#[cfg(test)]
#[path = "actor_tests.rs"]
mod actor_tests;
