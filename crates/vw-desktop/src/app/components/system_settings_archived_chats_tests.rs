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
    use crate::app::state::{ArchivedChatsSort, ArchivedChatsState};
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
        focus: Vec<bool>,
        controls: Vec<Rectangle>,
    }

    impl Operation for Geometry {
        fn traverse(&mut self, children: &mut dyn FnMut(&mut dyn Operation)) {
            children(self);
        }
        fn text(&mut self, _: Option<&iced::advanced::widget::Id>, bounds: Rectangle, value: &str) {
            self.labels.push((value.to_owned(), bounds));
        }
        fn focusable(
            &mut self,
            _: Option<&iced::advanced::widget::Id>,
            bounds: Rectangle,
            focus: &mut dyn iced::advanced::widget::operation::focusable::Focusable,
        ) {
            self.focus.push(focus.is_focused());
            if !self.inputs.contains(&bounds) {
                self.controls.push(bounds);
            }
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
    async fn archived_chat_tooltip_stays_directly_above_title() {
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
        for theme in [Theme::Dark, Theme::Light] {
            for width in [360.0, 860.0] {
                let mut state = fixture();
                state.sessions.truncate(1);
                state.sessions[0].title = "归档聊天 TUI".into();
                let mut ui = UserInterface::build(
                    page(&state, DesktopLocale::SimplifiedChinese),
                    Size::new(width, 650.0),
                    Cache::new(),
                    &mut renderer,
                );
                let mut geometry = Geometry::default();
                ui.operate(&mut renderer, &mut geometry);
                let title =
                    geometry.labels.iter().find(|(label, _)| label == "归档聊天 TUI").unwrap().1;
                assert!(title.width < 120.0, "tooltip anchor must fit the title");
                ui.draw(
                    &mut renderer,
                    &theme,
                    &Style { text_color: theme.palette().text },
                    mouse::Cursor::Unavailable,
                );
                let size = Size::new(width as u32, 650);
                let before = renderer.screenshot(size, 1.0, theme.palette().background);
                let cursor = mouse::Cursor::Available(title.center());
                ui.update(
                    &[
                        Event::Mouse(mouse::Event::CursorMoved { position: title.center() }),
                        Event::Window(iced::window::Event::RedrawRequested(
                            iced::time::Instant::now(),
                        )),
                    ],
                    cursor,
                    &mut renderer,
                    &mut clipboard::Null,
                    &mut Vec::new(),
                );
                ui.draw(&mut renderer, &theme, &Style { text_color: theme.palette().text }, cursor);
                let after = renderer.screenshot(size, 1.0, theme.palette().background);
                let changed_above_title = ((title.y as u32 - 30)..title.y as u32)
                    .flat_map(|y| {
                        (title.x as u32..(title.x + title.width) as u32)
                            .map(move |x| ((y * size.width + x) * 4) as usize)
                    })
                    .filter(|&pixel| before[pixel..pixel + 4] != after[pixel..pixel + 4])
                    .count();
                assert!(changed_above_title > 100, "tooltip must render directly above title");
            }
        }
    }

    #[tokio::test]
    async fn archived_chats_restore_renders_and_disables_duplicate_clicks() {
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
        for locale in [
            DesktopLocale::SimplifiedChinese,
            DesktopLocale::TraditionalChinese,
            DesktopLocale::English,
        ] {
            for (theme_name, theme) in [("dark", Theme::Dark), ("light", Theme::Light)] {
                for width in [320_u32, 860] {
                    for operating in [false, true] {
                        let mut state = fixture();
                        state.sessions.truncate(1);
                        let id = state.sessions[0].id.clone();
                        if operating {
                            state.operating_ids.insert(id.clone());
                        }
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
                        let label = I18n::new(locale).__(if operating {
                            "正在恢复会话"
                        } else {
                            "取消归档"
                        });
                        let bounds =
                            geometry.labels.iter().find(|(text, _)| text == label).unwrap().1;
                        assert!(bounds.x >= 0.0 && bounds.x + bounds.width <= width as f32);
                        let messages = click(&mut ui, &mut renderer, bounds.center());
                        assert_eq!(
                            messages.iter().filter(|message| matches!(
                                message,
                                Message::Settings(SettingsMessage::ArchivedChats(ArchivedChatsMessage::Restore(target))) if target == &id
                            )).count(),
                            usize::from(!operating),
                        );
                        let pixels = renderer.screenshot(
                            Size::new(width, 620),
                            1.0,
                            theme.palette().background,
                        );
                        assert_eq!(pixels.len(), width as usize * 620 * 4);
                        if let Some(path) = std::env::var_os("SHIJU_ARCHIVED_SCREENSHOTS") {
                            image::save_buffer(
                                std::path::PathBuf::from(path).join(format!(
                                    "restore-{locale:?}-{theme_name}-{width}-{operating}.png"
                                )),
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
                    for (name, trigger) in
                        [("list", None), ("sort", Some("更新时间")), ("project", Some("所有项目"))]
                    {
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
                        if let Some(trigger) = trigger {
                            let mut geometry = Geometry::default();
                            ui.operate(&mut renderer, &mut geometry);
                            let bounds = geometry.controls[usize::from(trigger == "所有项目")];
                            assert!(click(&mut ui, &mut renderer, bounds.center()).is_empty());
                        }
                        ui.draw(
                            &mut renderer,
                            &theme,
                            &Style { text_color: theme.palette().text },
                            mouse::Cursor::Unavailable,
                        );
                        let mut geometry = Geometry::default();
                        ui.operate(&mut renderer, &mut geometry);
                        assert_eq!(geometry.inputs.len(), 1);
                        assert!(!geometry.labels.iter().any(|(label, _)| {
                            ["类型", "本地", "云端", "全部聊天"]
                                .iter()
                                .any(|source| label == I18n::new(locale).__(source))
                        }));
                        assert_eq!(geometry.scrolls.len(), if trigger.is_some() { 2 } else { 1 });
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
        let bounds = geometry.controls[0];
        assert!(click(&mut ui, &mut renderer, bounds.center()).is_empty());
        let mut messages = Vec::new();
        for named in [iced::keyboard::key::Named::End, iced::keyboard::key::Named::Enter] {
            ui.update(
                &[Event::Keyboard(iced::keyboard::Event::KeyPressed {
                    key: iced::keyboard::Key::Named(named),
                    modified_key: iced::keyboard::Key::Named(named),
                    physical_key: iced::keyboard::key::Physical::Unidentified(
                        iced::keyboard::key::NativeCode::Unidentified,
                    ),
                    location: iced::keyboard::Location::Standard,
                    modifiers: iced::keyboard::Modifiers::empty(),
                    text: None,
                    repeat: false,
                })],
                mouse::Cursor::Unavailable,
                &mut renderer,
                &mut clipboard::Null,
                &mut messages,
            );
        }
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
    #[tokio::test]
    async fn archived_chats_delete_renders_modal_and_blocks_background_actions() {
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
        let current =
            Identity::from_endpoint(&vw_gateway_client::GatewayEndpoint::new("127.0.0.1", 1));
        for (theme_name, theme) in [("dark", Theme::Dark), ("light", Theme::Light)] {
            for width in [320_u32, 860] {
                for pending in [false, true] {
                    let mut state = fixture();
                    state.sessions.truncate(1);
                    state.sessions[0].title = "删除确认 · 中文子聊天".into();
                    state.identity = Some(current.clone());
                    let id = state.sessions[0].id.clone();
                    apply(&mut state, ArchivedChatsMessage::DeleteRequested(id), &current);
                    if pending {
                        apply(&mut state, ArchivedChatsMessage::DeleteConfirmed, &current);
                    }
                    let mut ui = UserInterface::build(
                        page(&state, DesktopLocale::SimplifiedChinese),
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
                        &mut vec![],
                    );
                    ui.draw(
                        &mut renderer,
                        &theme,
                        &Style { text_color: theme.palette().text },
                        mouse::Cursor::Unavailable,
                    );
                    let mut geometry = Geometry::default();
                    ui.operate(&mut renderer, &mut geometry);
                    assert!(geometry.labels.iter().any(|(label, _)| label == "删除已归档的聊天？"));
                    assert!(geometry.inputs.is_empty(), "background focus targets hidden");
                    assert_eq!(geometry.focus, if pending { vec![] } else { vec![true] });
                    let cancel =
                        geometry.labels.iter().find(|(label, _)| label == "取消").unwrap().1;
                    assert!(cancel.x >= 0.0 && cancel.x + cancel.width <= width as f32);
                    let messages = click(&mut ui, &mut renderer, cancel.center());
                    assert_eq!(
                        messages
                            .iter()
                            .filter(|m| matches!(
                                m,
                                Message::Settings(SettingsMessage::ArchivedChats(
                                    ArchivedChatsMessage::DeleteCancelled
                                ))
                            ))
                            .count(),
                        usize::from(!pending)
                    );
                    let confirm = geometry
                        .labels
                        .iter()
                        .find(|(label, _)| {
                            label == if pending { "正在删除…" } else { "删除" }
                        })
                        .unwrap()
                        .1;
                    let messages = click(&mut ui, &mut renderer, confirm.center());
                    assert_eq!(
                        messages
                            .iter()
                            .filter(|m| matches!(
                                m,
                                Message::Settings(SettingsMessage::ArchivedChats(
                                    ArchivedChatsMessage::DeleteConfirmed
                                ))
                            ))
                            .count(),
                        usize::from(!pending)
                    );
                    for key in
                        [iced::keyboard::key::Named::Enter, iced::keyboard::key::Named::Escape]
                    {
                        let mut messages = vec![];
                        ui.update(
                            &[Event::Keyboard(iced::keyboard::Event::KeyPressed {
                                key: iced::keyboard::Key::Named(key),
                                modified_key: iced::keyboard::Key::Named(key),
                                physical_key: iced::keyboard::key::Physical::Unidentified(
                                    iced::keyboard::key::NativeCode::Unidentified,
                                ),
                                location: iced::keyboard::Location::Standard,
                                modifiers: iced::keyboard::Modifiers::empty(),
                                text: None,
                                repeat: false,
                            })],
                            mouse::Cursor::Unavailable,
                            &mut renderer,
                            &mut clipboard::Null,
                            &mut messages,
                        );
                        assert!(!messages.iter().any(|m| matches!(
                            m,
                            Message::Settings(SettingsMessage::ArchivedChats(
                                ArchivedChatsMessage::DeleteConfirmed
                            ))
                        )));
                        if !pending {
                            assert!(messages.iter().any(|m| matches!(
                                m,
                                Message::Settings(SettingsMessage::ArchivedChats(
                                    ArchivedChatsMessage::DeleteCancelled
                                ))
                            )));
                        }
                    }
                    let pixels =
                        renderer.screenshot(Size::new(width, 620), 1.0, theme.palette().background);
                    assert_eq!(pixels.len(), width as usize * 620 * 4);
                    if let Some(path) = std::env::var_os("SHIJU_ARCHIVED_SCREENSHOTS") {
                        image::save_buffer(
                            std::path::PathBuf::from(path)
                                .join(format!("delete-{theme_name}-{width}-{pending}.png")),
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
    #[tokio::test]
    async fn archived_chats_project_delete_renders_modal_and_blocks_background_actions() {
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
        let current =
            Identity::from_endpoint(&vw_gateway_client::GatewayEndpoint::new("127.0.0.1", 1));
        for (theme_name, theme) in [("dark", Theme::Dark), ("light", Theme::Light)] {
            for width in [320_u32, 860] {
                for pending in [false, true] {
                    let mut state = fixture();
                    state.sessions.truncate(1);
                    state.sessions[0].title = "删除确认 · 中文子聊天".into();
                    state.identity = Some(current.clone());
                    let project_id = state.sessions[0].project_id.clone();
                    {
                        let mut ui = UserInterface::build(
                            page(&state, DesktopLocale::SimplifiedChinese),
                            Size::new(width as f32, 620.0),
                            Cache::new(),
                            &mut renderer,
                        );
                        let mut geometry = Geometry::default();
                        ui.operate(&mut renderer, &mut geometry);
                        let more =
                            geometry.labels.iter().find(|(label, _)| label == "…").unwrap().1;
                        assert!(click(&mut ui, &mut renderer, more.center()).iter().any(|m| matches!(m, Message::Settings(SettingsMessage::ArchivedChats(ArchivedChatsMessage::ProjectActionsToggled(id))) if id == &project_id)));
                    }
                    apply(
                        &mut state,
                        ArchivedChatsMessage::ProjectActionsToggled(project_id.clone()),
                        &current,
                    );
                    {
                        let mut ui = UserInterface::build(
                            page(&state, DesktopLocale::SimplifiedChinese),
                            Size::new(width as f32, 620.0),
                            Cache::new(),
                            &mut renderer,
                        );
                        let mut geometry = Geometry::default();
                        ui.operate(&mut renderer, &mut geometry);
                        let delete = geometry
                            .labels
                            .iter()
                            .find(|(label, _)| label == "删除项目中的全部已归档聊天")
                            .unwrap()
                            .1;
                        assert!(click(&mut ui, &mut renderer, delete.center()).iter().any(|m| matches!(m, Message::Settings(SettingsMessage::ArchivedChats(ArchivedChatsMessage::ProjectDeleteRequested(id))) if id == &project_id)));
                    }
                    apply(
                        &mut state,
                        ArchivedChatsMessage::ProjectDeleteRequested(project_id),
                        &current,
                    );
                    if pending {
                        apply(&mut state, ArchivedChatsMessage::DeleteConfirmed, &current);
                    }
                    let mut ui = UserInterface::build(
                        page(&state, DesktopLocale::SimplifiedChinese),
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
                        &mut vec![],
                    );
                    ui.draw(
                        &mut renderer,
                        &theme,
                        &Style { text_color: theme.palette().text },
                        mouse::Cursor::Unavailable,
                    );
                    let mut geometry = Geometry::default();
                    ui.operate(&mut renderer, &mut geometry);
                    assert!(
                        geometry
                            .labels
                            .iter()
                            .any(|(label, _)| label == "删除项目中的全部已归档聊天？")
                    );
                    assert!(geometry.inputs.is_empty(), "background focus targets hidden");
                    assert_eq!(geometry.focus, if pending { vec![] } else { vec![true] });
                    let cancel =
                        geometry.labels.iter().find(|(label, _)| label == "取消").unwrap().1;
                    assert!(cancel.x >= 0.0 && cancel.x + cancel.width <= width as f32);
                    let messages = click(&mut ui, &mut renderer, cancel.center());
                    assert_eq!(
                        messages
                            .iter()
                            .filter(|m| matches!(
                                m,
                                Message::Settings(SettingsMessage::ArchivedChats(
                                    ArchivedChatsMessage::DeleteCancelled
                                ))
                            ))
                            .count(),
                        usize::from(!pending)
                    );
                    let confirm = geometry
                        .labels
                        .iter()
                        .find(|(label, _)| {
                            label == if pending { "正在删除…" } else { "删除" }
                        })
                        .unwrap()
                        .1;
                    let messages = click(&mut ui, &mut renderer, confirm.center());
                    assert_eq!(
                        messages
                            .iter()
                            .filter(|m| matches!(
                                m,
                                Message::Settings(SettingsMessage::ArchivedChats(
                                    ArchivedChatsMessage::DeleteConfirmed
                                ))
                            ))
                            .count(),
                        usize::from(!pending)
                    );
                    for key in
                        [iced::keyboard::key::Named::Enter, iced::keyboard::key::Named::Escape]
                    {
                        let mut messages = vec![];
                        ui.update(
                            &[Event::Keyboard(iced::keyboard::Event::KeyPressed {
                                key: iced::keyboard::Key::Named(key),
                                modified_key: iced::keyboard::Key::Named(key),
                                physical_key: iced::keyboard::key::Physical::Unidentified(
                                    iced::keyboard::key::NativeCode::Unidentified,
                                ),
                                location: iced::keyboard::Location::Standard,
                                modifiers: iced::keyboard::Modifiers::empty(),
                                text: None,
                                repeat: false,
                            })],
                            mouse::Cursor::Unavailable,
                            &mut renderer,
                            &mut clipboard::Null,
                            &mut messages,
                        );
                        assert!(!messages.iter().any(|m| matches!(
                            m,
                            Message::Settings(SettingsMessage::ArchivedChats(
                                ArchivedChatsMessage::DeleteConfirmed
                            ))
                        )));
                        if !pending {
                            assert!(messages.iter().any(|m| matches!(
                                m,
                                Message::Settings(SettingsMessage::ArchivedChats(
                                    ArchivedChatsMessage::DeleteCancelled
                                ))
                            )));
                        }
                    }
                    let pixels =
                        renderer.screenshot(Size::new(width, 620), 1.0, theme.palette().background);
                    assert_eq!(pixels.len(), width as usize * 620 * 4);
                    if let Some(path) = std::env::var_os("SHIJU_ARCHIVED_SCREENSHOTS") {
                        image::save_buffer(
                            std::path::PathBuf::from(path)
                                .join(format!("project-delete-{theme_name}-{width}-{pending}.png")),
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
}
