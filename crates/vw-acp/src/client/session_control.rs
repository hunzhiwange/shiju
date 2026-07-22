//! ACP actor 会话控制命令。
//!
//! 本模块承载 actor 收到的 session/new、load、resume 和 session/set_* 控制命令。
//! actor 主循环只负责分发，具体协议调用和本地状态更新放在这里。

use super::*;

impl AcpClient {
    pub(super) async fn actor_create_session(
        &self,
        runtime: &mut Option<ActorRuntime>,
        cwd: PathBuf,
    ) -> Result<SessionInfo, AcpError> {
        self.prepare_actor_runtime(runtime, cwd.clone()).await?;
        let mut active_runtime = runtime
            .take()
            .ok_or_else(|| AcpError::Initialize("ACP actor runtime is missing".to_string()))?;
        match self.new_session_info(&active_runtime.conn, &cwd).await {
            Ok(session_info) => {
                let session_id = session_info.session_id.clone();
                *active_runtime.expected_session_id.lock() = Some(session_id.clone());
                tokio::task::yield_now().await;
                if let Some(reason) = self.actor_runtime_restart_reason(&mut active_runtime, &cwd) {
                    self.shutdown_actor_runtime(active_runtime, Some(reason), false).await;
                } else {
                    self.store_reusable_session(Some(session_id.clone()));
                    *runtime = Some(active_runtime);
                }
                Ok(session_info)
            }
            Err(err) => {
                let finalized = self
                    .shutdown_actor_runtime(active_runtime, Some("create_session_failed"), false)
                    .await;
                Err(enrich_acp_error_with_process_context(err, &finalized))
            }
        }
    }

    pub(super) async fn actor_load_session(
        &self,
        runtime: &mut Option<ActorRuntime>,
        session_id: String,
        cwd: PathBuf,
    ) -> Result<SessionInfo, AcpError> {
        self.prepare_actor_runtime(runtime, cwd.clone()).await?;
        let runtime = runtime
            .as_mut()
            .ok_or_else(|| AcpError::Initialize("ACP actor runtime is missing".to_string()))?;
        let session_info = self.load_session_info(&runtime.conn, &cwd, session_id.clone()).await?;
        *runtime.expected_session_id.lock() = Some(session_id.clone());
        self.store_reusable_session(Some(session_id.clone()));
        Ok(session_info)
    }

    pub(super) async fn actor_resume_session(
        &self,
        runtime: &mut Option<ActorRuntime>,
        session_id: String,
        cwd: PathBuf,
    ) -> Result<SessionInfo, AcpError> {
        self.prepare_actor_runtime(runtime, cwd.clone()).await?;
        let runtime = runtime
            .as_mut()
            .ok_or_else(|| AcpError::Initialize("ACP actor runtime is missing".to_string()))?;
        let session_info =
            self.resume_session_info(&runtime.conn, &cwd, session_id.clone()).await?;
        *runtime.expected_session_id.lock() = Some(session_id.clone());
        self.store_reusable_session(Some(session_id.clone()));
        Ok(session_info)
    }

    pub(super) async fn actor_set_session_mode(
        &self,
        runtime: &mut Option<ActorRuntime>,
        session_id: String,
        cwd: PathBuf,
        mode_id: String,
    ) -> Result<(), AcpError> {
        self.prepare_actor_runtime(runtime, cwd.clone()).await?;
        let runtime = runtime
            .as_mut()
            .ok_or_else(|| AcpError::Initialize("ACP actor runtime is missing".to_string()))?;
        self.resolve_existing_session(
            &runtime.conn,
            &cwd,
            session_id.clone(),
            &runtime.expected_session_id,
        )
        .await?;
        runtime
            .conn
            .set_session_mode(acp::SetSessionModeRequest::new(
                acp::SessionId::new(session_id.clone()),
                mode_id,
            ))
            .await
            .map_err(|err| AcpError::Prompt(err.to_string()))?;
        self.store_reusable_session(Some(session_id));
        Ok(())
    }

    pub(super) async fn actor_set_session_config_option(
        &self,
        runtime: &mut Option<ActorRuntime>,
        session_id: String,
        cwd: PathBuf,
        option_name: String,
        value_id: String,
    ) -> Result<acp::SetSessionConfigOptionResponse, AcpError> {
        self.prepare_actor_runtime(runtime, cwd.clone()).await?;
        let runtime = runtime
            .as_mut()
            .ok_or_else(|| AcpError::Initialize("ACP actor runtime is missing".to_string()))?;
        self.resolve_existing_session(
            &runtime.conn,
            &cwd,
            session_id.clone(),
            &runtime.expected_session_id,
        )
        .await?;
        let error_context = format!(r#"for "{}"="{}""#, option_name, value_id);
        let response = runtime
            .conn
            .set_session_config_option(acp::SetSessionConfigOptionRequest::new(
                acp::SessionId::new(session_id.clone()),
                option_name,
                value_id,
            ))
            .await
            .map_err(|err| {
                wrap_session_control_error(
                    "session/set_config_option",
                    Some(error_context),
                    err,
                    AcpError::SetSessionConfigOption,
                )
            })?;
        self.store_reusable_session(Some(session_id));
        Ok(response)
    }

    pub(super) async fn actor_set_session_model(
        &self,
        runtime: &mut Option<ActorRuntime>,
        session_id: String,
        cwd: PathBuf,
        model: String,
    ) -> Result<(), AcpError> {
        self.prepare_actor_runtime(runtime, cwd.clone()).await?;
        let runtime = runtime
            .as_mut()
            .ok_or_else(|| AcpError::Initialize("ACP actor runtime is missing".to_string()))?;
        self.resolve_existing_session(
            &runtime.conn,
            &cwd,
            session_id.clone(),
            &runtime.expected_session_id,
        )
        .await?;
        let error_context = format!(r#"for model "{}""#, model);
        runtime
            .conn
            .set_session_model(acp::SetSessionModelRequest::new(
                acp::SessionId::new(session_id.clone()),
                model,
            ))
            .await
            .map_err(|err| {
                wrap_session_control_error(
                    "session/set_model",
                    Some(error_context),
                    err,
                    AcpError::SetSessionModel,
                )
            })?;
        self.store_reusable_session(Some(session_id));
        Ok(())
    }
}
