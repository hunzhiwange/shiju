use super::fixture::catalog;
use super::fixture::*;
use iced::{
    Font, Rectangle, Renderer, Size, Theme,
    advanced::{
        renderer::{Headless, Style},
        widget::{Id, Operation},
    },
    mouse,
};
use iced_runtime::user_interface::{Cache, UserInterface};
use vw_api_app_identity::{
    AppCatalogEntry, AppId, AppInstallation, AppInstallationEnvironment, AppInstallationId,
    AppInstallationRuntimeStatus, AppInstallationSource, AppReadResponse, AppRuntimeKind,
};
use vw_api_app_operations::{AppActionExecutionResult, AppOperationPhase, AppOperationStatus};
#[derive(Default)]
struct Labels(Vec<String>);
impl Operation for Labels {
    fn traverse(&mut self, children: &mut dyn FnMut(&mut dyn Operation)) {
        children(self);
    }
    fn text(&mut self, _: Option<&Id>, _: Rectangle, value: &str) {
        self.0.push(value.into());
    }
}
fn draw(element: iced::Element<'_, McpMessage>, theme: Theme) -> Vec<String> {
    let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
    let mut renderer =
        runtime.block_on(Renderer::new(Font::DEFAULT, 14.0.into(), Some("tiny-skia"))).unwrap();
    {
        let mut fonts = iced::advanced::graphics::text::font_system().write().unwrap();
        for font in desktop_assets::fonts::load_all() {
            fonts.load_font(font);
        }
    }
    let mut ui =
        UserInterface::build(element, Size::new(1200., 3000.), Cache::new(), &mut renderer);
    ui.draw(
        &mut renderer,
        &theme,
        &Style { text_color: theme.palette().text },
        mouse::Cursor::Unavailable,
    );
    let mut labels = Labels::default();
    ui.operate(&renderer, &mut labels);
    labels.0
}

use super::fixture::resync::*;

use iced::Task;
use {gateway_mcp_distribution::*, vw_api_app_identity::*, vw_api_mcp_management::*};
fn target() -> McpAppTarget {
    McpAppTarget {
        app_id: AppId::Codex,
        installation_id: AppInstallationId("trusted-instance".into()),
        target_host_id: "host".into(),
    }
}
fn setup() -> Fixture {
    use desktop_settings_apps::system_settings_apps::catalog::CatalogRow;
    let mut app = Fixture::default();
    app.authority.host = "host".into();
    app.authority.token = "fixture".into();
    app.authority.context = Some(catalog::context(&app).unwrap().endpoint);
    app.authority.rows.push(CatalogRow {
        entry: AppCatalogEntry {
            id: AppId::Codex,
            name: "Codex".into(),
            cli_command: Some("codex".into()),
            runtime_kinds: vec![AppRuntimeKind::NativeCli],
        },
        observation: Ok(AppReadResponse {
            desktop: None,
            installations: vec![AppInstallation {
                id: target().installation_id,
                app_id: AppId::Codex,
                target_host_id: "host".into(),
                environment: AppInstallationEnvironment::Macos,
                path: "/fixture/中文路径/codex".into(),
                real_path: None,
                source: AppInstallationSource::Native,
                is_path_default: true,
                version: None,
                runtime_status: AppInstallationRuntimeStatus::Runnable,
            }],
        }),
    });
    app
}
fn run(
    task: Task<McpMessage>,
    runtime: &tokio::runtime::Runtime,
) -> tokio::task::JoinHandle<ResyncMessage> {
    use iced_runtime::{Action, futures::futures::StreamExt};
    runtime.spawn(async move {
        let mut stream = iced_runtime::task::into_stream(task).expect("request task");
        let mut result = None;
        while let Some(action) = stream.next().await {
            if let Action::Output(McpMessage::Resync(event)) = action {
                result = Some(event);
            }
        }
        result.expect("missing resync response")
    })
}
fn request() -> McpResyncRequest {
    McpResyncRequest {
        target_host_id: "host".into(),
        project_id: None,
        targets: vec![McpResyncTarget {
            app_id: AppId::Codex,
            installation_id: target().installation_id,
            account_id: None,
        }],
    }
}
fn status(phase: AppOperationPhase, outcome: Option<McpBatchOutcome>) -> McpResyncOperationStatus {
    // The SDK owns this wire shape; serialize its DTO when serving loopback responses.
    serde_json::from_value(serde_json::json!({
        "operationId":"job /?", "status": if phase == AppOperationPhase::Terminal { "failed" } else { "running" },
        "phase": phase, "outcome": outcome,
        "targets": if outcome.is_some() { serde_json::json!([{"appId":"codex","code":"target_unavailable"}, {"appId":"gemini", "distribution":{"outcome":"succeeded", "items":[]}}]) } else { serde_json::json!([{"appId":"gemini", "distribution":{"outcome":"succeeded", "items":[]}}]) }
    })).unwrap()
}
fn send(responses: &std::sync::mpsc::Sender<Option<String>>, value: &impl serde::Serialize) {
    responses.send(Some(serde_json::to_string(value).unwrap())).unwrap();
}
#[test]
fn cc_mcp_t48_real_sdk_job_polls_cancels_retains_failure_and_retries_only_failed_app() {
    use std::time::Duration;
    super::fixture::with_settings_gateway(|requests, responses| {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .unwrap();
        let mut app = setup();
        let mut gemini = app.authority.rows[0].clone();
        gemini.entry.id = AppId::Gemini;
        let installation = &mut gemini.observation.as_mut().unwrap().installations[0];
        installation.app_id = AppId::Gemini;
        installation.id.0 = "gemini-instance".into();
        app.authority.rows.push(gemini);
        let gemini = McpAppTarget {
            app_id: AppId::Gemini,
            installation_id: AppInstallationId("gemini-instance".into()),
            target_host_id: "host".into(),
        };
        let before = app.state.runtime_overrides.clone();
        drop(update(&mut app, ResyncMessage::Select(target(), true)));
        drop(update(&mut app, ResyncMessage::Select(gemini.clone(), true)));
        let start = run(update(&mut app, ResyncMessage::Start), &runtime);
        let recv = || requests.recv_timeout(Duration::from_secs(10)).unwrap();
        assert!(recv().starts_with("GET /v1/agent-apps/mcp?"));
        send(
            &responses,
            &McpCatalogResponse { records: vec![], bindings: vec![], capabilities: vec![] },
        );
        let native = recv();
        assert!(native.starts_with("GET /v1/agent-apps/mcp/native?"));
        assert!(native.contains("installationId=trusted-instance"));
        send(
            &responses,
            &McpNativePreview {
                target: target(),
                status: McpNativeStatus::Missing,
                entries: vec![],
            },
        );
        assert!(recv().contains("installationId=gemini-instance"));
        send(
            &responses,
            &McpNativePreview { target: gemini, status: McpNativeStatus::Present, entries: vec![] },
        );
        let mut initial = request();
        initial.targets.push(McpResyncTarget {
            app_id: AppId::Gemini,
            installation_id: AppInstallationId("gemini-instance".into()),
            account_id: None,
        });
        let submit = recv();
        assert!(submit.starts_with("POST /v1/agent-apps/mcp/resync "));
        assert!(submit.to_lowercase().contains("x-agent-apps-token: fixture"));
        assert_eq!(
            serde_json::from_str::<McpResyncRequest>(submit.split("\r\n\r\n").nth(1).unwrap())
                .unwrap(),
            initial
        );
        send(
            &responses,
            &AppActionExecutionResult {
                operation_id: "job /?".into(),
                status: vw_api_task::TaskStatus::Queued,
            },
        );
        let poll = run(update(&mut app, runtime.block_on(start).unwrap()), &runtime);
        let get = recv();
        assert!(get.starts_with("GET /v1/agent-apps/operations/job%20%2F%3F?targetHostId=host "));
        send(&responses, &status(AppOperationPhase::Executing, None));
        let next_poll = update(&mut app, runtime.block_on(poll).unwrap());
        assert_eq!(app.state.resync.status.as_ref().unwrap().targets.len(), 1);
        // Cancel supersedes a scheduled read; its late output cannot replace the result.
        let old_generation = app.state.resync.generation;
        drop(next_poll);
        let cancel = run(update(&mut app, ResyncMessage::Cancel), &runtime);
        assert!(
            recv().starts_with("DELETE /v1/agent-apps/operations/job%20%2F%3F?targetHostId=host ")
        );
        let terminal =
            status(AppOperationPhase::Terminal, Some(McpBatchOutcome::PartiallySucceeded));
        send(&responses, &terminal);
        let observations = run(update(&mut app, runtime.block_on(cancel).unwrap()), &runtime);
        for _ in 0..3 {
            let read = recv();
            assert!(read.starts_with("GET "));
            if read.contains("/mcp/native?") {
                let mut t = target();
                if read.contains("gemini-instance") {
                    t.app_id = AppId::Gemini;
                    t.installation_id = AppInstallationId("gemini-instance".into());
                }
                send(
                    &responses,
                    &McpNativePreview {
                        target: t,
                        status: McpNativeStatus::Missing,
                        entries: vec![],
                    },
                );
            } else {
                assert!(read.contains("/mcp?"));
                send(
                    &responses,
                    &McpCatalogResponse { records: vec![], bindings: vec![], capabilities: vec![] },
                );
            }
        }
        drop(update(&mut app, runtime.block_on(observations).unwrap()));
        assert_eq!(app.state.resync.observed.len(), 2);
        assert_eq!(app.state.resync.status, Some(terminal.clone()));
        assert!(catalog::loading(&app));
        let context = app.state.resync.context.clone().unwrap();
        drop(update(
            &mut app,
            ResyncMessage::Finished {
                context,
                generation: old_generation,
                result: Ok(ResyncResult::Status(status(AppOperationPhase::Executing, None))),
            },
        ));
        assert_eq!(app.state.resync.status, Some(terminal));
        for theme in [Theme::Light, Theme::Dark] {
            let labels = draw(super::fixture::view(&app), theme).join("\n");
            assert!(labels.contains("TargetUnavailable"));
            assert!(labels.contains("仅重试失败目标"));
            assert!(!labels.contains("/fixture/中文路径"));
        }
        let retry = run(update(&mut app, ResyncMessage::RetryFailed), &runtime);
        assert!(recv().starts_with("GET /v1/agent-apps/mcp?"));
        send(
            &responses,
            &McpCatalogResponse { records: vec![], bindings: vec![], capabilities: vec![] },
        );
        assert!(recv().starts_with("GET /v1/agent-apps/mcp/native?"));
        send(
            &responses,
            &McpNativePreview {
                target: target(),
                status: McpNativeStatus::Missing,
                entries: vec![],
            },
        );
        let submit = recv();
        assert_eq!(
            serde_json::from_str::<McpResyncRequest>(submit.split("\r\n\r\n").nth(1).unwrap())
                .unwrap(),
            request()
        );
        send(
            &responses,
            &AppActionExecutionResult {
                operation_id: "retry".into(),
                status: vw_api_task::TaskStatus::Queued,
            },
        );
        drop(update(&mut app, runtime.block_on(retry).unwrap()));
        assert_eq!(app.state.runtime_overrides, before);
    });
}
#[test]
fn cc_mcp_t48_unknown_submission_and_changed_credentials_prevent_replay_and_stale_results() {
    super::fixture::with_settings_gateway(|requests, _| {
        let mut app = setup();
        let mut foreign = target();
        foreign.installation_id.0 = "unknown".into();
        drop(update(&mut app, ResyncMessage::Select(foreign, true)));
        assert!(app.state.resync.selected.is_empty());
        drop(update(&mut app, ResyncMessage::Select(target(), true)));
        drop(update(&mut app, ResyncMessage::Start));
        let state = &app.state.resync;
        let context = state.context.clone().unwrap();
        let generation = state.generation;
        drop(update(
            &mut app,
            ResyncMessage::Finished {
                context: context.clone(),
                generation,
                result: Err(ResyncFailure {
                    label: "提交结果未知；禁止重复提交".into(),
                    uncertain: true,
                }),
            },
        ));
        assert!(app.state.resync.uncertain);
        let old = app.state.resync.generation;
        drop(update(&mut app, ResyncMessage::Start));
        assert_eq!(app.state.resync.generation, old);
        app.authority.generation += 1;
        drop(update(
            &mut app,
            ResyncMessage::Finished {
                context,
                generation,
                result: Ok(ResyncResult::Accepted(AppActionExecutionResult {
                    operation_id: "late".into(),
                    status: vw_api_task::TaskStatus::Queued,
                })),
            },
        ));
        assert!(app.state.resync.operation.is_none());
        assert!(requests.try_recv().is_err());
    });
}
#[test]
fn cc_mcp_t48_incomplete_foreign_and_duplicate_reports_cannot_finish_job() {
    super::fixture::with_settings_gateway(|_, _| {
        for variant in 0..4 {
            let mut app = setup();
            drop(update(&mut app, ResyncMessage::Select(target(), true)));
            drop(update(&mut app, ResyncMessage::Start));
            let context = app.state.resync.context.clone().unwrap();
            let generation = app.state.resync.generation;
            drop(update(
                &mut app,
                ResyncMessage::Finished {
                    context: context.clone(),
                    generation,
                    result: Ok(ResyncResult::Accepted(AppActionExecutionResult {
                        operation_id: "job /?".into(),
                        status: vw_api_task::TaskStatus::Queued,
                    })),
                },
            ));
            let generation = app.state.resync.generation;
            let mut report = status(AppOperationPhase::Terminal, Some(McpBatchOutcome::Failed));
            report.targets.retain(|t| t.app_id == AppId::Codex);
            match variant {
                0 => report.targets.clear(),
                1 => report.operation.operation_id = "foreign".into(),
                2 => report.targets.push(report.targets[0].clone()),
                _ => {
                    report.targets[0].app_id = AppId::Gemini;
                }
            }
            drop(update(
                &mut app,
                ResyncMessage::Finished {
                    context,
                    generation,
                    result: Ok(ResyncResult::Status(report)),
                },
            ));
            assert!(app.state.resync.status.is_none());
            assert!(app.state.resync.error.as_ref().unwrap().contains("不完整或目标不匹配"));
            assert_eq!(app.state.resync.operation.as_ref().unwrap().0, "job /?");
        }
    });
}
#[test]
fn cc_mcp_t48_cancellation_accepts_empty_unexecuted_items_partial_commit_and_completed_job() {
    use std::time::Duration;
    use vw_api_task::TaskStatus;
    super::fixture::with_settings_gateway(|requests, responses| {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .unwrap();
        for stage in 0..3 {
            let mut app = setup();
            drop(update(&mut app, ResyncMessage::Select(target(), true)));
            drop(update(&mut app, ResyncMessage::Start));
            let context = app.state.resync.context.clone().unwrap();
            let generation = app.state.resync.generation;
            drop(update(
                &mut app,
                ResyncMessage::Finished {
                    context,
                    generation,
                    result: Ok(ResyncResult::Accepted(AppActionExecutionResult {
                        operation_id: "job /?".into(),
                        status: TaskStatus::Queued,
                    })),
                },
            ));
            let mut items = Vec::new();
            if stage > 0 {
                items.push(McpDistributionItem {
                    record: McpRecordRef {
                        id: McpRecordId("中文目标😀".into()), revision: 4
                    },
                    target: target(),
                    enabled: true,
                    desired_committed: true,
                    native_outcome: McpDistributionOutcome::Applied,
                });
            }
            let outcome =
                if stage == 2 { McpBatchOutcome::Succeeded } else { McpBatchOutcome::Cancelled };
            let expected = McpResyncOperationStatus {
                operation: AppOperationStatus {
                    operation_id: "job /?".into(),
                    status: if stage == 2 { TaskStatus::Completed } else { TaskStatus::Cancelled },
                    phase: AppOperationPhase::Terminal,
                },
                outcome: Some(outcome),
                targets: vec![McpResyncTargetResult {
                    app_id: AppId::Codex,
                    code: None,
                    distribution: Some(McpDistributionResult { outcome, items }),
                }],
            };
            let cancel = run(update(&mut app, ResyncMessage::Cancel), &runtime);
            let request = requests.recv_timeout(Duration::from_secs(10)).unwrap();
            assert!(
                request.starts_with(
                    "DELETE /v1/agent-apps/operations/job%20%2F%3F?targetHostId=host "
                )
            );
            send(&responses, &expected);
            drop(update(&mut app, runtime.block_on(cancel).unwrap()));
            assert_eq!(app.state.resync.status, Some(expected));
            assert!(app.state.resync.error.is_none());
            for theme in [Theme::Light, Theme::Dark] {
                let labels = draw(super::fixture::view(&app), theme).join("\n");
                if stage > 0 {
                    assert!(labels.contains("中文目标😀"));
                    assert!(labels.contains("已应用"));
                }
                if stage < 2 {
                    assert!(labels.contains("已取消"));
                }
            }
        }
    });
}

#[test]
fn cc_mcp_t48_options_modal_draws_chinese_account_and_cancel_does_not_submit() {
    super::fixture::with_settings_gateway(|requests, _| {
        let mut app = setup();
        let mut row = app.authority.rows[0].clone();
        row.entry.id = AppId::Claude;
        let installation = &mut row.observation.as_mut().unwrap().installations[0];
        installation.app_id = AppId::Claude;
        app.authority.rows.push(row);
        drop(update(&mut app, ResyncMessage::Open));
        let mut claude = target();
        claude.app_id = AppId::Claude;
        drop(update(&mut app, ResyncMessage::Select(claude, true)));
        drop(update(&mut app, ResyncMessage::Account("中文账号引用 😀".into())));
        for theme in [Theme::Light, Theme::Dark] {
            let labels =
                draw(super::fixture::overlay(&app, iced::widget::text("底层页面").into()), theme)
                    .join("\n");
            assert!(labels.contains("重新同步应用"));
            assert!(labels.contains("确认重新同步"));
        }
        drop(update(&mut app, ResyncMessage::Close));
        assert!(!app.state.resync.open);
        assert!(requests.try_recv().is_err());
    });
}

#[test]
fn cc_mcp_t48_unverified_native_items_cannot_produce_success_feedback() {
    use vw_api_task::TaskStatus;
    super::fixture::with_settings_gateway(|_, _| {
        for native_outcome in [
            McpDistributionOutcome::NotVerified,
            McpDistributionOutcome::Unsupported,
            McpDistributionOutcome::Cancelled,
        ] {
            let mut app = setup();
            drop(update(&mut app, ResyncMessage::Select(target(), true)));
            drop(update(&mut app, ResyncMessage::Start));
            let context = app.state.resync.context.clone().unwrap();
            let generation = app.state.resync.generation;
            drop(update(
                &mut app,
                ResyncMessage::Finished {
                    context: context.clone(),
                    generation,
                    result: Ok(ResyncResult::Accepted(AppActionExecutionResult {
                        operation_id: "job /?".into(),
                        status: TaskStatus::Queued,
                    })),
                },
            ));
            let generation = app.state.resync.generation;
            let report = McpResyncOperationStatus {
                operation: AppOperationStatus {
                    operation_id: "job /?".into(),
                    status: TaskStatus::Completed,
                    phase: AppOperationPhase::Terminal,
                },
                outcome: Some(McpBatchOutcome::Succeeded),
                targets: vec![McpResyncTargetResult {
                    app_id: AppId::Codex,
                    code: None,
                    distribution: Some(McpDistributionResult {
                        outcome: McpBatchOutcome::Succeeded,
                        items: vec![McpDistributionItem {
                            record: McpRecordRef {
                                id: McpRecordId("中文目标😀".into()),
                                revision: 4,
                            },
                            target: target(),
                            enabled: true,
                            desired_committed: true,
                            native_outcome,
                        }],
                    }),
                }],
            };
            drop(update(
                &mut app,
                ResyncMessage::Finished {
                    context,
                    generation,
                    result: Ok(ResyncResult::Status(report)),
                },
            ));
            assert!(matches!(
                app.effects.iter().rev().find(|e| matches!(
                    e,
                    crate::management_effect::Effect::Error(_)
                        | crate::management_effect::Effect::Success(_)
                )),
                Some(crate::management_effect::Effect::Error(_))
            ));
            for theme in [Theme::Light, Theme::Dark] {
                let labels = draw(super::fixture::view(&app), theme).join("\n");
                assert!(labels.contains("未全部验证"));
                assert!(!labels.contains("总体 全部成功"));
            }
        }
    });
}
