use super::lifecycle::LifecycleState;
use vw_api_app_identity::{
    AppId, AppInstallationEnvironment, AppInstallationId, AppInstallationSource,
};
use vw_api_app_operations::{
    AppActionPreview, AppActionPreviewId, AppActionPreviewRequest, AppActionSubmitRequest,
    AppActionTarget, AppLifecycleAction, AppOperationPhase, AppOperationStatus,
};
use {vw_api_task::TaskStatus, vw_api_transport::id::TaskId};
fn status(id: &str, phase: AppOperationPhase, status: TaskStatus) -> AppOperationStatus {
    AppOperationStatus { operation_id: TaskId(id.into()), status, phase }
}

#[test]
fn cc_apps_desktop_lifecycle_state_uncertain_submit_blocks_retry_and_invalidates_generation() {
    let mut state = LifecycleState { uncertain: true, generation: 7, ..Default::default() };
    assert!(state.active());
    state.invalidate();
    assert_eq!(state.generation, 8);
    assert!(!state.active());
}

#[tokio::test]
async fn cc_apps_desktop_lifecycle_state_authorized_preview_submit_disconnect_recover_cancel() {
    use crate::system_settings_apps::lifecycle_update::{LifecycleResult, Work, execute};
    use serde_json::json;
    use std::io::{Read, Write};
    use {gateway_endpoint::GatewayEndpoint, vw_gateway_client::GatewayClient};
    let request = AppActionPreviewRequest {
        app_id: AppId::Codex,
        action: AppLifecycleAction::Update,
        target: AppActionTarget::Installation {
            installation_id: AppInstallationId("selected-second-instance".into()),
        },
    };
    let responses = vec![
        (
            200,
            json!({"previewId":"review","request":request,"source":"system","confirmationRequired":true,"summary":"隔离目标","displayCommand":null}),
        ),
        (202, json!({"operationId":"same","status":"queued"})),
        (503, json!({"code":"disconnected"})),
        (200, json!({"operationId":"same","status":"running","phase":"executing"})),
        (200, json!({"operationId":"same","status":"cancelled","phase":"terminal"})),
        (422, json!({"code":"node_insufficient"})),
        (503, json!({"code":"submission_unknown"})),
    ];
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let server = std::thread::spawn(move || {
        let mut recorded = vec![];
        for (status, body) in responses {
            let (mut stream, _) = listener.accept().unwrap();
            stream.set_read_timeout(Some(std::time::Duration::from_secs(5))).unwrap();
            let mut header = vec![];
            while !header.ends_with(b"\r\n\r\n") {
                let mut b = [0];
                stream.read_exact(&mut b).unwrap();
                header.push(b[0]);
            }
            let header = String::from_utf8(header).unwrap();
            assert!(header.contains("x-agent-apps-token: fixture-grant"));
            assert!(header.contains("targetHostId=remote"));
            let length = header
                .lines()
                .find_map(|l| {
                    l.to_lowercase()
                        .strip_prefix("content-length: ")
                        .and_then(|s| s.parse::<usize>().ok())
                })
                .unwrap_or(0);
            let mut bytes = vec![0; length];
            stream.read_exact(&mut bytes).unwrap();
            recorded.push((
                header.lines().next().unwrap().to_owned(),
                if bytes.is_empty() {
                    serde_json::Value::Null
                } else {
                    serde_json::from_slice(&bytes).unwrap()
                },
            ));
            let body = body.to_string();
            write!(stream,"HTTP/1.1 {status} Result\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).unwrap();
        }
        recorded
    });
    let client = GatewayClient::new(GatewayEndpoint::new("127.0.0.1", port)).unwrap();
    let run = |work| execute(client.clone(), "remote".into(), "fixture-grant".into(), work);
    let LifecycleResult::Preview(preview) = run(Work::Preview(request.clone())).await.unwrap()
    else {
        panic!("preview required")
    };
    assert_eq!(preview.request.target, request.target);
    let LifecycleResult::Status(accepted) = run(Work::Submit(AppActionSubmitRequest {
        preview_id: preview.preview_id,
        confirmed: true,
    }))
    .await
    .unwrap() else {
        panic!("status required")
    };
    let id = accepted.operation_id.clone();
    let mut state = LifecycleState::default();
    state.accept_status(accepted);
    let failure = run(Work::Poll(id.clone())).await.unwrap_err();
    assert!(!failure.uncertain);
    assert_eq!(state.operation.as_ref().unwrap().operation_id, id);
    let LifecycleResult::Status(recovered) = run(Work::Read(id.clone())).await.unwrap() else {
        panic!("status required")
    };
    assert!(state.accept_status(recovered));
    let LifecycleResult::Status(cancelled) = run(Work::Cancel(id)).await.unwrap() else {
        panic!("status required")
    };
    assert!(state.accept_status(cancelled));
    assert!(!state.active());
    let failure = run(Work::Preview(request)).await.unwrap_err();
    assert!(failure.label.contains("Node") && !failure.uncertain);
    let unknown = run(Work::Submit(AppActionSubmitRequest {
        preview_id: AppActionPreviewId("uncertain".into()),
        confirmed: true,
    }))
    .await
    .unwrap_err();
    assert!(unknown.uncertain);
    assert!(unknown.label.contains("禁止重复提交"));
    let recorded = server.join().unwrap();
    assert_eq!(recorded[0].1["target"]["installationId"], "selected-second-instance");
    assert_eq!(recorded[1].1, json!({"previewId":"review","confirmed":true}));
    assert!(recorded[2].0.starts_with("GET /v1/agent-apps/operations/same/events?"));
    assert!(recorded[3].0.starts_with("GET /v1/agent-apps/operations/same?"));
    assert!(recorded[4].0.starts_with("DELETE /v1/agent-apps/operations/same?"));
}

use super::{lifecycle_app, route};
fn install_request() -> AppActionPreviewRequest {
    AppActionPreviewRequest {
        app_id: AppId::Codex,
        action: AppLifecycleAction::Install,
        target: AppActionTarget::Environment { environment: AppInstallationEnvironment::Macos },
    }
}
#[test]
fn cc_apps_desktop_lifecycle_state_message_confirmation_scope_cancel_and_toast() {
    use crate::system_settings_apps::lifecycle_update::{
        LifecycleFailure, LifecycleMessage as M, LifecycleResult as R,
    };
    let (mut app, context) = lifecycle_app();
    route(&mut app, M::Confirm);
    assert!(!app.lifecycle.busy);
    let request = install_request();
    route(&mut app, M::Preview(request.clone()));
    let generation = app.lifecycle.generation;
    assert!(app.lifecycle.busy);
    let preview = AppActionPreview {
        preview_id: AppActionPreviewId("review".into()),
        request,
        source: AppInstallationSource::System,
        confirmation_required: true,
        summary: "确认".into(),
        display_command: None,
    };
    route(
        &mut app,
        M::Finished {
            context: context.clone(),
            host: "other-host".into(),
            generation,
            result: Ok(R::Preview(preview.clone())),
        },
    );
    assert!(app.lifecycle.preview.is_none());
    route(
        &mut app,
        M::Finished {
            context: context.clone(),
            host: "fixture-host".into(),
            generation,
            result: Ok(R::Preview(preview)),
        },
    );
    assert!(!app.lifecycle.busy && app.lifecycle.preview.is_some());
    route(&mut app, M::Confirm);
    let generation = app.lifecycle.generation;
    route(&mut app, M::Confirm);
    assert_eq!(app.lifecycle.generation, generation);
    route(
        &mut app,
        M::Finished {
            context: context.clone(),
            host: "fixture-host".into(),
            generation,
            result: Ok(R::Status(status("same", AppOperationPhase::Queued, TaskStatus::Queued))),
        },
    );
    let polling_generation = app.lifecycle.generation;
    route(&mut app, M::Cancel);
    let cancelling_generation = app.lifecycle.generation;
    assert_ne!(polling_generation, cancelling_generation);
    route(
        &mut app,
        M::Finished {
            context: context.clone(),
            host: "fixture-host".into(),
            generation: polling_generation,
            result: Err(LifecycleFailure { uncertain: false, label: "过期错误".into() }),
        },
    );
    assert!(app.lifecycle.error.is_none());
    route(
        &mut app,
        M::Finished {
            context: context.clone(),
            host: "fixture-host".into(),
            generation: cancelling_generation,
            result: Ok(R::Status(status(
                "same",
                AppOperationPhase::Terminal,
                TaskStatus::Cancelled,
            ))),
        },
    );
    assert!(!app.lifecycle.active());
    route(&mut app, M::Preview(install_request()));
    let generation = app.lifecycle.generation;
    route(
        &mut app,
        M::Finished {
            context,
            host: "fixture-host".into(),
            generation,
            result: Err(LifecycleFailure {
                uncertain: false,
                label: "Node 版本不足，预检拒绝".into(),
            }),
        },
    );
    assert!(app.lifecycle.preview.is_none());
    assert!(
        matches!(app.notice.as_ref(), Some(crate::aggregation::Notice::Error(message)) if message == "Node 版本不足，预检拒绝")
    );
    route(&mut app, M::Confirm);
    assert!(!app.lifecycle.busy);
}
#[test]
fn cc_apps_desktop_lifecycle_state_message_unknown_submit_cannot_replay() {
    use crate::system_settings_apps::lifecycle_update::{LifecycleFailure, LifecycleMessage as M};
    let (mut app, context) = lifecycle_app();
    app.lifecycle.busy = true;
    app.lifecycle.generation = 9;
    route(
        &mut app,
        M::Finished {
            context,
            host: "fixture-host".into(),
            generation: 9,
            result: Err(LifecycleFailure { uncertain: true, label: "提交结果未知".into() }),
        },
    );
    assert!(app.lifecycle.uncertain);
    route(&mut app, M::Confirm);
    route(&mut app, M::Preview(install_request()));
    assert_eq!(app.lifecycle.generation, 9);
    assert!(app.lifecycle.operation.is_none());
}
