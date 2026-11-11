use super::*;
use crate::app::message::settings::aggregation_list::clients::{Action, Event, Session};
use vw_gateway_client::vw_api_types::agent_apps::aggregation::CodexClientsState;
#[derive(Default)]
struct Probe(Vec<(String, iced::Rectangle)>);
impl iced::advanced::widget::operation::Operation for Probe {
    fn traverse(
        &mut self,
        f: &mut dyn FnMut(&mut dyn iced::advanced::widget::operation::Operation),
    ) {
        f(self);
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
async fn aggregation_t62_real_draw_and_click_in_both_themes() {
    use iced::{
        Font, Size, Theme,
        advanced::renderer::{Headless, Style},
        mouse,
    };
    use iced_runtime::user_interface::{Cache, UserInterface};
    {
        let mut fonts = iced::advanced::graphics::text::font_system().write().unwrap();
        for f in crate::fonts::load_all() {
            fonts.load_font(f);
        }
    }
    let mut renderer =
        iced::Renderer::new(Font::with_name("Noto Sans CJK SC"), 14.0.into(), Some("tiny-skia"))
            .await
            .unwrap();
    for (name, theme) in [("light", Theme::Light), ("dark", Theme::Dark)] {
        for (mode, daemon, others, unverified, confirming) in [
            ("daemon", true, false, false, false),
            ("desktop-editor", false, true, false, false),
            ("unknown", false, true, true, false),
            ("confirm", true, true, true, true),
        ] {
            let s = Session {
                snapshot: Some(CodexClientsState {
                    revision: "r62".into(),
                    daemon,
                    others,
                    auth: true,
                    unverified,
                }),
                confirming,
                ..Default::default()
            };
            let base = iced::widget::Space::new()
                .width(iced::Length::Fill)
                .height(iced::Length::Fill)
                .into();
            let mut ui = UserInterface::build(
                overlay(&s, base),
                Size::new(960.0, 800.0),
                Cache::new(),
                &mut renderer,
            );
            let mut probe = Probe::default();
            ui.operate(&renderer, &mut probe);
            for label in ["Codex 客户端状态", "重新探测", "返回目录"] {
                let b = probe.0.iter().find(|(t, _)| t == label).unwrap().1;
                assert!(
                    b.x >= 0.0 && b.y >= 0.0 && b.x + b.width <= 960.0 && b.y + b.height <= 800.0
                );
            }
            if others {
                assert!(probe.0.iter().any(|(t, _)| t.contains("完全退出编辑器")));
            }
            if unverified {
                assert!(probe.0.iter().any(|(t, _)| t.contains("未知探测不代表已刷新")));
            }
            if confirming {
                let b = probe
                    .0
                    .iter()
                    .find(|(t, _)| t.contains("中断守护进程中正在运行的任务"))
                    .unwrap()
                    .1;
                assert!(
                    b.y >= 0.0 && b.y + b.height < 650.0,
                    "interruption impact must be visible before confirmation"
                );
            }
            if !daemon {
                assert!(!probe.0.iter().any(|(t, _)| t == "重启守护进程" || t == "确认重启"));
            }
            ui.update(
                &[iced::Event::Window(iced::window::Event::RedrawRequested(
                    iced::time::Instant::now(),
                ))],
                mouse::Cursor::Unavailable,
                &mut renderer,
                &mut iced::advanced::clipboard::Null,
                &mut Vec::new(),
            );
            ui.draw(
                &mut renderer,
                &theme,
                &Style { text_color: theme.palette().text },
                mouse::Cursor::Unavailable,
            );
            let pixels = renderer.screenshot(Size::new(960, 800), 1.0, theme.palette().background);
            assert_eq!(pixels.len(), 960 * 800 * 4);
            if let Some(dir) = std::env::var_os("SHIJU_T62_PREVIEW_DIR") {
                std::fs::create_dir_all(&dir).unwrap();
                image::save_buffer(
                    std::path::PathBuf::from(dir).join(format!("t62-{name}-{mode}.png")),
                    &pixels,
                    960,
                    800,
                    image::ColorType::Rgba8,
                )
                .unwrap();
            }
            let label = if confirming {
                "确认重启"
            } else if daemon {
                "重启守护进程"
            } else if unverified {
                "确认提醒"
            } else {
                "重新探测"
            };
            let point = probe.0.iter().find(|(t, _)| t == label).unwrap().1.center();
            let mut messages = vec![];
            ui.update(
                &[
                    iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
                    iced::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
                ],
                mouse::Cursor::Available(point),
                &mut renderer,
                &mut iced::advanced::clipboard::Null,
                &mut messages,
            );
            assert!(messages.iter().any(|m| {
                let Message::Settings(crate::app::message::SettingsMessage::AppProviders(
                    crate::app::message::app_providers::list::ListMessage::Aggregation(
                        crate::app::message::settings::aggregation_list::Event::Clients(e),
                    ),
                )) = m
                else {
                    return false;
                };
                matches!(
                    (label, e),
                    ("确认重启", Event::Act(Action::Restart))
                        | ("重启守护进程", Event::AskRestart)
                        | ("确认提醒", Event::Act(Action::Ack))
                        | ("重新探测", Event::Act(Action::Read))
                )
            }));
        }
    }
}
