use crate::app::state::aggregation_omo::*;
use vw_gateway_client::{AggregationOmoError, vw_api_types::agent_apps::aggregation::*};
fn standard(revision: &str, current: Option<&str>) -> Snapshot {
    Snapshot::Standard(OmoState {
        revision: revision.into(),
        current_provider_id: current.map(Into::into),
        agents: None,
        categories: Some(Default::default()),
        other_fields: None,
    })
}
#[test]
fn aggregation_t64_aggregation_omo_preserves_contract() {
    let mut form = Form::new(Variant::Standard);
    form.provider = "draft".into();
    form.finish(Ok(standard("r1", Some("p1"))));
    assert_eq!(form.snapshot.as_ref().unwrap().current(), Some("p1"));
    assert!(form.snapshot.as_ref().unwrap().sections().iter().any(|(s, _)| *s == "Categories"));
    form.finish(Err(AggregationOmoError::Http {
        status: 409,
        body: serde_json::json!({"error":{"code":"revision_conflict"}}),
    }));
    assert!(form.snapshot.is_none(), "stale revision must not enable another write");
    assert_eq!(form.provider, "draft");
    assert!(!form.busy);
    form.finish(Ok(standard("r2", None)));
    assert_eq!(form.snapshot.as_ref().unwrap().revision(), "r2");
    assert_eq!(form.snapshot.as_ref().unwrap().current(), None);
    assert!(form.error.is_none());
}
#[test]
fn aggregation_t64_slim_has_no_categories_and_jsonc_error_is_retryable() {
    let mut form = Form::new(Variant::Slim);
    form.finish(Ok(Snapshot::Slim(OmoSlimState {
        revision: "s1".into(),
        current_provider_id: None,
        agents: Some(Default::default()),
        other_fields: None,
    })));
    assert!(!form.snapshot.as_ref().unwrap().sections().iter().any(|(s, _)| *s == "Categories"));
    form.finish(Err(AggregationOmoError::Http {
        status: 422,
        body: serde_json::json!({"secret":"must never display"}),
    }));
    assert!(!form.busy);
    assert!(!form.error.as_ref().unwrap().contains("secret"));
    assert!(form.error.as_ref().unwrap().contains("刷新"));
}
#[tokio::test]
async fn aggregation_t64_actual_text_rendering_in_both_themes() {
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
        for variant in [Variant::Standard, Variant::Slim] {
            let mut form = Form::new(variant);
            form.busy = false;
            form.provider = "测试供应商／日本語".into();
            form.snapshot = Some(match variant {
                Variant::Standard => standard("r1", Some("当前供应商")),
                Variant::Slim => Snapshot::Slim(OmoSlimState {
                    revision: "s1".into(),
                    current_provider_id: Some("当前供应商".into()),
                    agents: Some(
                        serde_json::from_value(serde_json::json!({"助手":{"model":"示例模型"}}))
                            .unwrap(),
                    ),
                    other_fields: None,
                }),
            });
            let element = crate::app::components::aggregation_omo::overlay(
                &form,
                Space::new().width(iced::Length::Fill).height(iced::Length::Fill).into(),
            );
            let size = Size::new(960, 800);
            let mut ui =
                UserInterface::build(element, Size::new(960.0, 800.0), Cache::new(), &mut renderer);
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
            if let Some(directory) = std::env::var_os("SHIJU_T64_PREVIEW_DIR") {
                image::save_buffer(
                    std::path::PathBuf::from(directory)
                        .join(format!("t64-{theme_name}-{variant:?}.png")),
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
#[tokio::test]
async fn aggregation_t64_sdk_uses_variant_scope_revision_and_provider_reference() {
    use super::{Action, Context, request};
    use crate::app::state::{AgentAppsContext, app_providers::list::Scope};
    use std::io::{Read, Write};
    use vw_gateway_client::{
        GatewayAuth, GatewayClient, GatewayEndpoint, vw_api_types::agent_apps::AppId,
    };
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let worker = std::thread::spawn(move || {
        let mut captured = vec![];
        for path in
            ["omo", "omo/apply", "omo/disable", "omo-slim", "omo-slim/apply", "omo-slim/disable"]
        {
            let (mut stream, _) = listener.accept().unwrap();
            stream.set_read_timeout(Some(std::time::Duration::from_secs(10))).unwrap();
            let mut bytes = vec![];
            let mut chunk = [0; 4096];
            loop {
                let n = stream.read(&mut chunk).unwrap();
                assert!(n > 0);
                bytes.extend_from_slice(&chunk[..n]);
                if let Some(end) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
                    let headers = String::from_utf8_lossy(&bytes[..end]).to_lowercase();
                    let length = headers
                        .lines()
                        .find_map(|l| {
                            l.strip_prefix("content-length:")
                                .map(|v| v.trim().parse::<usize>().unwrap())
                        })
                        .unwrap_or(0);
                    if bytes.len() >= end + 4 + length {
                        break;
                    }
                }
            }
            let input = String::from_utf8(bytes).unwrap();
            assert!(
                input.lines().next().unwrap().contains(&format!("/v1/agent-apps/opencode/{path}"))
            );
            assert!(input.to_lowercase().contains("authorization: bearer fixture-t64"));
            captured.push(input);
            let result = if path.ends_with("disable") {
                serde_json::json!({"revision":"r3","currentProviderId":null})
            } else {
                serde_json::json!({"revision":"r2","currentProviderId":"saved"})
            };
            let body = result.to_string();
            write!(stream,"HTTP/1.1 200 OK\r\nContent-Length: {}\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n{}",body.len(),body).unwrap();
        }
        captured
    });
    let client = GatewayClient::new(
        GatewayEndpoint::new("127.0.0.1", port)
            .with_auth(GatewayAuth { skey: Some("fixture-t64".into()) }),
    )
    .unwrap();
    let context = Context {
        scope: Scope {
            context: AgentAppsContext {
                gateway: client.endpoint().base_url(),
                tenant: Default::default(),
                project: Some("/fixture".into()),
            },
            host: "host-fixture".into(),
            app: AppId::OpenCode,
        },
        project: "project-fixture".into(),
        access: 1,
    };
    for variant in [Variant::Standard, Variant::Slim] {
        for action in [Action::Read, Action::Apply("saved".into()), Action::Disable] {
            let disabled = action == Action::Disable;
            let value = request(client.clone(), context.clone(), variant, action, "r1".into())
                .await
                .unwrap();
            assert_eq!(value.current(), if disabled { None } else { Some("saved") });
        }
    }
    for (n, input) in worker.join().unwrap().iter().enumerate() {
        if n % 3 == 0 {
            assert!(input.contains("projectId=project-fixture"));
            assert!(input.contains("targetHostId=host-fixture"));
        } else {
            let body: serde_json::Value =
                serde_json::from_str(input.split_once("\r\n\r\n").unwrap().1).unwrap();
            assert_eq!(body["projectId"], "project-fixture");
            assert_eq!(body["targetHostId"], "host-fixture");
            assert_eq!(body["expectedRevision"], "r1");
            assert_eq!(
                body.get("providerId").and_then(|v| v.as_str()),
                if n % 3 == 1 { Some("saved") } else { None }
            );
            assert!(body.get("categories").is_none());
            assert!(body.get("agents").is_none());
        }
    }
}
