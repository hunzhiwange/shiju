//! 原生审批超时和客户端非法选项都必须结束或拒绝当前权限请求。

use std::cell::Cell;

use super::*;
use crate::server::agent_tests::FakeBackend;

struct PermissionSink {
    started: Rc<Cell<bool>>,
    selected: Option<String>,
}

#[async_trait::async_trait(?Send)]
impl SessionUpdateSink for PermissionSink {
    async fn send(&self, _notification: SessionNotification) -> Result<()> {
        Ok(())
    }

    async fn request_permission(
        &self,
        request: RequestPermissionRequest,
    ) -> Result<agent_client_protocol::RequestPermissionResponse> {
        assert_eq!(request.tool_call.fields.kind, Some(agent_client_protocol::ToolKind::Execute));
        self.started.set(true);
        match &self.selected {
            Some(id) => Ok(agent_client_protocol::RequestPermissionResponse::new(
                RequestPermissionOutcome::Selected(
                    agent_client_protocol::SelectedPermissionOutcome::new(id.clone()),
                ),
            )),
            None => std::future::pending().await,
        }
    }
}

fn permission(response: tokio::sync::oneshot::Sender<Option<String>>) -> BackendPermissionRequest {
    BackendPermissionRequest {
        request_id: "permission-1".into(),
        call_id: "call-1".into(),
        title: "fixture".into(),
        kind: agent_client_protocol::ToolKind::Execute,
        raw_input: serde_json::Value::Null,
        options: vec![super::super::BackendPermissionOption {
            id: "allow_once".into(),
            name: "Allow".into(),
            kind: BackendPermissionOptionKind::AllowOnce,
        }],
        response,
    }
}

#[tokio::test]
async fn permission_wait_ends_when_backend_stops_waiting() {
    let started = Rc::new(Cell::new(false));
    let agent = VibeWindowAgent::with_update_sink(
        FakeBackend::default(),
        PermissionSink { started: started.clone(), selected: None },
    );
    let (response, selected) = tokio::sync::oneshot::channel();
    let request = agent.request_permission("session-1", permission(response));
    let timeout = async {
        while !started.get() {
            tokio::task::yield_now().await;
        }
        drop(selected);
    };
    tokio::time::timeout(std::time::Duration::from_secs(1), async {
        tokio::join!(request, timeout)
    })
    .await
    .expect("stale permission must not block later approvals");
}

#[tokio::test]
async fn permission_selection_rejects_unoffered_option_ids() {
    for (option, expected) in [("allow_once", Some("allow_once".to_string())), ("unknown", None)] {
        let agent = VibeWindowAgent::with_update_sink(
            FakeBackend::default(),
            PermissionSink { started: Rc::new(Cell::new(false)), selected: Some(option.into()) },
        );
        let (response, selected) = tokio::sync::oneshot::channel();
        agent.request_permission("session-1", permission(response)).await;
        assert_eq!(selected.await.unwrap(), expected);
    }
}
