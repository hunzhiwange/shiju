use super::{SystemTab, system_tab_matches_query};

#[test]
fn archived_chats_browse_navigation_is_searchable() {
    let tab =
        SystemTab::all().find(|tab| tab.to_string() == "已归档的聊天").expect("archive navigation");
    assert!(system_tab_matches_query(tab, "archived"));
    assert!(system_tab_matches_query(tab, "归档"));
}

#[cfg(not(target_arch = "wasm32"))]
mod rendering {
    use crate::app::Message;
    use crate::app::components::system_settings_archived_chats::page;
    use crate::app::i18n::I18n;
    use crate::app::message::settings::{
        SettingsMessage,
        archived_chats::{ArchivedChatsMessage, Identity, apply},
    };
    use crate::app::state::{ArchivedChatsMenu, ArchivedChatsSort, ArchivedChatsState};
    use iced::advanced::{
        clipboard,
        renderer::{Headless, Style},
        widget::Operation,
    };
    use iced::{Event, Font, Point, Rectangle, Renderer, Size, Theme, mouse};
    use iced_runtime::user_interface::{Cache, UserInterface};
    use vw_config_types::ui::DesktopLocale;

    #[derive(Default)]
    struct Geometry {
        labels: Vec<(String, Rectangle)>,
        inputs: Vec<Rectangle>,
        scrolls: Vec<Rectangle>,
    }

    impl Operation for Geometry {
        fn traverse(&mut self, children: &mut dyn FnMut(&mut dyn Operation)) {
            children(self);
        }
        fn text(&mut self, _: Option<&iced::advanced::widget::Id>, bounds: Rectangle, value: &str) {
            self.labels.push((value.to_owned(), bounds));
        }
        fn text_input(
            &mut self,
            _: Option<&iced::advanced::widget::Id>,
            bounds: Rectangle,
            _: &mut dyn iced::advanced::widget::operation::TextInput,
        ) {
            self.inputs.push(bounds);
        }
        fn scrollable(
            &mut self,
            _: Option<&iced::advanced::widget::Id>,
            bounds: Rectangle,
            _: Rectangle,
            _: iced::Vector,
            _: &mut dyn iced::advanced::widget::operation::Scrollable,
        ) {
            self.scrolls.push(bounds);
        }
    }

    fn fixture() -> ArchivedChatsState {
        let sessions = (0..120).map(|index| serde_json::from_value(serde_json::json!({
            "id": format!("fixture-{index}"), "slug": "fixture", "projectID": if index < 100 { "p" } else { "q" },
            "directory": if index < 100 { "/isolated/历史项目" } else { "/isolated/Other" },
            "title": format!("归档聊天 {index} {}", "很长的标题".repeat(40)), "version": "1",
            "time": {"created": index, "updated": 1794350000000_i64 + index, "archived": 1}
        })).unwrap()).collect();
        ArchivedChatsState { sessions, ..Default::default() }
    }

    fn click(
        ui: &mut UserInterface<'_, Message, Theme, Renderer>,
        renderer: &mut Renderer,
        point: Point,
    ) -> Vec<Message> {
        let mut messages = Vec::new();
        ui.update(
            &[
                Event::Mouse(mouse::Event::CursorMoved { position: point }),
                Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
                Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
            ],
            mouse::Cursor::Available(point),
            renderer,
            &mut clipboard::Null,
            &mut messages,
        );
        messages
    }

    #[tokio::test]
    async fn archived_chats_browse_renders_themes_locales_states_and_interactions() {
        {
            let mut fonts = iced::advanced::graphics::text::font_system().write().unwrap();
            for font in crate::fonts::load_all() {
                fonts.load_font(font);
            }
        }
        let mut renderer =
            Renderer::new(Font::with_name("Noto Sans CJK SC"), 14.0.into(), Some("tiny-skia"))
                .await
                .unwrap();
        let mut state = fixture();
        for locale in [
            DesktopLocale::SimplifiedChinese,
            DesktopLocale::TraditionalChinese,
            DesktopLocale::English,
        ] {
            for (theme_name, theme) in [("dark", Theme::Dark), ("light", Theme::Light)] {
                for width in [320_u32, 860] {
                    for (name, menu) in [
                        ("list", None),
                        ("type", Some(ArchivedChatsMenu::Type)),
                        ("project", Some(ArchivedChatsMenu::Project)),
                    ] {
                        state.menu = menu;
                        let size = Size::new(width, 620);
                        let mut ui = UserInterface::build(
                            page(&state, locale),
                            Size::new(width as f32, 620.0),
                            Cache::new(),
                            &mut renderer,
                        );
                        ui.update(
                            &[Event::Window(iced::window::Event::RedrawRequested(
                                iced::time::Instant::now(),
                            ))],
                            mouse::Cursor::Unavailable,
                            &mut renderer,
                            &mut clipboard::Null,
                            &mut Vec::new(),
                        );
                        ui.draw(
                            &mut renderer,
                            &theme,
                            &Style { text_color: theme.palette().text },
                            mouse::Cursor::Unavailable,
                        );
                        let mut geometry = Geometry::default();
                        ui.operate(&mut renderer, &mut geometry);
                        assert_eq!(geometry.inputs.len(), 1);
                        assert_eq!(
                            geometry.scrolls.len(),
                            if menu == Some(ArchivedChatsMenu::Project) { 2 } else { 1 }
                        );
                        assert!(geometry.inputs[0].width > 100.0);
                        assert!(
                            geometry.scrolls[0].y < 200.0,
                            "toolbar must not consume list height"
                        );
                        assert!(
                            geometry
                                .labels
                                .iter()
                                .any(|(label, _)| label == I18n::new(locale).__("已归档的聊天"))
                        );
                        let pixels = renderer.screenshot(size, 1.0, theme.palette().background);
                        assert_eq!(pixels.len(), width as usize * 620 * 4);
                        if let Some(path) = std::env::var_os("SHIJU_ARCHIVED_SCREENSHOTS") {
                            image::save_buffer(
                                std::path::PathBuf::from(path)
                                    .join(format!("{locale:?}-{theme_name}-{width}-{name}.png")),
                                &pixels,
                                width,
                                620,
                                image::ColorType::Rgba8,
                            )
                            .unwrap();
                        }
                    }
                }
            }
        }
        let locale = DesktopLocale::SimplifiedChinese;
        let current =
            Identity::from_endpoint(&vw_gateway_client::GatewayEndpoint::new("127.0.0.1", 1));
        state.menu = None;
        let mut ui = UserInterface::build(
            page(&state, locale),
            Size::new(860.0, 620.0),
            Cache::new(),
            &mut renderer,
        );
        ui.update(
            &[Event::Window(iced::window::Event::RedrawRequested(iced::time::Instant::now()))],
            mouse::Cursor::Unavailable,
            &mut renderer,
            &mut clipboard::Null,
            &mut Vec::new(),
        );
        let mut geometry = Geometry::default();
        ui.operate(&mut renderer, &mut geometry);
        let bounds = geometry.labels.iter().find(|(label, _)| label == "全部聊天").unwrap().1;
        let messages = click(&mut ui, &mut renderer, bounds.center());
        drop(ui);
        assert!(messages.iter().any(|message| matches!(
            message,
            Message::Settings(SettingsMessage::ArchivedChats(
                ArchivedChatsMessage::TypeMenuToggled
            ))
        )));
        for message in messages {
            if let Message::Settings(SettingsMessage::ArchivedChats(message)) = message {
                apply(&mut state, message, &current);
            }
        }
        assert_eq!(state.menu, Some(ArchivedChatsMenu::Type));
        let mut ui = UserInterface::build(
            page(&state, locale),
            Size::new(860.0, 620.0),
            Cache::new(),
            &mut renderer,
        );
        ui.update(
            &[Event::Window(iced::window::Event::RedrawRequested(iced::time::Instant::now()))],
            mouse::Cursor::Unavailable,
            &mut renderer,
            &mut clipboard::Null,
            &mut Vec::new(),
        );
        let mut geometry = Geometry::default();
        ui.operate(&mut renderer, &mut geometry);
        let bounds = geometry
            .labels
            .iter()
            .find(|(label, _)| label == "按字母顺序")
            .expect("sort overlay")
            .1;
        let messages = click(&mut ui, &mut renderer, bounds.center());
        drop(ui);
        assert!(messages.iter().any(|message| matches!(
            message,
            Message::Settings(SettingsMessage::ArchivedChats(ArchivedChatsMessage::SortSelected(
                ArchivedChatsSort::Title
            )))
        )));
        for message in messages {
            if let Message::Settings(SettingsMessage::ArchivedChats(message)) = message {
                apply(&mut state, message, &current);
            }
        }
        assert_eq!(state.sort, ArchivedChatsSort::Title);
        assert!(state.menu.is_none());

        for (name, state, expected) in [
            (
                "loading",
                ArchivedChatsState { loading: true, ..Default::default() },
                "正在加载已归档的聊天…",
            ),
            ("empty", ArchivedChatsState::default(), "没有已归档的聊天"),
            (
                "error",
                ArchivedChatsState { error: Some("fixture-error".into()), ..Default::default() },
                "无法加载已归档的聊天，请重试。",
            ),
            (
                "filtered",
                ArchivedChatsState { search: "missing".into(), ..fixture() },
                "没有匹配的已归档聊天",
            ),
        ] {
            let mut ui = UserInterface::build(
                page(&state, locale),
                Size::new(320.0, 620.0),
                Cache::new(),
                &mut renderer,
            );
            ui.update(
                &[Event::Window(iced::window::Event::RedrawRequested(iced::time::Instant::now()))],
                mouse::Cursor::Unavailable,
                &mut renderer,
                &mut clipboard::Null,
                &mut Vec::new(),
            );
            ui.draw(
                &mut renderer,
                &Theme::Dark,
                &Style { text_color: Theme::Dark.palette().text },
                mouse::Cursor::Unavailable,
            );
            let mut geometry = Geometry::default();
            ui.operate(&mut renderer, &mut geometry);
            assert!(geometry.labels.iter().any(|(label, _)| label == expected), "{name}");
            if name == "error" {
                let bounds = geometry.labels.iter().find(|(label, _)| label == "重试").unwrap().1;
                assert!(click(&mut ui, &mut renderer, bounds.center()).iter().any(
                    |message| matches!(
                        message,
                        Message::Settings(SettingsMessage::ArchivedChats(
                            ArchivedChatsMessage::Refresh
                        ))
                    )
                ));
            }
        }
    }
}
