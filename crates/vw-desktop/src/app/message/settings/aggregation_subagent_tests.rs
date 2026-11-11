use super::aggregation_subagent::*;
use vw_gateway_client::vw_api_types::{ReasoningEffortDto as Effort, agent_apps::aggregation::*};
fn snapshot() -> CodexAggregationSettings {
    let requested = CodexSettingsValues {
        subagent_model_id: Some("published-a".into()),
        subagent_effort: Some(Effort::High),
        auto_review_model_id: Some("review-keep".into()),
        classic_tools: false,
    };
    CodexAggregationSettings {
        revision: "r1".into(),
        effective: requested.clone(),
        requested,
        choices: vec![
            CodexSettingsChoice {
                model_id: "published-a".into(),
                label: "中文模型／日本語".into(),
                efforts: vec![Effort::Low, Effort::High],
            },
            CodexSettingsChoice {
                model_id: "published-b".into(),
                label: "另一个模型".into(),
                efforts: vec![Effort::Low],
            },
        ],
        notices: vec![AggregationNotice::NewSession],
    }
}
#[test]
fn aggregation_t60_aggregation_subagent_preserves_contract() {
    let state = snapshot();
    let mut form =
        Form { draft: state.requested.clone(), snapshot: Some(state), ..Default::default() };
    form.select_model(None);
    assert_eq!(form.draft.subagent_model_id, None);
    assert_eq!(form.draft.subagent_effort, None, "following main must clear reasoning override");
    assert_eq!(form.draft.auto_review_model_id.as_deref(), Some("review-keep"));
    assert!(!form.draft.classic_tools);
}
fn loaded() -> Form {
    let state = snapshot();
    Form { draft: state.requested.clone(), snapshot: Some(state), ..Default::default() }
}
fn context() -> Context {
    use crate::app::state::{AgentAppsContext, app_providers::list::Scope};
    Context {
        scope: Scope {
            context: AgentAppsContext {
                gateway: "http://127.0.0.1".into(),
                tenant: Default::default(),
                project: Some("/fixture".into()),
            },
            host: "host-t60".into(),
            app: vw_gateway_client::vw_api_types::agent_apps::AppId::Codex,
        },
        project: "project-t60".into(),
        access: 4,
    }
}
#[test]
fn aggregation_t60_switch_model_only_retains_supported_effort() {
    let mut form = loaded();
    form.select_model(Some("published-b".into()));
    assert_eq!(form.draft.subagent_effort, None);
    form.select_effort(Some(Effort::High));
    assert_eq!(form.draft.subagent_effort, None);
    form.select_effort(Some(Effort::Low));
    form.select_model(Some("published-a".into()));
    assert_eq!(form.draft.subagent_effort, Some(Effort::Low));
    form.select_effort(None);
    assert_eq!(form.draft.subagent_effort, None);
    form.select_model(Some("unpublished".into()));
    assert_eq!(form.draft.subagent_model_id.as_deref(), Some("published-a"));
}
#[test]
fn aggregation_t60_removed_model_keeps_intent_and_effective_state_distinct() {
    let mut form = loaded();
    let state = form.snapshot.as_mut().unwrap();
    state.choices.clear();
    state.effective.subagent_model_id = None;
    state.effective.subagent_effort = None;
    assert!(form.unavailable());
    assert!(form.selected().is_none());
    assert_eq!(form.draft.subagent_model_id.as_deref(), Some("published-a"));
    let write = form.write(&context()).unwrap();
    assert_eq!(write.expected_revision, "r1");
    assert_eq!(write.requested.auto_review_model_id.as_deref(), Some("review-keep"));
    form.select_model(None);
    assert!(!form.unavailable());
    assert_eq!(form.write(&context()).unwrap().requested.subagent_effort, None);
}
#[test]
fn aggregation_t60_errors_preserve_draft_and_require_refresh() {
    use vw_gateway_client::AgentAppsSettingsError;
    for status in [401, 403, 409, 422, 501, 502, 503, 504] {
        let mut form = loaded();
        form.pending = Some(10);
        assert!(!form.finish(9, true, Ok(snapshot())));
        assert_eq!(form.pending, Some(10));
        assert!(form.finish(
            10,
            true,
            Err(AgentAppsSettingsError::Http {
                status,
                body: serde_json::json!({"secret":"never display"})
            })
        ));
        assert!(form.snapshot.is_none());
        assert!(form.write(&context()).is_none());
        assert!(!form.saved);
        assert_eq!(form.draft.subagent_model_id.as_deref(), Some("published-a"));
        assert!(!form.error.unwrap().contains("secret"));
        form.pending = Some(11);
        assert!(form.finish(11, false, Ok(snapshot())));
        assert!(form.error.is_none());
        assert!(form.write(&context()).is_some());
    }
}
#[tokio::test]
async fn aggregation_t60_form_calls_sdk_with_scope_revision_and_clear_values() {
    use std::io::{Read, Write};
    use vw_gateway_client::{GatewayAuth, GatewayClient, GatewayEndpoint};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let port = listener.local_addr().unwrap().port();
    let server = std::thread::spawn(move || {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
        let mut captured = Vec::new();
        for method in ["GET", "PUT"] {
            let mut stream = loop {
                match listener.accept() {
                    Ok((stream, _)) => break stream,
                    Err(e)
                        if e.kind() == std::io::ErrorKind::WouldBlock
                            && std::time::Instant::now() < deadline =>
                    {
                        std::thread::sleep(std::time::Duration::from_millis(10))
                    }
                    Err(e) => panic!("HTTP fixture accept: {e}"),
                }
            };
            stream.set_nonblocking(false).unwrap();
            stream.set_read_timeout(Some(std::time::Duration::from_secs(5))).unwrap();
            let mut bytes = vec![];
            let mut chunk = [0; 4096];
            loop {
                let n = stream.read(&mut chunk).unwrap();
                assert!(n > 0);
                bytes.extend_from_slice(&chunk[..n]);
                if let Some(end) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
                    let headers = String::from_utf8_lossy(&bytes[..end]).to_lowercase();
                    let len = headers
                        .lines()
                        .find_map(|l| {
                            l.strip_prefix("content-length:")
                                .map(|v| v.trim().parse::<usize>().unwrap())
                        })
                        .unwrap_or(0);
                    if bytes.len() >= end + 4 + len {
                        break;
                    }
                }
            }
            let input = String::from_utf8(bytes).unwrap();
            assert!(
                input.starts_with(&format!(
                    "{method} /v1/agent-apps/codex/aggregation/codex/settings"
                ))
            );
            assert!(input.to_lowercase().contains("authorization: bearer fixture-t60"));
            captured.push(input);
            let body = serde_json::to_string(&snapshot()).unwrap();
            write!(stream,"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(),body).unwrap();
        }
        captured
    });
    let client = GatewayClient::new(
        GatewayEndpoint::new("127.0.0.1", port)
            .with_auth(GatewayAuth { skey: Some("fixture-t60".into()) }),
    )
    .unwrap();
    let c = context();
    let mut form = Form { pending: Some(1), ..Default::default() };
    let state = execute(client.clone(), c.clone(), None).await.unwrap();
    assert!(form.finish(1, false, Ok(state)));
    form.select_model(None);
    let write = form.write(&c).unwrap();
    form.pending = Some(2);
    let state = execute(client, c, Some(write)).await.unwrap();
    assert!(form.finish(2, true, Ok(state)));
    assert!(form.saved);
    let captured = server.join().unwrap();
    assert!(captured[0].contains("targetHostId=host-t60"));
    assert!(captured[0].contains("projectId=project-t60"));
    let body: serde_json::Value =
        serde_json::from_str(captured[1].split_once("\r\n\r\n").unwrap().1).unwrap();
    assert_eq!(
        body,
        serde_json::json!({"targetHostId":"host-t60","projectId":"project-t60","expectedRevision":"r1","requested":{"subagentModelId":null,"subagentEffort":null,"autoReviewModelId":"review-keep","classicTools":false}})
    );
}
#[derive(Default)]
struct TextProbe(Vec<(String, iced::Rectangle)>);
impl iced::advanced::widget::operation::Operation for TextProbe {
    fn traverse(
        &mut self,
        children: &mut dyn FnMut(&mut dyn iced::advanced::widget::operation::Operation),
    ) {
        children(self);
    }
    fn text(
        &mut self,
        _: Option<&iced::advanced::widget::Id>,
        bounds: iced::Rectangle,
        value: &str,
    ) {
        self.0.push((value.into(), bounds));
    }
}
#[tokio::test]
async fn aggregation_t60_actual_text_rendering_in_both_themes() {
    use iced::{
        Font, Size, Theme,
        advanced::renderer::{Headless, Style},
        mouse,
        widget::Space,
    };
    use iced_runtime::user_interface::{Cache, UserInterface};
    static FONTS: std::sync::Once = std::sync::Once::new();
    FONTS.call_once(|| {
        let mut fonts = iced::advanced::graphics::text::font_system().write().unwrap();
        for font in crate::fonts::load_all() {
            fonts.load_font(font);
        }
    });
    let mut renderer =
        iced::Renderer::new(Font::with_name("Noto Sans CJK SC"), 14.0.into(), Some("tiny-skia"))
            .await
            .unwrap();
    for (theme_name, theme) in [("light", Theme::Light), ("dark", Theme::Dark)] {
        for removed in [false, true] {
            let mut form = loaded();
            if removed {
                let s = form.snapshot.as_mut().unwrap();
                s.choices.clear();
                s.effective.subagent_model_id = None;
                s.effective.subagent_effort = None;
                s.notices.push(AggregationNotice::MultiAgentV2Forced);
                s.notices.push(AggregationNotice::EncryptedTaskCrossProvider);
            }
            let element = crate::app::components::aggregation_subagent::overlay(
                &form,
                Space::new().width(iced::Length::Fill).height(iced::Length::Fill).into(),
            );
            let size = Size::new(960, 800);
            let mut ui =
                UserInterface::build(element, Size::new(960.0, 800.0), Cache::new(), &mut renderer);
            let mut probe = TextProbe::default();
            ui.operate(&renderer, &mut probe);
            for label in ["Codex 子代理设置", "保存", "取消"] {
                let (_, bounds) = probe.0.iter().find(|(t, _)| t == label).expect(label);
                assert!(
                    bounds.x >= 0.0
                        && bounds.y >= 0.0
                        && bounds.x + bounds.width <= 960.0
                        && bounds.y + bounds.height <= 800.0
                );
            }
            assert!(probe.0.iter().any(|(t, _)| t.contains("新建 Codex 会话")));
            if removed {
                assert!(probe.0.iter().any(|(t, _)| t.contains("所选模型已不在发布目录")));
            }
            let mut messages = Vec::new();
            ui.update(
                &[iced::Event::Window(iced::window::Event::RedrawRequested(
                    iced::time::Instant::now(),
                ))],
                mouse::Cursor::Unavailable,
                &mut renderer,
                &mut iced::advanced::clipboard::Null,
                &mut messages,
            );
            messages.retain(|message| !matches!(message, crate::app::Message::None));
            assert!(messages.is_empty());
            ui.draw(
                &mut renderer,
                &theme,
                &Style { text_color: theme.palette().text },
                mouse::Cursor::Unavailable,
            );
            let pixels = renderer.screenshot(size, 1.0, theme.palette().background);
            assert_eq!(pixels.len(), 960 * 800 * 4);
            assert!(pixels.chunks_exact(4).any(|p| p[0] != p[1] || p[1] != p[2]));
            let save = probe.0.iter().find(|(t, _)| t == "保存").unwrap().1.center();
            let mut clicked = Vec::new();
            ui.update(
                &[
                    iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
                    iced::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
                ],
                mouse::Cursor::Available(save),
                &mut renderer,
                &mut iced::advanced::clipboard::Null,
                &mut clicked,
            );
            assert!(clicked.iter().any(|m| matches!(
                m,
                crate::app::Message::Settings(crate::app::message::SettingsMessage::AppProviders(
                    crate::app::message::app_providers::list::ListMessage::CodexSubagent(
                        Event::Save
                    )
                ))
            )));
            if let Some(directory) = std::env::var_os("SHIJU_T60_PREVIEW_DIR") {
                image::save_buffer(
                    std::path::PathBuf::from(directory)
                        .join(format!("t60-{theme_name}-removed-{removed}.png")),
                    &pixels,
                    960,
                    800,
                    image::ColorType::Rgba8,
                )
                .unwrap();
            }
        }
    }
}
