use super::aggregation_desktop::{Action, Reply, execute, finish};
use crate::app::state::{
    AgentAppsContext,
    app_providers::{desktop::Form, list::Scope},
};
use std::io::{Read, Write};
use vw_gateway_client::vw_api_types::{
    agent_apps::aggregation::*, app_providers::desktop_mapping::DesktopModelRoute,
};
use vw_gateway_client::{GatewayClient, GatewayEndpoint, vw_api_types::agent_apps::AppId};
fn fixture(
    path: &'static str,
    status: u16,
    response: &'static str,
) -> (GatewayClient, Scope, std::thread::JoinHandle<Option<serde_json::Value>>) {
    fixture_sequence(vec![(path, status, response)])
}
fn fixture_sequence(
    responses: Vec<(&'static str, u16, &'static str)>,
) -> (GatewayClient, Scope, std::thread::JoinHandle<Option<serde_json::Value>>) {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let client = GatewayClient::new(
        GatewayEndpoint::new("127.0.0.1", listener.local_addr().unwrap().port())
            .with_auth(vw_gateway_client::GatewayAuth { skey: Some("fake-host-token".into()) }),
    )
    .unwrap();
    let scope = Scope {
        context: AgentAppsContext {
            gateway: client.endpoint().base_url(),
            tenant: client.endpoint().tenant_scope().clone(),
            project: None,
        },
        host: "fixture-host".into(),
        app: AppId::ClaudeDesktop,
    };
    let server = std::thread::spawn(move || {
        listener.set_nonblocking(true).unwrap();
        let mut captured = None;
        for (path, status, response) in responses {
            let start = std::time::Instant::now();
            let mut stream = loop {
                match listener.accept() {
                    Ok((stream, _)) => break stream,
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        if start.elapsed().as_secs() >= 3 {
                            return None;
                        }
                        std::thread::sleep(std::time::Duration::from_millis(10));
                    }
                    Err(e) => panic!("{e}"),
                }
            };
            // Bound accept with a nonblocking listener, then read each request in blocking mode.
            stream.set_nonblocking(false).unwrap();
            stream.set_read_timeout(Some(std::time::Duration::from_secs(10))).unwrap();
            let mut head = Vec::new();
            while !head.ends_with(b"\r\n\r\n") {
                let mut byte = [0];
                stream.read_exact(&mut byte).unwrap_or_else(|e| {
                    panic!("{path}: incomplete request head ({} bytes): {e}", head.len())
                });
                head.push(byte[0]);
            }
            let head = String::from_utf8(head).unwrap();
            assert!(head.starts_with(path), "{head}");
            assert!(
                head.to_lowercase().contains("authorization: bearer fake-host-token"),
                "{head}"
            );
            if head.starts_with("GET ") || head.starts_with("DELETE ") {
                assert!(head.contains("projectId=fixture-project"), "{head}");
                assert!(head.contains("targetHostId=fixture-host"), "{head}");
            }
            let len = head
                .lines()
                .find_map(|l| {
                    l.to_lowercase()
                        .strip_prefix("content-length:")
                        .map(|s| s.trim().parse::<usize>().unwrap())
                })
                .unwrap_or(0);
            let mut bytes = vec![0; len];
            stream.read_exact(&mut bytes).unwrap();
            let value = if bytes.is_empty() {
                serde_json::Value::Null
            } else {
                serde_json::from_slice(&bytes).unwrap()
            };
            write!(stream,"HTTP/1.1 {status} Result\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{response}",response.len()).unwrap();
            captured = Some(value);
        }
        captured
    });
    (client, scope, server)
}
#[tokio::test]
async fn aggregation_t63_aggregation_desktop_preserves_contract() {
    let (client, scope, server) =
        fixture("PUT /v1/app-providers/claude-desktop/fixture/model-mapping ", 409, "{}");
    let result = execute(client, scoped_form(scope), Action::Save).await;
    assert!(matches!(
        result,
        Err(vw_gateway_client::AgentAppsSettingsError::Http { status: 409, .. })
    ));
    let body = server.join().unwrap().unwrap();
    assert_eq!(
        body["projectId"], "fixture-project",
        "Desktop mapping writes must retain project scope"
    );
    assert_eq!(body["expectedRevision"], "r1");
    assert_eq!(body["mapping"]["mode"], "proxy");
    assert_eq!(body["mapping"]["routes"]["claude-sonnet-4-6"]["model"], "deepseek-v4-pro");
    assert_eq!(body["mapping"]["routes"]["claude-sonnet-4-6"]["supports1m"], true);
    assert!(
        body["mapping"]["routes"]["claude-sonnet-4-6"]["labelOverride"]
            .as_str()
            .unwrap()
            .contains("中文模型")
    );
}

fn scoped_form(scope: Scope) -> Form {
    let mut form = Form::new("fixture".into());
    let mut session =
        super::aggregation_desktop::Session::new(super::aggregation_desktop::Binding {
            scope,
            project: "fixture-project".into(),
            access: 7,
        });
    session.revision = "r1".into();
    session.status = Some(DesktopProfileStatus {
        revision: "status-r1".into(),
        installation: DesktopInstallation::Installed,
        configured: true,
        mode: DesktopProfileMode::Routing,
        applied_provider_id: Some("fixture".into()),
        notices: vec![AggregationNotice::RestartDesktop],
        recovery: RecoveryState::None,
    });
    form.aggregation = Some(session);
    form.mapping_editable = true;
    form.mapping.mode =
        vw_gateway_client::vw_api_types::app_providers::desktop_mapping::DesktopMappingMode::Proxy;
    form.create = false;
    form.name = "中文模型供应商".into();
    form.mapping.routes.insert(
        "claude-sonnet-4-6".into(),
        DesktopModelRoute {
            model: "deepseek-v4-pro".into(),
            label_override: Some("中文模型／长描述".repeat(12)),
            supports_1m: true,
        },
    );
    form
}
fn preview(blocking: bool) -> AggregationPreview {
    AggregationPreview {
        revision: "preview-r2".into(),
        preview_token: Some("opaque-token".into()),
        default_provider_id: Some("fixture".into()),
        members: vec![],
        issues: if blocking {
            vec![PreviewIssue {
                code: PreviewIssueCode::FailoverEnabled,
                blocking: true,
                provider_id: None,
                model_id: None,
            }]
        } else {
            vec![]
        },
        notices: vec![AggregationNotice::RestartDesktop],
    }
}
#[test]
fn aggregation_t63_preview_blockers_and_failure_preserve_draft() {
    let (_, scope, server) = fixture("", 200, "{}");
    // The no-request fixture expires; this test exercises UI state, not transport.
    let mut form = scoped_form(scope);
    finish(&mut form, Ok(Reply::Preview(preview(true))));
    assert!(!form.aggregation.as_ref().unwrap().can_publish());
    finish(&mut form, Ok(Reply::Preview(preview(false))));
    assert!(form.aggregation.as_ref().unwrap().can_publish());
    form.aggregation.as_mut().unwrap().invalidate();
    assert!(!form.aggregation.as_ref().unwrap().can_publish());
    for status in [401, 403, 409, 422, 501, 502, 503, 504] {
        finish(
            &mut form,
            Err(vw_gateway_client::AgentAppsSettingsError::Http {
                status,
                body: serde_json::json!({"secret":"never-display"}),
            }),
        );
        assert!(form.aggregation.as_ref().unwrap().revision.is_empty());
        assert!(form.aggregation.as_ref().unwrap().status.is_none());
        assert!(form.notice.as_ref().is_some_and(|n| !n.contains("secret")));
        assert_eq!(form.mapping.routes["claude-sonnet-4-6"].model, "deepseek-v4-pro");
    }
    assert!(server.join().unwrap().is_none());
}
#[tokio::test]
async fn aggregation_t63_preview_publish_restore_and_progress_use_scoped_sdk() {
    for (action, path, reply) in [
        (
            Action::Preview,
            "POST /v1/agent-apps/claude-desktop/aggregation/preview ",
            serde_json::to_string(&preview(false)).unwrap(),
        ),
        (
            Action::Publish,
            "POST /v1/agent-apps/claude-desktop/aggregation/publish ",
            operation_json("running", "settling", serde_json::Value::Null),
        ),
        (
            Action::Restore,
            "POST /v1/agent-apps/claude-desktop/aggregation/exit ",
            operation_json(
                "failed",
                "terminal",
                serde_json::json!({"kind":"failed","status":502,"code":"recovery_failed","recovery":"failed"}),
            ),
        ),
        (
            Action::Poll,
            "GET /v1/app-providers/operations/op?",
            operation_json("running", "settling", serde_json::Value::Null),
        ),
        (
            Action::Cancel,
            "DELETE /v1/app-providers/operations/op?",
            operation_json("running", "settling", serde_json::Value::Null),
        ),
    ] {
        let (client, scope, server) = fixture(path, 200, Box::leak(reply.into_boxed_str()));
        let mut form = scoped_form(scope);
        form.aggregation.as_mut().unwrap().preview = Some(preview(false));
        form.aggregation.as_mut().unwrap().operation = Some(
            serde_json::from_str(&operation_json("running", "settling", serde_json::Value::Null))
                .unwrap(),
        );
        let result = execute(client, form.clone(), action.clone()).await;
        let body = server.join().unwrap().unwrap();
        if !matches!(action, Action::Poll | Action::Cancel) {
            assert_eq!(body["projectId"], "fixture-project");
            assert_eq!(body["targetHostId"], "fixture-host");
        }
        if matches!(action, Action::Publish) {
            assert_eq!(body["expectedRevision"], "preview-r2");
            assert_eq!(body["previewToken"], "opaque-token");
            assert_eq!(body["confirmed"], true);
        }
        if matches!(action, Action::Restore) {
            assert_eq!(body["expectedRevision"], "status-r1");
            assert_eq!(body["confirmed"], true);
        }
        assert!(result.is_ok(), "{result:?}");
        finish(&mut form, result);
        if matches!(action, Action::Publish | Action::Cancel | Action::Poll) {
            assert!(form.notice.as_ref().unwrap().contains("进行中"));
            assert!(!form.notice.as_ref().unwrap().contains("已发布"));
        }
        if matches!(action, Action::Restore) {
            assert!(form.notice.as_ref().unwrap().contains("RecoveryFailed"));
        }
    }
}
fn operation_json(status: &str, phase: &str, outcome: serde_json::Value) -> String {
    let mut value = serde_json::json!({"operationId":"op","requestId":"request","status":status,"phase":phase,"completed":0,"total":1,"cancellationRequested":true});
    if !outcome.is_null() {
        value["aggregationOutcome"] = outcome;
    }
    value.to_string()
}
#[tokio::test]
async fn aggregation_t63_reads_status_and_redacted_mapping_from_sdk() {
    let (client, scope, server) =
        fixture("GET /v1/agent-apps/claude-desktop/profile/status?", 403, "{}");
    let mut form = scoped_form(scope);
    let result = execute(client, form.clone(), Action::Read).await;
    assert!(server.join().unwrap().is_some());
    finish(&mut form, result);
    assert!(form.notice.unwrap().contains("无权"));
}
#[tokio::test]
async fn aggregation_t63_draws_chinese_mapping_in_light_and_dark() {
    use iced::{
        Font, Size, Theme,
        advanced::renderer::{Headless, Style},
        mouse,
        widget::Space,
    };
    use iced_runtime::user_interface::{Cache, UserInterface};
    {
        let mut fonts = iced::advanced::graphics::text::font_system().write().unwrap();
        for font in crate::fonts::load_all() {
            fonts.load_font(font);
        }
    }
    let (client, scope, server) = fixture("", 200, "{}");
    drop(client);
    let mut form = scoped_form(scope);
    form.notice = Some("中文长说明与失败反馈：请检查模型、配置与权限。".repeat(18));
    let mut renderer =
        iced::Renderer::new(Font::with_name("Noto Sans CJK SC"), 14.0.into(), Some("tiny-skia"))
            .await
            .expect("renderer");
    for (name, theme) in [("light", Theme::Light), ("dark", Theme::Dark)] {
        let help = crate::app::components::app_providers::list::Help::new(
            crate::app::i18n::I18n::new(vw_config_types::ui::DesktopLocale::SimplifiedChinese),
            &theme,
        );
        let element = crate::app::components::app_providers::desktop::overlay(
            &form,
            help,
            Space::new().width(iced::Length::Fill).height(iced::Length::Fill).into(),
        );
        let size = Size::new(1000, 900);
        let mut ui =
            UserInterface::build(element, Size::new(1000.0, 900.0), Cache::new(), &mut renderer);
        let mut messages = vec![];
        ui.update(
            &[iced::Event::Window(
                iced::window::Event::RedrawRequested(iced::time::Instant::now()),
            )],
            mouse::Cursor::Unavailable,
            &mut renderer,
            &mut iced::advanced::clipboard::Null,
            &mut messages,
        );
        ui.draw(
            &mut renderer,
            &theme,
            &Style { text_color: theme.palette().text },
            mouse::Cursor::Unavailable,
        );
        let mut before = Geometry::default();
        ui.operate(&renderer, &mut before);
        let pixels = renderer.screenshot(size, 1.0, theme.palette().background);
        assert_eq!(pixels.len(), 1000 * 900 * 4);
        assert!(pixels.chunks_exact(4).any(|p| p != &pixels[..4]));
        if let Some(root) = std::env::var_os("SHIJU_T63_RENDER_DIR") {
            std::fs::create_dir_all(&root).unwrap();
            image::save_buffer(
                std::path::PathBuf::from(root).join(format!("{name}.png")),
                &pixels,
                1000,
                900,
                image::ColorType::Rgba8,
            )
            .unwrap();
        }
        ui.update(
            &[
                iced::Event::Mouse(mouse::Event::WheelScrolled {
                    delta: mouse::ScrollDelta::Lines { x: 0.0, y: -80.0 },
                }),
                iced::Event::Window(iced::window::Event::RedrawRequested(
                    iced::time::Instant::now(),
                )),
            ],
            mouse::Cursor::Available(iced::Point::new(500.0, 400.0)),
            &mut renderer,
            &mut iced::advanced::clipboard::Null,
            &mut messages,
        );
        ui.draw(
            &mut renderer,
            &theme,
            &Style { text_color: theme.palette().text },
            mouse::Cursor::Unavailable,
        );
        let mut after = Geometry::default();
        ui.operate(&renderer, &mut after);
        assert!(before.header.is_some() && before.footer.is_some());
        assert_eq!(before.header, after.header);
        assert_eq!(before.footer, after.footer);
        let (viewport, content, offset) = after.scroll.unwrap();
        assert!(content.height > viewport.height && offset.y > before.scroll.unwrap().2.y);
        let bottom = renderer.screenshot(size, 1.0, theme.palette().background);
        assert!(pixels != bottom, "long form must scroll into the actual model controls");
        if let Some(root) = std::env::var_os("SHIJU_T63_RENDER_DIR") {
            image::save_buffer(
                std::path::PathBuf::from(root).join(format!("{name}-mapping.png")),
                &bottom,
                1000,
                900,
                image::ColorType::Rgba8,
            )
            .unwrap();
        }
        let _ = ui.into_cache();
    }
    assert!(server.join().unwrap().is_none());
}

#[derive(Default)]
struct Geometry {
    header: Option<iced::Rectangle>,
    footer: Option<iced::Rectangle>,
    scroll: Option<(iced::Rectangle, iced::Rectangle, iced::Vector)>,
}
impl iced::advanced::widget::operation::Operation for Geometry {
    fn traverse(
        &mut self,
        operate: &mut dyn FnMut(&mut dyn iced::advanced::widget::operation::Operation),
    ) {
        operate(self);
    }
    fn text(
        &mut self,
        _: Option<&iced::advanced::widget::Id>,
        bounds: iced::Rectangle,
        value: &str,
    ) {
        match value {
            "Desktop 模型映射与应用" => self.header = Some(bounds),
            "保存映射" => self.footer = Some(bounds),
            _ => {}
        }
    }
    fn scrollable(
        &mut self,
        _: Option<&iced::advanced::widget::Id>,
        bounds: iced::Rectangle,
        content: iced::Rectangle,
        translation: iced::Vector,
        _: &mut dyn iced::advanced::widget::operation::Scrollable,
    ) {
        self.scroll.get_or_insert((bounds, content, translation));
    }
}

#[tokio::test]
async fn aggregation_t63_refresh_and_save_accept_only_gateway_revisions() {
    let (_, scope, server) = fixture_sequence(vec![]);
    server.join().unwrap();
    let mut form = scoped_form(scope);
    let status = form.aggregation.as_ref().unwrap().status.clone().unwrap();
    let view = DesktopMappingView {
        revision: "mapping-r2".into(),
        form: vw_gateway_client::vw_api_types::app_providers::desktop_mapping::DesktopFormView {
            name: "新中文档案".into(),
            base_url: "https://example.invalid".into(),
            official: false,
            direct_editable: true,
            mapping: form.mapping.clone(),
            mapping_editable: true,
        },
    };
    let (client, scope, server) = fixture_sequence(vec![
        (
            "GET /v1/agent-apps/claude-desktop/profile/status?",
            200,
            Box::leak(serde_json::to_string(&status).unwrap().into_boxed_str()),
        ),
        (
            "POST /v1/app-providers/claude-desktop/fixture/form/read ",
            200,
            Box::leak(serde_json::to_string(&view).unwrap().into_boxed_str()),
        ),
    ]);
    form = scoped_form(scope);
    let reply = execute(client, form.clone(), Action::Read).await.unwrap();
    let body = server.join().unwrap().unwrap();
    assert_eq!(body["projectId"], "fixture-project");
    finish(&mut form, Ok(reply));
    assert_eq!(form.name, "新中文档案");
    assert_eq!(form.aggregation.as_ref().unwrap().revision, "mapping-r2");
    assert_eq!(
        form.aggregation.as_ref().unwrap().status.as_ref().unwrap().mode,
        DesktopProfileMode::Routing
    );
    form.aggregation.as_mut().unwrap().dirty = true;
    form.aggregation.as_mut().unwrap().preview = Some(preview(false));
    assert!(!form.aggregation.as_ref().unwrap().can_publish());
    finish(&mut form, Ok(Reply::Saved(view)));
    assert!(!form.aggregation.as_ref().unwrap().dirty);
    assert!(form.aggregation.as_ref().unwrap().preview.is_none());
    assert!(form.aggregation.as_ref().unwrap().status.is_none());
}
