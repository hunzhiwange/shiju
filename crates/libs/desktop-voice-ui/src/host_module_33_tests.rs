use crate::components::voice_chat::{self as view, LocalView, Stage};
use crate::message::voice::Event;
use crate::message::voice_screen::ScreenEvent;
use desktop_voice::conversation::{Phase, Snapshot};
use iced::{
    Element, Length,
    widget::{container, text},
};
fn local_inline(state: &crate::message::voice::State) -> Option<Element<'_, Event>> {
    let chat = state.local_conversation.as_ref()?;
    Some(
        view::local_inline(desktop_i18n::I18n::new(config_ui::DesktopLocale::default()),
            LocalView {
                stage: chat.stage,
                muted: chat.muted,
                notice: chat.notice.as_deref(),
            },
            &state.screen,
        )
        .map(view::map),
    )
}
fn state() -> crate::message::voice::State {
    let mut state = crate::message::voice::State::default();
    state.local_conversation =
        Some(crate::message::voice_local_chat::new(Some("local-ui".into()), Default::default()));
    state
}
fn overlay_screen<'a>(
    base: Element<'a, Event>,
    snapshot: Snapshot,
    screen: Option<&'a desktop_voice::screen_context::State>,
) -> Element<'a, Event> {
    view::overlay_screen(desktop_i18n::I18n::new(config_ui::DesktopLocale::default()), base, view::map, snapshot, screen)
}

#[derive(Default)]
struct Labels(
    Vec<(String, iced::Rectangle)>,
    Vec<(iced::widget::Id, iced::Rectangle)>,
);
impl Labels {
    fn bounds(&self, id: &'static str) -> iced::Rectangle {
        let id = iced::widget::Id::new(id);
        self.1.iter().find(|(existing, _)| *existing == id).unwrap().1
    }
}
impl iced::advanced::widget::Operation for Labels {
    fn traverse(&mut self, children: &mut dyn FnMut(&mut dyn iced::advanced::widget::Operation)) {
        children(self);
    }
    fn container(&mut self, id: Option<&iced::widget::Id>, bounds: iced::Rectangle) {
        if let Some(id) = id {
            self.1.push((id.clone(), bounds));
        }
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
async fn voice_task_17_screen_consent_renders_chinese_and_routes_buttons() {
    {
        let mut fonts = iced::advanced::graphics::text::font_system().write().unwrap();
        for font in desktop_assets::fonts::load_all() {
            fonts.load_font(font);
        }
    }
    use iced::advanced::{clipboard, renderer::Headless};
    use iced::{Event as InputEvent, Font, Size, Theme, mouse};
    use iced_runtime::user_interface::{Cache, UserInterface};
    let mut renderer =
        iced::Renderer::new(Font::DEFAULT, 14.0.into(), Some("tiny-skia")).await.unwrap();
    for local in [false, true] {
        let mut screen = desktop_voice::screen_context::State::default();
        screen.set_enabled(true);
        screen.offered = Some(desktop_voice::screen_context::Target {
            application: None, title: None,
            pid: 42,
            window: 7,
            label: "中性窗口 · 中文标题与 emoji 🌱".into(),
        });
        let mut voice = state();
        voice.screen = screen;
        let base = container(text("聊天")).width(Length::Fill).height(Length::Fill).into();
        let element = if local {
            assert!(voice.local_conversation.is_some());
            local_inline(&voice).unwrap()
        } else {
            overlay_screen(
                base,
                Snapshot { phase: Phase::Listening, ..Default::default() },
                Some(&voice.screen),
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
            [("确认采集并分享这一帧", 0), ("取消分享", 1)]
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
                ([Event::Screen(ScreenEvent::Confirm)], 0)
                    | ([Event::Screen(ScreenEvent::Cancel)], 1)
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
        for font in desktop_assets::fonts::load_all() {
            fonts.load_font(font);
        }
    }
    let mut voice = state();
    let chat = voice.local_conversation.as_mut().unwrap();
    chat.notice = Some("中文提示、非 ASCII 🌱。".into());
    let mut renderer =
        iced::Renderer::new(iced::Font::DEFAULT, 14.0.into(), Some("tiny-skia")).await.unwrap();
    let mut ui = UserInterface::build(
        local_inline(&voice).unwrap(),
        iced::Size::new(700.0, 640.0),
        Cache::new(),
        &mut renderer,
    );
    let mut labels = Labels::default();
    ui.operate(&renderer, &mut labels);
    assert!(labels.0.iter().any(|(label, _)| label == "中文提示、非 ASCII 🌱。"));
    for label in ["点击开始录音", "静音", "打断本轮", "×", "屏幕录制授权"] {
        assert!(!labels.0.iter().any(|(text, _)| text == label));
    }
    let first = labels.bounds("voice-chat-mute");
    let last = labels.bounds("voice-chat-settings");
    let midpoint = (first.x + last.x + last.width) / 2.0;
    assert!((midpoint - 350.0).abs() < 1.0);
    assert_eq!(labels.bounds("voice-chat-end").size(), iced::Size::new(32.0, 32.0));
    for theme in [iced::Theme::Light, iced::Theme::Dark] {
        ui.draw(&mut renderer, &theme, &Default::default(), iced::mouse::Cursor::Unavailable);
    }
}

#[tokio::test]
async fn voice_moon_ui_inline_draws_and_routes_controls() {
    use iced::advanced::{clipboard, renderer::Headless};
    use iced::{Event as InputEvent, Font, Size, Theme, mouse};
    use iced_runtime::user_interface::{Cache, UserInterface};
    for font in desktop_assets::fonts::load_all() {
        iced::advanced::graphics::text::font_system().write().unwrap().load_font(font);
    }
    let mut renderer =
        iced::Renderer::new(Font::DEFAULT, 14.0.into(), Some("tiny-skia")).await.unwrap();
    let mut voice = state();

    for stage in [Stage::Ready, Stage::Preparing, Stage::Recording, Stage::Speaking] {
        voice.local_conversation.as_mut().unwrap().stage = stage;
        for size in [Size::new(700.0, 640.0), Size::new(1280.0, 800.0)] {
            let mut ui = UserInterface::build(
                iced::widget::column![local_inline(&voice).unwrap(), text("输入框")],
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
                    pixels.as_chunks::<4>().0.iter().any(|p| p[2] > 180 && p[2] > p[0].saturating_add(30)),
                    "blue moon must be painted"
                );
            }
            for (id, expected) in [
                ("voice-chat-mute", 0),
                ("voice-chat-interrupt", 1),
                ("voice-chat-end", 2),
            ] {
                let bounds = labels.bounds(id);
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
                    ([Event::ChatMute], 0) | ([Event::ChatInterrupt], 1) | ([Event::ChatEnd], 2)
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
            use crate::message::voice_local_chat::Event as Local;
            assert!(matches!(
                (&messages[..], stage),
                ([Event::Local(Local::Begin)], Stage::Ready)
                    | ([Event::Local(Local::Finish)], Stage::Recording)
                    | ([Event::ChatInterrupt], Stage::Preparing)
                    | ([], Stage::Speaking)
            ));
        }
    }
}

mod hotkey_edit {
    use desktop_voice::hotkeys::*;

    #[test]
    fn voice_task_15_recording_chords_stays_in_edit_until_save_and_cancel_discards() {
        use desktop_settings_voice::{self as voice, Event};
        use desktop_voice::preferences::{Edit, Preferences};
        let mut state = voice::State::default();
        let mut hotkeys = desktop_voice::hotkeys::Runtime::default();
        state.edit = Some(Edit::new(Default::default(), Preferences::default()));
        let _ = voice::update(
            &mut state,
            &mut hotkeys,
            false,
            gateway_endpoint::GatewayEndpoint::new("127.0.0.1", 1),
            Err("fixture path unavailable".into()),
            Event::RecordHotkey(Some(false)),
        );
        let shortcut = Shortcut { key: 0, modifiers: CONTROL | ALT };
        let _ = voice::update(
            &mut state,
            &mut hotkeys,
            false,
            gateway_endpoint::GatewayEndpoint::new("127.0.0.1", 1),
            Err("fixture path unavailable".into()),
            Event::RecordedHotkey(shortcut),
        );
        assert_eq!(state.edit.as_ref().unwrap().local.hotkeys.dictation, Some(shortcut));
        assert!(!state.edit.as_ref().unwrap().local.hotkeys.enabled);
        assert_eq!(state.recording_hotkey, None);
        let _ = voice::update(
            &mut state,
            &mut hotkeys,
            false,
            gateway_endpoint::GatewayEndpoint::new("127.0.0.1", 1),
            Err("fixture path unavailable".into()),
            Event::RecordHotkey(Some(true)),
        );
        let _ = voice::update(
            &mut state,
            &mut hotkeys,
            false,
            gateway_endpoint::GatewayEndpoint::new("127.0.0.1", 1),
            Err("fixture path unavailable".into()),
            Event::RecordHotkey(None),
        );
        assert_eq!(state.edit.as_ref().unwrap().local.hotkeys.chat, None);
        let _ = voice::update(
            &mut state,
            &mut hotkeys,
            false,
            gateway_endpoint::GatewayEndpoint::new("127.0.0.1", 1),
            Err("fixture path unavailable".into()),
            Event::Cancel,
        );
        assert!(state.edit.is_none());
    }
}
