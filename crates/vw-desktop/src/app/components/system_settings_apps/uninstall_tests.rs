use super::lifecycle::LifecycleState;

use vw_gateway_client::vw_api_types::agent_apps::*;
fn lifecycle_app() -> (crate::app::App, crate::app::state::AgentAppsContext) {
    let mut app = crate::app::App::new().0;
    let client = crate::app::config::gateway_client().unwrap();
    let context = crate::app::state::AgentAppsContext {
        gateway: client.endpoint().base_url(),
        tenant: client.endpoint().tenant_scope().clone(),
        project: app.project_path.clone(),
    };
    app.apps_catalog.context = Some(context.clone());
    app.apps_catalog.host = "fixture-host".into();
    app.apps_catalog.token = "fixture-grant".into();
    (app, context)
}
fn route(
    app: &mut crate::app::App,
    message: crate::app::message::settings::apps_lifecycle::LifecycleMessage,
) {
    let _ = crate::app::message::settings::update(
        app,
        crate::app::message::SettingsMessage::AppsCatalog(
            crate::app::message::settings::AppsCatalogMessage::Lifecycle(message),
        ),
    );
}

#[test]
fn cc_apps_desktop_uninstall_state_selected_instance_requires_new_preview_after_rejection() {
    use super::catalog::CatalogRow;
    use crate::app::message::settings::apps_lifecycle::{
        LifecycleFailure, LifecycleMessage as M, LifecycleResult as R,
    };
    let (mut app, context) = lifecycle_app();
    let installation = |id: &str| AppInstallation {
        id: AppInstallationId(id.into()),
        app_id: AppId::GrokBuild,
        target_host_id: "fixture-host".into(),
        environment: AppInstallationEnvironment::Macos,
        path: format!("/隔离/{id}/grok"),
        real_path: Some(format!("/真实/{id}/grok")),
        source: AppInstallationSource::System,
        is_path_default: false,
        version: None,
        runtime_status: AppInstallationRuntimeStatus::Runnable,
    };
    app.apps_catalog.rows = vec![CatalogRow {
        entry: AppCatalogEntry {
            id: AppId::GrokBuild,
            name: "Grok".into(),
            cli_command: Some("grok".into()),
            runtime_kinds: vec![AppRuntimeKind::NativeCli],
        },
        observation: Ok(AppReadResponse {
            desktop: None,
            installations: vec![installation("first"), installation("second")],
        }),
    }];
    let request = AppActionPreviewRequest {
        app_id: AppId::GrokBuild,
        action: AppLifecycleAction::Uninstall,
        target: AppActionTarget::Installation {
            installation_id: AppInstallationId("second".into()),
        },
    };
    route(&mut app, M::Confirm);
    assert!(!app.apps_catalog.lifecycle.busy);
    route(&mut app, M::Preview(request.clone()));
    assert!(app.apps_catalog.lifecycle.busy, "selected uninstall must request backend preview");
    assert_eq!(app.apps_catalog.lifecycle.request.as_ref(), Some(&request));
    assert!(app.apps_catalog.lifecycle.target_label.contains("/真实/second/grok"));
    let generation = app.apps_catalog.lifecycle.generation;
    route(
        &mut app,
        M::Finished {
            context: context.clone(),
            host: "fixture-host".into(),
            generation,
            result: Ok(R::Preview(AppActionPreview {
                preview_id: AppActionPreviewId("review".into()),
                request: request.clone(),
                source: AppInstallationSource::System,
                confirmation_required: true,
                summary: "选定副本".into(),
                display_command: None,
            })),
        },
    );
    route(&mut app, M::Confirm);
    assert!(app.apps_catalog.lifecycle.preview.is_none());
    let generation = app.apps_catalog.lifecycle.generation;
    route(
        &mut app,
        M::Finished {
            context,
            host: "fixture-host".into(),
            generation,
            result: Err(LifecycleFailure {
                uncertain: false,
                label: "计划已变化，请重新预检并确认".into(),
            }),
        },
    );
    route(&mut app, M::Confirm);
    assert!(!app.apps_catalog.lifecycle.busy);
    assert_eq!(app.apps_catalog.lifecycle.generation, generation);
    route(&mut app, M::Preview(request));
    assert!(app.apps_catalog.lifecycle.busy);
}

#[test]
fn cc_apps_desktop_uninstall_state_unknown_missing_and_environment_targets_never_execute() {
    use super::catalog::CatalogRow;
    use crate::app::message::settings::apps_lifecycle::LifecycleMessage as M;
    let (mut app, context) = lifecycle_app();
    let installation = |id: &str| AppInstallation {
        id: AppInstallationId(id.into()),
        app_id: AppId::GrokBuild,
        target_host_id: "fixture-host".into(),
        environment: AppInstallationEnvironment::Macos,
        path: format!("/隔离/{id}/grok"),
        real_path: Some(format!("/真实/{id}/grok")),
        source: AppInstallationSource::System,
        is_path_default: false,
        version: None,
        runtime_status: AppInstallationRuntimeStatus::Runnable,
    };
    app.apps_catalog.rows = vec![CatalogRow {
        entry: AppCatalogEntry {
            id: AppId::GrokBuild,
            name: "Grok".into(),
            cli_command: Some("grok".into()),
            runtime_kinds: vec![AppRuntimeKind::NativeCli],
        },
        observation: Ok(AppReadResponse {
            desktop: None,
            installations: vec![installation("first"), installation("second")],
        }),
    }];

    let _ = context;
    app.apps_catalog.rows[0].observation.as_mut().unwrap().installations[1].source =
        AppInstallationSource::Unknown;
    for target in [
        AppActionTarget::Installation { installation_id: AppInstallationId("second".into()) },
        AppActionTarget::Installation { installation_id: AppInstallationId("absent".into()) },
        AppActionTarget::Environment { environment: AppInstallationEnvironment::Macos },
    ] {
        route(
            &mut app,
            M::Preview(AppActionPreviewRequest {
                app_id: AppId::GrokBuild,
                action: AppLifecycleAction::Uninstall,
                target,
            }),
        );
        route(&mut app, M::Confirm);
        assert!(!app.apps_catalog.lifecycle.busy);
        assert!(app.apps_catalog.lifecycle.request.is_none());
        assert!(app.apps_catalog.lifecycle.operation.is_none());
    }
    assert!(app.active_toast.is_some());
}

#[test]
fn cc_apps_desktop_uninstall_state_mismatched_preview_and_unknown_source_rejected() {
    use crate::app::message::settings::apps_lifecycle::{
        LifecycleMessage as M, LifecycleResult as R,
    };
    let (mut app, context) = lifecycle_app();
    let request = AppActionPreviewRequest {
        app_id: AppId::Hermes,
        action: AppLifecycleAction::Uninstall,
        target: AppActionTarget::Installation {
            installation_id: AppInstallationId("selected".into()),
        },
    };
    app.apps_catalog.lifecycle.request = Some(request.clone());
    for source in [AppInstallationSource::Unknown, AppInstallationSource::System] {
        let mut returned = request.clone();
        if source == AppInstallationSource::System {
            returned.target = AppActionTarget::Installation {
                installation_id: AppInstallationId("other".into()),
            };
        }
        route(
            &mut app,
            M::Finished {
                context: context.clone(),
                host: "fixture-host".into(),
                generation: 0,
                result: Ok(R::Preview(AppActionPreview {
                    preview_id: AppActionPreviewId("bad".into()),
                    request: returned,
                    source,
                    confirmation_required: true,
                    summary: "中文".into(),
                    display_command: None,
                })),
            },
        );
        route(&mut app, M::Confirm);
        assert!(app.apps_catalog.lifecycle.preview.is_none());
        assert!(!app.apps_catalog.lifecycle.busy);
        assert!(app.apps_catalog.lifecycle.operation.is_none());
    }
    assert!(super::uninstall::guidance(AppId::Hermes).contains("保留"));
    assert!(super::uninstall::guidance(AppId::GrokBuild).contains("重建"));
}

#[tokio::test]
async fn cc_apps_desktop_uninstall_state_http_selected_instance_stale_submit_and_no_replay() {
    use crate::app::message::settings::apps_lifecycle::{LifecycleResult, Work, execute};
    use serde_json::json;
    use std::io::{Read, Write};
    use vw_gateway_client::{GatewayClient, GatewayEndpoint};
    let request = AppActionPreviewRequest {
        app_id: AppId::Codex,
        action: AppLifecycleAction::Uninstall,
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
        (409, json!({"code":"preview_changed"})),
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
    let failure = run(Work::Submit(AppActionSubmitRequest {
        preview_id: AppActionPreviewId("expired".into()),
        confirmed: true,
    }))
    .await
    .unwrap_err();
    assert!(failure.label.contains("重新预检") && !failure.uncertain);
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
    assert_eq!(recorded[5].1, json!({"previewId":"expired","confirmed":true}));
    assert_eq!(recorded.len(), 7);
}
