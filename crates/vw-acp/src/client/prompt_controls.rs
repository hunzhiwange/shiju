//! 活动提示期间复用同一协议连接发送会话控制，不阻塞提示和取消。

use super::*;

pub(super) type PromptControlFuture<'a> = Pin<Box<dyn std::future::Future<Output = ()> + 'a>>;

pub(super) fn control_future<'a>(
    connection: &'a acp::ClientSideConnection,
    command: ActorCommand,
    active_session_id: &str,
    active_cwd: &Path,
) -> Result<PromptControlFuture<'a>, ActorCommand> {
    let invalid_target = |id: &str, cwd: &Path| id != active_session_id || cwd != active_cwd;
    match command {
        ActorCommand::SetSessionMode { session_id, cwd, mode_id, response_tx } => {
            let invalid = invalid_target(&session_id, &cwd);
            Ok(Box::pin(async move {
                let result = if invalid {
                    Err(AcpError::Prompt(
                        "cannot switch another session during an active prompt".into(),
                    ))
                } else {
                    connection
                        .set_session_mode(acp::SetSessionModeRequest::new(session_id, mode_id))
                        .await
                        .map(|_| ())
                        .map_err(|error| AcpError::Prompt(error.to_string()))
                };
                let _ = response_tx.send(result);
            }))
        }
        ActorCommand::SetSessionModel { session_id, cwd, model, response_tx } => {
            let invalid = invalid_target(&session_id, &cwd);
            Ok(Box::pin(async move {
                let result = if invalid {
                    Err(AcpError::SetSessionModel(
                        "cannot switch another session during an active prompt".into(),
                    ))
                } else {
                    connection
                        .set_session_model(acp::SetSessionModelRequest::new(session_id, model))
                        .await
                        .map(|_| ())
                        .map_err(|error| AcpError::SetSessionModel(error.to_string()))
                };
                let _ = response_tx.send(result);
            }))
        }
        ActorCommand::SetSessionConfigOption {
            session_id,
            cwd,
            option_name,
            value_id,
            response_tx,
        } => {
            let invalid = invalid_target(&session_id, &cwd);
            Ok(Box::pin(async move {
                let result = if invalid {
                    Err(AcpError::SetSessionConfigOption(
                        "cannot switch another session during an active prompt".into(),
                    ))
                } else {
                    connection
                        .set_session_config_option(acp::SetSessionConfigOptionRequest::new(
                            session_id,
                            option_name,
                            value_id,
                        ))
                        .await
                        .map_err(|error| AcpError::SetSessionConfigOption(error.to_string()))
                };
                let _ = response_tx.send(result);
            }))
        }
        other => Err(other),
    }
}
