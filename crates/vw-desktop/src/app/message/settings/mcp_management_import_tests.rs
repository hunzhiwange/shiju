use crate::app::{App, Message};
use iced::{
    Font, Rectangle, Renderer, Size, Theme,
    advanced::{
        renderer::{Headless, Style},
        widget::{Id, Operation},
    },
    mouse,
};
use iced_runtime::user_interface::{Cache, UserInterface};
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
fn draw(element: iced::Element<'_, Message>, theme: Theme) -> Vec<String> {
    let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
    let mut renderer =
        runtime.block_on(Renderer::new(Font::DEFAULT, 14.0.into(), Some("tiny-skia"))).unwrap();
    {
        let mut fonts = iced::advanced::graphics::text::font_system().write().unwrap();
        for font in crate::fonts::load_all() {
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
#[test]
fn cc_mcp_t47_settings_exposes_explicit_native_import_confirmation() {
    let (_lock, _home, _guard) = crate::app::config::isolated_settings();
    let app = App::new().0;
    for theme in [Theme::Light, Theme::Dark] {
        let labels = draw(crate::app::components::system_settings_mcp::view(&app), theme);
        assert!(
            labels.iter().any(|text| text == "从原生应用导入"),
            "missing native import confirmation entry: {labels:?}"
        );
    }
}

use super::*;
use vw_gateway_client::vw_api_types::{TenantScope, agent_apps::*};
fn target() -> McpAppTarget {
    McpAppTarget {
        app_id: AppId::Codex,
        installation_id: AppInstallationId("trusted-instance".into()),
        target_host_id: "host".into(),
    }
}
fn setup() -> App {
    use crate::app::components::system_settings_apps::catalog::CatalogRow;
    let mut app = App::new().0;
    app.apps_catalog.host = "host".into();
    app.apps_catalog.token = "fixture".into();
    app.apps_catalog.context = Some(catalog::context(&app).unwrap().endpoint);
    app.apps_catalog.rows.push(CatalogRow {
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
fn preview() -> McpNativePreview {
    McpNativePreview {
        target: target(),
        status: McpNativeStatus::Present,
        entries: vec![McpNativePreviewEntry {
            id: McpRecordId("已有中文 😀".into()),
            status: McpNativeStatus::Present,
            enabled: Some(true),
            transport: Some(McpTransport::Stdio),
        }],
    }
}
fn response() -> McpImportResponse {
    let record = |id: &str| McpRecordRef { id: McpRecordId(id.into()), revision: 2 };
    McpImportResponse {
        sources: vec![McpImportSourceResult {
            target: target(),
            code: None,
            entries: Some(vec![
                McpImportEntryResult {
                    id: Some(McpRecordId("new".into())),
                    outcome: McpImportEntryOutcome::Added { record: record("new") },
                },
                McpImportEntryResult {
                    id: Some(McpRecordId("existing".into())),
                    outcome: McpImportEntryOutcome::Unchanged { record: record("existing") },
                },
                McpImportEntryResult {
                    id: Some(McpRecordId("merged".into())),
                    outcome: McpImportEntryOutcome::Merged { record: record("merged") },
                },
                McpImportEntryResult {
                    id: Some(McpRecordId("conflict".into())),
                    outcome: McpImportEntryOutcome::Failed { code: McpFailureCode::Conflict },
                },
            ]),
        }],
    }
}
#[test]
fn cc_mcp_t47_results_preserve_conflicts_imported_and_unchanged_items() {
    let labels = result_labels(response(), &target()).unwrap().join("\n");
    assert!(labels.contains("new · 已导入"));
    assert!(labels.contains("existing · 已导入，无需变更"));
    assert!(labels.contains("conflict · 冲突，未覆盖"));
    assert!(labels.contains("merged · 已合并"));
    let mut malformed = response();
    malformed.sources[0].target.target_host_id = "other".into();
    assert!(result_labels(malformed, &target()).is_err());
    let mut malformed = response();
    malformed.sources[0].code = Some(McpFailureCode::Conflict);
    assert!(result_labels(malformed, &target()).is_err());
    let mut malformed = response();
    malformed.sources.clear();
    assert!(result_labels(malformed, &target()).is_err());
}
fn run(
    task: Task<Message>,
    runtime: &tokio::runtime::Runtime,
) -> tokio::task::JoinHandle<ImportMessage> {
    use iced_runtime::{Action, futures::futures::StreamExt};
    runtime.spawn(async move {
        let mut stream = iced_runtime::task::into_stream(task).expect("request task");
        while let Some(action) = stream.next().await {
            if let Action::Output(Message::Settings(SettingsMessage::Mcp(McpMessage::Import(
                event,
            )))) = action
            {
                return event;
            }
        }
        panic!("missing import response")
    })
}
#[test]
fn cc_mcp_t47_confirm_uses_real_sdk_and_cancel_never_posts_or_reads_local_files() {
    use std::time::Duration;
    crate::app::config::with_settings_gateway(|requests, responses| {
        let mut app = setup();
        let runtime_before = app.mcp_settings.runtime_overrides.clone();
        let acp_before = app.acp_settings.enabled.clone();
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .unwrap();
        drop(update(&mut app, ImportMessage::Open));
        let mut foreign = target();
        foreign.target_host_id = "foreign".into();
        drop(update(&mut app, ImportMessage::Select(foreign)));
        assert!(app.mcp_settings.import.as_ref().unwrap().source.is_none());
        drop(update(&mut app, ImportMessage::Select(target())));
        drop(update(&mut app, ImportMessage::Confirm));
        assert!(requests.try_recv().is_err(), "confirmation before preview must not write");
        let pending = run(update(&mut app, ImportMessage::Preview), &runtime);
        let request = requests.recv_timeout(Duration::from_secs(5)).unwrap();
        assert!(request.starts_with("GET /v1/agent-apps/mcp/native?"));
        assert!(request.contains("installationId=trusted-instance"));
        assert!(request.contains("targetHostId=host"));
        assert!(!request.contains("/fixture/"));
        responses.send(Some(serde_json::to_string(&preview()).unwrap())).unwrap();
        let event = runtime.block_on(pending).unwrap();
        drop(update(&mut app, ImportMessage::Close));
        drop(update(&mut app, event));
        assert!(app.mcp_settings.import.is_none());
        assert!(requests.try_recv().is_err());
        drop(update(&mut app, ImportMessage::Open));
        drop(update(&mut app, ImportMessage::Select(target())));
        let pending = run(update(&mut app, ImportMessage::Preview), &runtime);
        assert!(
            requests
                .recv_timeout(Duration::from_secs(5))
                .unwrap()
                .starts_with("GET /v1/agent-apps/mcp/native?")
        );
        responses.send(Some(serde_json::to_string(&preview()).unwrap())).unwrap();
        drop(update(&mut app, runtime.block_on(pending).unwrap()));
        let state = app.mcp_settings.import.as_ref().unwrap();
        assert!(state.can_confirm());
        let state_context = state.context.clone();
        let generation = state.generation;
        let pending = run(update(&mut app, ImportMessage::Confirm), &runtime);
        let request = requests.recv_timeout(Duration::from_secs(5)).unwrap();
        assert!(request.starts_with("POST /v1/agent-apps/mcp/import "));
        assert!(request.to_lowercase().contains("x-agent-apps-token: fixture"));
        let body = request.split("\r\n\r\n").nth(1).unwrap();
        let body: McpImportRequest = serde_json::from_str(body).unwrap();
        assert_eq!(
            body,
            McpImportRequest {
                project_id: None,
                sources: vec![McpImportSource { target: target(), account_id: None }]
            }
        );
        responses.send(Some(serde_json::to_string(&response()).unwrap())).unwrap();
        let event = runtime.block_on(pending).unwrap();
        drop(update(&mut app, event));
        let labels = app.mcp_settings.import.as_ref().unwrap().result.clone().unwrap().unwrap();
        assert!(labels.iter().any(|s| s.contains("冲突")));
        assert!(catalog::loading(&app));
        drop(update(
            &mut app,
            ImportMessage::Finished(state_context, generation, Err("late".into())),
        ));
        assert_eq!(app.mcp_settings.import.as_ref().unwrap().result, Some(Ok(labels)));
        assert_eq!(app.mcp_settings.runtime_overrides, runtime_before);
        assert_eq!(app.acp_settings.enabled, acp_before);
    });
}
fn draft() -> McpCreateRequest {
    McpCreateRequest {
        target_host_id: "host".into(),
        project_id: None,
        id: "draft".into(),
        name: Some("中文草稿 😀".into()),
        description: None,
        homepage: None,
        documentation: None,
        tags: vec![],
        connection: McpConnectionDefinition::Stdio {
            command: "fixture-command".into(),
            args: vec![],
            env: [("TOKEN".into(), McpConfigValue::Literal { value: "must-not-render".into() })]
                .into(),
            cwd: None,
        },
    }
}
#[test]
fn cc_mcp_t47_clipboard_draft_requires_confirmation_and_cancellation_discards_payload() {
    crate::app::config::with_settings_gateway(|requests, _| {
        let mut app = setup();
        drop(update(&mut app, ImportMessage::Open));
        let _clipboard_task = update(&mut app, ImportMessage::Paste);
        let state = app.mcp_settings.import.as_ref().unwrap();
        let context = state.context.clone();
        let generation = state.generation;
        drop(update(
            &mut app,
            ImportMessage::Pasted(
                context.clone(),
                generation.wrapping_sub(1),
                Some(serde_json::to_string(&draft()).unwrap()),
            ),
        ));
        assert!(app.mcp_settings.import.as_ref().unwrap().draft.is_none());
        drop(update(
            &mut app,
            ImportMessage::Pasted(
                context,
                generation,
                Some(serde_json::to_string(&draft()).unwrap()),
            ),
        ));
        assert!(app.mcp_settings.import.as_ref().unwrap().can_confirm());
        assert!(requests.try_recv().is_err(), "pasting only forms a pending draft");
        for theme in [Theme::Light, Theme::Dark] {
            let labels = draw(
                crate::app::components::system_settings_mcp::view_overlays(
                    &app,
                    iced::widget::text("页面").into(),
                ),
                theme,
            )
            .join("\n");
            assert!(labels.contains("深链草稿确认"));
            assert!(labels.contains("中文草稿"));
            assert!(!labels.contains("must-not-render"));
            assert!(!labels.contains("fixture-command"));
        }
        drop(update(&mut app, ImportMessage::Close));
        assert!(app.mcp_settings.import.is_none());
        assert!(requests.try_recv().is_err());
    });
}
#[test]
fn cc_mcp_t47_native_modal_draws_redacted_preview_and_conflicts_in_both_themes() {
    crate::app::config::with_settings_gateway(|_, _| {
        let mut app = setup();
        drop(update(&mut app, ImportMessage::Open));
        drop(update(&mut app, ImportMessage::Select(target())));
        let state = app.mcp_settings.import.as_mut().unwrap();
        state.preview = Some(preview());
        state.result = Some(result_labels(response(), &target()));
        for theme in [Theme::Light, Theme::Dark] {
            let labels = draw(
                crate::app::components::system_settings_mcp::view_overlays(
                    &app,
                    iced::widget::text("页面").into(),
                ),
                theme,
            )
            .join("\n");
            assert!(labels.contains("冲突"));
            assert!(labels.contains("已导入"));
            assert!(labels.contains("已有中文"));
            assert!(labels.contains("仅对比已有目录"));
            assert!(!labels.contains("/fixture/"));
        }
        let old = app.mcp_settings.import.as_ref().unwrap().context.clone();
        let generation = app.mcp_settings.import.as_ref().unwrap().generation;
        app.apps_catalog.generation += 1;
        drop(update(&mut app, ImportMessage::Previewed(old, generation, Ok(preview()))));
        assert!(visible(&app).is_none());
        drop(update(&mut app, ImportMessage::Open));
        assert!(visible(&app).is_some());
    });
}
#[test]
fn cc_mcp_t47_draft_confirmation_creates_definition_only_through_gateway() {
    use std::time::Duration;
    crate::app::config::with_settings_gateway(|requests, responses| {
        let mut app = setup();
        drop(open_draft(&mut app, draft()));
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .unwrap();
        let pending = run(update(&mut app, ImportMessage::Confirm), &runtime);
        assert!(app.mcp_settings.import.as_ref().unwrap().importing);
        drop(update(&mut app, ImportMessage::Close));
        assert!(app.mcp_settings.import.is_some(), "accepted writes cannot be cancelled locally");
        let request = requests.recv_timeout(Duration::from_secs(5)).unwrap();
        assert!(request.starts_with("POST /v1/agent-apps/mcp "));
        let body: McpCreateRequest =
            serde_json::from_str(request.split("\r\n\r\n").nth(1).unwrap()).unwrap();
        assert!(body == draft());
        let record = McpCatalogRecord {
            record: McpRecordRef { id: McpRecordId("draft".into()), revision: 1 },
            scope: McpCatalogScope::Tenant {
                scope: TenantScope::new("10000", "100000001").unwrap(),
            },
            name: "中文草稿 😀".into(),
            description: None,
            homepage: None,
            documentation: None,
            tags: vec![],
            transport: McpTransport::Stdio,
            connection_revision: 1,
        };
        responses.send(Some(serde_json::to_string(&record).unwrap())).unwrap();
        drop(update(&mut app, runtime.block_on(pending).unwrap()));
        assert!(
            app.mcp_settings.import.as_ref().unwrap().result.as_ref().unwrap().as_ref().unwrap()[0]
                .contains("尚未分发或启用")
        );
        assert!(requests.try_recv().is_err(), "no binding, connection, or native write request");
    });
}
