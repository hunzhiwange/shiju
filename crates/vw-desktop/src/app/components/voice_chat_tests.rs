use super::*;
#[derive(Default)]
struct Labels(Vec<(String, iced::Rectangle)>);
impl iced::advanced::widget::Operation for Labels {
    fn traverse(&mut self, children: &mut dyn FnMut(&mut dyn iced::advanced::widget::Operation)) {
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
async fn voice_task_11_chat_renders_chinese_in_both_themes_and_routes_controls() {
    use iced::advanced::{clipboard, renderer::Headless};
    use iced::{Event as InputEvent, Font, Size, Theme, mouse};
    use iced_runtime::user_interface::{Cache, UserInterface};
    {
        let mut fonts = iced::advanced::graphics::text::font_system().write().unwrap();
        for font in crate::fonts::load_all() {
            fonts.load_font(font);
        }
    }
    let mut renderer =
        iced::Renderer::new(Font::DEFAULT, 14.0.into(), Some("tiny-skia")).await.unwrap();
    for phase in [Phase::Speaking, Phase::Muted, Phase::Error] {
        let state = Snapshot {
            phase,
            microphone: phase != Phase::Error,
            sending: false,
            playing: phase != Phase::Error,
            error: Some("设备状态说明：请检查麦克风。"),
            ..Default::default()
        };
        let mut ui = UserInterface::build(
            overlay(
                container(text("底层聊天")).width(Length::Fill).height(Length::Fill).into(),
                state,
            ),
            Size::new(700.0, 640.0),
            Cache::new(),
            &mut renderer,
        );
        let mut labels = Labels::default();
        ui.operate(&renderer, &mut labels);
        assert!(labels.0.iter().any(|(s, _)| s == phase_label(phase)));
        for theme in [Theme::Light, Theme::Dark] {
            ui.draw(&mut renderer, &theme, &Default::default(), mouse::Cursor::Unavailable);
        }
        for (label, action) in [
            (if phase == Phase::Muted { "取消静音" } else { "静音" }, 0),
            ("打断回答", 1),
            ("结束通话", 2),
        ] {
            let bounds = labels.0.iter().find(|(s, _)| s == label).unwrap().1;
            assert!(
                bounds.x >= 0.0
                    && bounds.y >= 0.0
                    && bounds.x + bounds.width <= 700.0
                    && bounds.y + bounds.height <= 640.0,
                "button outside viewport: {bounds:?}"
            );
            let mut messages = vec![];
            ui.update(
                &[
                    InputEvent::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
                    InputEvent::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
                ],
                mouse::Cursor::Available(bounds.center()),
                &mut renderer,
                &mut clipboard::Null,
                &mut messages,
            );
            if phase == Phase::Error && action != 2 {
                assert!(messages.is_empty());
                continue;
            }
            assert_eq!(messages.len(), 1, "phase={phase:?} action={action} bounds={bounds:?}");
            assert!(matches!(
                (&messages[0], action),
                (Message::Voice(Event::ChatMute), 0)
                    | (Message::Voice(Event::ChatInterrupt), 1)
                    | (Message::Voice(Event::ChatEnd), 2)
            ));
        }
    }
}

#[tokio::test]
async fn voice_task_17_screen_consent_renders_chinese_and_routes_buttons() {
    {
        let mut fonts = iced::advanced::graphics::text::font_system().write().unwrap();
        for font in crate::fonts::load_all() {
            fonts.load_font(font);
        }
    }
    use iced::advanced::{clipboard, renderer::Headless};
    use iced::{Event as InputEvent, Font, Size, Theme, mouse};
    use iced_runtime::user_interface::{Cache, UserInterface};
    let mut renderer =
        iced::Renderer::new(Font::DEFAULT, 14.0.into(), Some("tiny-skia")).await.unwrap();
    for local in [false, true] {
        let mut screen = crate::app::voice::screen_context::State::default();
        screen.set_enabled(true);
        screen.offered = Some(crate::app::voice::screen_context::Target {
            pid: 42,
            window: 7,
            label: "中性窗口 · 中文标题与 emoji 🌱".into(),
        });
        let (mut app, _) = App::new();
        app.project_path = Some("/repo".into());
        app.active_session_id = Some("local-screen-ui".into());
        app.current_session_runtime_mut().acp_agent = None;
        let _ = crate::app::message::voice::update(&mut app, Event::ChatStart);
        app.voice.screen = screen;
        let base = container(text("聊天")).width(Length::Fill).height(Length::Fill).into();
        let element = if local {
            assert!(app.voice.local_conversation.is_some());
            local_inline(&app).unwrap()
        } else {
            overlay_screen(
                base,
                Snapshot { phase: Phase::Listening, ..Default::default() },
                Some(&app.voice.screen),
            )
        };
        let mut ui =
            UserInterface::build(element, Size::new(700.0, 840.0), Cache::new(), &mut renderer);
        let mut labels = Labels::default();
        ui.operate(&renderer, &mut labels);
        assert!(labels.0.iter().any(|(label, _)| label.contains("中性窗口")));
        for theme in [Theme::Light, Theme::Dark] {
            ui.draw(&mut renderer, &theme, &Default::default(), mouse::Cursor::Unavailable);
        }
        for (label, action) in
            [("确认采集并分享这一帧", 0), ("取消分享", 1), ("改用当前聊天附件分享", 2)]
        {
            let bounds = labels.0.iter().find(|(s, _)| s == label).unwrap().1;
            assert!(bounds.y >= 0.0 && bounds.y + bounds.height <= 840.0, "{label}: {bounds:?}");
            let mut messages = vec![];
            ui.update(
                &[
                    InputEvent::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
                    InputEvent::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
                ],
                mouse::Cursor::Available(bounds.center()),
                &mut renderer,
                &mut clipboard::Null,
                &mut messages,
            );
            assert!(matches!(
                (&messages[..], action),
                ([Message::Voice(Event::Screen(ScreenEvent::Confirm))], 0)
                    | ([Message::Voice(Event::Screen(ScreenEvent::Cancel))], 1)
                    | ([Message::Voice(Event::Screen(ScreenEvent::AttachmentChat))], 2)
            ));
        }
    }
}

#[tokio::test]
async fn voice_task_11_local_overlay_shapes_chinese_in_both_themes() {
    use iced::advanced::renderer::Headless;
    use iced_runtime::user_interface::{Cache, UserInterface};
    {
        let mut fonts = iced::advanced::graphics::text::font_system().write().unwrap();
        for font in crate::fonts::load_all() {
            fonts.load_font(font);
        }
    }
    let (mut app, _) = App::new();
    app.project_path = Some("/repo".into());
    app.active_session_id = Some("local-ui".into());
    app.current_session_runtime_mut().acp_agent = None;
    let _ = crate::app::message::voice::update(&mut app, Event::ChatStart);
    let chat = app.voice.local_conversation.as_mut().unwrap();
    chat.captions.push((true, "中文正文、非 ASCII 🌱。".into()));
    let mut renderer =
        iced::Renderer::new(iced::Font::DEFAULT, 14.0.into(), Some("tiny-skia")).await.unwrap();
    let mut ui = UserInterface::build(
        local_inline(&app).unwrap(),
        iced::Size::new(700.0, 640.0),
        Cache::new(),
        &mut renderer,
    );
    let mut labels = Labels::default();
    ui.operate(&renderer, &mut labels);
    for expected in ["点击开始录音", "静音", "打断本轮", "×"] {
        let bounds = labels.0.iter().find(|(s, _)| s == expected).unwrap().1;
        assert!(bounds.x >= 0.0 && bounds.y >= 0.0 && bounds.y + bounds.height <= 640.0);
    }
    for theme in [iced::Theme::Light, iced::Theme::Dark] {
        ui.draw(&mut renderer, &theme, &Default::default(), iced::mouse::Cursor::Unavailable);
    }
}

#[tokio::test]
async fn voice_moon_ui_local_keeps_chat_without_modal() {
    use iced::advanced::renderer::Headless;
    use iced_runtime::user_interface::{Cache, UserInterface};
    let (mut app, _) = App::new();
    app.project_path = Some("/repo".into());
    app.active_session_id = Some("moon-ui".into());
    app.current_session_runtime_mut().acp_agent = None;
    let _ = crate::app::message::voice::update(&mut app, Event::ChatStart);
    let mut renderer =
        iced::Renderer::new(iced::Font::DEFAULT, 14.0.into(), Some("tiny-skia")).await.unwrap();
    let mut ui = UserInterface::build(
        with_voice_chat(&app, text("聊天正文").into()),
        iced::Size::new(700.0, 640.0),
        Cache::new(),
        &mut renderer,
    );
    let mut labels = Labels::default();
    ui.operate(&renderer, &mut labels);
    assert!(labels.0.iter().any(|(s, _)| s == "聊天正文"));
    assert!(!labels.0.iter().any(|(s, _)| s == "本机逐轮语音"));
}

#[tokio::test]
async fn voice_moon_ui_inline_draws_and_routes_controls() {
    use iced::advanced::{clipboard, renderer::Headless};
    use iced::{Event as InputEvent, Font, Size, Theme, mouse};
    use iced_runtime::user_interface::{Cache, UserInterface};
    for font in crate::fonts::load_all() {
        iced::advanced::graphics::text::font_system().write().unwrap().load_font(font);
    }
    let mut renderer =
        iced::Renderer::new(Font::DEFAULT, 14.0.into(), Some("tiny-skia")).await.unwrap();
    let (mut app, _) = App::new();
    app.project_path = Some("/repo".into());
    app.active_session_id = Some("moon-controls".into());
    app.current_session_runtime_mut().acp_agent = None;
    let _ = crate::app::message::voice::update(&mut app, Event::ChatStart);
    use crate::app::message::voice::local_chat::Stage;
    for stage in [Stage::Ready, Stage::Preparing, Stage::Recording, Stage::Speaking] {
        app.voice.local_conversation.as_mut().unwrap().stage = stage;
        for size in [Size::new(700.0, 640.0), Size::new(1280.0, 800.0)] {
            let mut ui = UserInterface::build(
                iced::widget::column![local_inline(&app).unwrap(), text("输入框")],
                size,
                Cache::new(),
                &mut renderer,
            );
            let mut labels = Labels::default();
            ui.operate(&renderer, &mut labels);
            for theme in [Theme::Light, Theme::Dark] {
                ui.draw(
                    &mut renderer,
                    &theme,
                    &iced::advanced::renderer::Style { text_color: theme.palette().text },
                    mouse::Cursor::Unavailable,
                );
                let pixels = renderer.screenshot(
                    Size::new(size.width as u32, size.height as u32),
                    1.0,
                    theme.palette().background,
                );
                if let Some(directory) = std::env::var_os("SHIJU_MOON_SCREENSHOTS") {
                    ::image::save_buffer(
                        std::path::PathBuf::from(directory)
                            .join(format!("{stage:?}-{theme:?}-{}.png", size.width)),
                        &pixels,
                        size.width as u32,
                        size.height as u32,
                        ::image::ColorType::Rgba8,
                    )
                    .unwrap();
                }
                assert!(
                    pixels.chunks_exact(4).any(|p| p[2] > 180 && p[2] > p[0].saturating_add(30)),
                    "blue moon must be painted"
                );
            }
            for (label, expected) in [("静音", 0), ("打断本轮", 1), ("×", 2)] {
                let bounds = labels.0.iter().find(|(s, _)| s == label).unwrap().1;
                assert!(bounds.y + bounds.height < size.height);
                let mut messages = vec![];
                ui.update(
                    &[
                        InputEvent::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
                        InputEvent::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
                    ],
                    mouse::Cursor::Available(bounds.center()),
                    &mut renderer,
                    &mut clipboard::Null,
                    &mut messages,
                );
                assert!(matches!(
                    (&messages[..], expected),
                    ([Message::Voice(Event::ChatMute)], 0)
                        | ([Message::Voice(Event::ChatInterrupt)], 1)
                        | ([Message::Voice(Event::ChatEnd)], 2)
                ));
            }
            let mut messages = vec![];
            ui.update(
                &[
                    InputEvent::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
                    InputEvent::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
                ],
                mouse::Cursor::Available(iced::Point::new(size.width / 2.0, 48.0)),
                &mut renderer,
                &mut clipboard::Null,
                &mut messages,
            );
            use crate::app::message::voice::local_chat::Event as Local;
            assert!(matches!(
                (&messages[..], stage),
                ([Message::Voice(Event::Local(Local::Begin))], Stage::Ready)
                    | ([Message::Voice(Event::Local(Local::Finish))], Stage::Recording)
                    | ([Message::Voice(Event::ChatInterrupt)], Stage::Preparing)
                    | ([], Stage::Speaking)
            ));
        }
    }
}

#[tokio::test]
async fn voice_task_17_permission_action_is_visible_before_screen_grant() {
    use iced::advanced::renderer::Headless;
    use iced_runtime::user_interface::{Cache, UserInterface};
    let screen = crate::app::voice::screen_context::State::default();
    let mut renderer =
        iced::Renderer::new(iced::Font::DEFAULT, 14.0.into(), Some("tiny-skia")).await.unwrap();
    let mut ui = UserInterface::build(
        screen_controls(&screen, true, false, true),
        iced::Size::new(700.0, 640.0),
        Cache::new(),
        &mut renderer,
    );
    let mut labels = Labels::default();
    ui.operate(&renderer, &mut labels);
    assert!(labels.0.iter().any(|(label, _)| label == "屏幕录制授权"));
    assert!(!labels.0.iter().any(|(label, _)| label == "确认采集并分享这一帧"));
}
