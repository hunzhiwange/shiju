//! Keep form replies independent from the busy chat stream and its MCP peer mutex.
use crate::app::{
    Message,
    message::mcp_form::{FormScope, McpFormMessage},
};
use iced::futures::SinkExt;
use std::sync::Arc;
use vw_gateway_client::GatewayClient;

#[derive(Debug)]
pub(crate) struct FormRun {
    _closed: tokio::sync::oneshot::Sender<()>,
}
pub(crate) async fn watch(
    client: Arc<GatewayClient>,
    run_id: u64,
    session: String,
    workspace: Option<String>,
    mut output: iced::futures::channel::mpsc::Sender<Message>,
) -> Result<FormRun, String> {
    let opened = client.mcp_form_open(&session, workspace).await?;
    let scope = FormScope { run_id, session, host_id: opened.host_id, client: client.clone() };
    let (closed, mut receiver) = tokio::sync::oneshot::channel();
    tokio::spawn(async move {
        loop {
            let result = tokio::select! {
                _ = &mut receiver => break,
                result = client.mcp_form_poll(&scope.host_id) => result,
            };
            match result {
                Ok(requests) => {
                    if output
                        .send(Message::McpForm(McpFormMessage::Pending {
                            scope: scope.clone(),
                            requests,
                        }))
                        .await
                        .is_err()
                    {
                        break;
                    }
                }
                Err(_) => {
                    let _ = output
                        .send(Message::McpForm(McpFormMessage::Unavailable {
                            run_id,
                            session: scope.session.clone(),
                        }))
                        .await;
                    break;
                }
            }
            tokio::select! {
                _ = &mut receiver => break,
                _ = tokio::time::sleep(std::time::Duration::from_secs(1)) => {},
            }
        }
        let _ = client.mcp_form_close(&scope.host_id).await;
        let _ = output.send(Message::McpForm(McpFormMessage::Closed(scope))).await;
    });
    Ok(FormRun { _closed: closed })
}
