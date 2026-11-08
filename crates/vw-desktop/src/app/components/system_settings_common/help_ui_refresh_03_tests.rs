use super::super::{settings_modal_card, settings_modal_overlay, settings_select};
use crate::app::Message;
use iced::advanced::renderer::Headless;
use iced::advanced::{Layout, Shell, clipboard, layout, widget::Tree};
use iced::widget::{container, text};
use iced::{Element, Event, Font, Point, Rectangle, Renderer, Size, Vector, mouse};

fn update(
    element: &mut Element<'_, Message>,
    tree: &mut Tree,
    layout: Layout<'_>,
    renderer: &Renderer,
    viewport: &Rectangle,
    event: Event,
    cursor: mouse::Cursor,
) -> Vec<Message> {
    let mut messages = Vec::new();
    element.as_widget_mut().update(
        tree,
        &event,
        layout,
        cursor,
        renderer,
        &mut clipboard::Null,
        &mut Shell::new(&mut messages),
        viewport,
    );
    messages
}

#[tokio::test]
async fn ui_refresh_03_layout_and_interaction_hides_background_menu() {
    let renderer = Renderer::new(Font::DEFAULT, 16.0.into(), Some("tiny-skia")).await.unwrap();
    let viewport = Rectangle::with_size(Size::new(420.0, 440.0));
    let limits = layout::Limits::new(Size::ZERO, viewport.size());
    let mut modal = settings_modal_overlay(
        Some(
            container(settings_select(["原值", "第二项"], Some("原值"), |_| {
                Message::GatewayHealthTick
            }))
            .width(iced::Length::Fill)
            .height(iced::Length::Fill)
            .into(),
        ),
        Message::None,
        settings_modal_card(text("設定の確認")).width(240).height(180),
    );
    let mut tree = Tree::new(modal.as_widget());
    let node = modal.as_widget_mut().layout(&mut tree, &renderer, &limits);
    let root = Layout::new(&node);
    let base = root.children().next().unwrap();
    let select = base.children().next().unwrap();
    let cursor = mouse::Cursor::Available(Point::new(select.bounds().center_x(), 16.0));

    // Recreate the state of a selector that was already open when the modal appeared.
    let mut underlying: Element<'_, Message> =
        settings_select(["原值", "第二项"], Some("原值"), |_| Message::GatewayHealthTick).into();
    for event in [
        mouse::Event::ButtonPressed(mouse::Button::Left),
        mouse::Event::ButtonReleased(mouse::Button::Left),
    ] {
        assert!(
            update(
                &mut underlying,
                &mut tree.children[0],
                select,
                &renderer,
                &viewport,
                Event::Mouse(event),
                cursor,
            )
            .is_empty()
        );
    }
    assert!(
        underlying
            .as_widget_mut()
            .overlay(&mut tree.children[0], select, &renderer, &viewport, Vector::ZERO,)
            .is_some(),
        "the background select must really be open"
    );
    assert!(
        modal
            .as_widget_mut()
            .overlay(&mut tree, root, &renderer, &viewport, Vector::ZERO)
            .is_none(),
        "an open background select must stay behind the modal"
    );
}

use super::super::{
    primary_action_btn_style, rounded_action_btn_style, settings_error_banner,
    settings_modal_header, settings_modal_layout, settings_row, settings_toggle,
};
use crate::app::message::{GeneralSettingsMessage, SettingsMessage};
use iced::advanced::{overlay, renderer, widget};
use iced::keyboard::{self, Key, key::Named};
use iced::widget::{button, column, row};
use iced::{Length, Theme};

struct Scene<'a> {
    element: Element<'a, Message>,
    tree: Tree,
    node: layout::Node,
    renderer: Renderer,
    viewport: Rectangle,
    messages: Vec<Message>,
}

impl<'a> Scene<'a> {
    async fn new(mut element: Element<'a, Message>, width: f32, height: f32) -> Self {
        let renderer = Renderer::new(Font::DEFAULT, 16.0.into(), Some("tiny-skia")).await.unwrap();
        let viewport = Rectangle::with_size(Size::new(width, height));
        let mut tree = Tree::new(element.as_widget());
        let node = element.as_widget_mut().layout(
            &mut tree,
            &renderer,
            &layout::Limits::new(Size::ZERO, viewport.size()),
        );
        Self { element, tree, node, renderer, viewport, messages: Vec::new() }
    }

    fn overlay(&mut self) -> Option<overlay::Element<'_, Message, Theme, Renderer>> {
        self.element.as_widget_mut().overlay(
            &mut self.tree,
            Layout::new(&self.node),
            &self.renderer,
            &self.viewport,
            Vector::ZERO,
        )
    }

    fn dispatch(&mut self, event: Event, mut cursor: mouse::Cursor) {
        let mut shell = Shell::new(&mut self.messages);
        if let Some(overlay) = self.element.as_widget_mut().overlay(
            &mut self.tree,
            Layout::new(&self.node),
            &self.renderer,
            &self.viewport,
            Vector::ZERO,
        ) {
            let mut overlay = overlay::Nested::new(overlay);
            let layout = overlay.layout(&self.renderer, self.viewport.size());
            let layout = Layout::new(&layout);
            let interaction = overlay.mouse_interaction(layout, cursor, &self.renderer);
            overlay.update(
                &event,
                layout,
                cursor,
                &self.renderer,
                &mut clipboard::Null,
                &mut shell,
            );
            if interaction != mouse::Interaction::None {
                cursor = mouse::Cursor::Unavailable;
            }
        }
        if !shell.is_event_captured() {
            self.element.as_widget_mut().update(
                &mut self.tree,
                &event,
                Layout::new(&self.node),
                cursor,
                &self.renderer,
                &mut clipboard::Null,
                &mut shell,
                &self.viewport,
            );
        }
    }

    fn click(&mut self, point: Point) {
        for event in [
            mouse::Event::ButtonPressed(mouse::Button::Left),
            mouse::Event::ButtonReleased(mouse::Button::Left),
        ] {
            self.dispatch(Event::Mouse(event), mouse::Cursor::Available(point));
        }
    }

    fn key(&mut self, key: Named) {
        self.dispatch(
            Event::Keyboard(keyboard::Event::KeyPressed {
                key: Key::Named(key),
                modified_key: Key::Named(key),
                physical_key: keyboard::key::Physical::Unidentified(
                    keyboard::key::NativeCode::Unidentified,
                ),
                location: keyboard::Location::Standard,
                modifiers: keyboard::Modifiers::empty(),
                text: None,
                repeat: false,
            }),
            mouse::Cursor::Unavailable,
        );
    }

    fn snapshot(&mut self, theme: &Theme, cursor: mouse::Cursor, file: &str) -> image::RgbaImage {
        use iced::advanced::Renderer as _;

        self.dispatch(
            Event::Window(iced::window::Event::RedrawRequested(iced::time::Instant::now())),
            cursor,
        );
        self.renderer.reset(self.viewport);
        let defaults = renderer::Style { text_color: theme.palette().text };
        let root = Layout::new(&self.node);
        let base_cursor = if let Some(overlay) = self.element.as_widget_mut().overlay(
            &mut self.tree,
            root,
            &self.renderer,
            &self.viewport,
            Vector::ZERO,
        ) {
            let mut overlay = overlay::Nested::new(overlay);
            let node = overlay.layout(&self.renderer, self.viewport.size());
            if overlay.mouse_interaction(Layout::new(&node), cursor, &self.renderer)
                == mouse::Interaction::None
            {
                cursor
            } else {
                mouse::Cursor::Unavailable
            }
        } else {
            cursor
        };
        self.element.as_widget().draw(
            &self.tree,
            &mut self.renderer,
            theme,
            &defaults,
            root,
            base_cursor,
            &self.viewport,
        );
        // Rebuild the overlay after update and paint it in the same layer as the Iced runtime.
        if let Some(overlay) = self.element.as_widget_mut().overlay(
            &mut self.tree,
            root,
            &self.renderer,
            &self.viewport,
            Vector::ZERO,
        ) {
            let mut overlay = overlay::Nested::new(overlay);
            let layout = overlay.layout(&self.renderer, self.viewport.size());
            overlay.draw(&mut self.renderer, theme, &defaults, Layout::new(&layout), cursor);
        }
        let size = Size::new(self.viewport.width as u32, self.viewport.height as u32);
        let pixels = self.renderer.screenshot(size, 1.0, theme.palette().background);
        let image = image::RgbaImage::from_raw(size.width, size.height, pixels).unwrap();
        if let Some(directory) = std::env::var_os("SHIJU_UI_REFRESH_03_PREVIEW_DIR") {
            std::fs::create_dir_all(&directory).unwrap();
            image.save(std::path::Path::new(&directory).join(file)).unwrap();
        }
        image
    }
}

fn full_window(content: impl Into<Element<'static, Message>>) -> Element<'static, Message> {
    container(content).width(Length::Fill).height(Length::Fill).into()
}

fn modal_card(node: &layout::Node) -> Layout<'_> {
    Layout::new(node)
        .children()
        .nth(1)
        .expect("modal stack")
        .children()
        .nth(1)
        .expect("centered layer")
        .children()
        .next()
        .expect("card")
}

fn cancel() -> Message {
    Message::Settings(SettingsMessage::CustomProviderModelClose)
}

fn selection(value: String) -> Message {
    Message::Settings(SettingsMessage::General(GeneralSettingsMessage::TimezoneSelected(value)))
}

fn crop(image: &image::RgbaImage, bounds: Rectangle) -> image::RgbaImage {
    image::imageops::crop_imm(
        image,
        bounds.x.ceil() as u32,
        bounds.y.ceil() as u32,
        bounds.width.floor() as u32 - 1,
        bounds.height.floor() as u32 - 1,
    )
    .to_image()
}

#[derive(Default)]
struct FormContents {
    labels: Vec<String>,
    field: Option<Rectangle>,
}

impl widget::Operation for FormContents {
    fn traverse(&mut self, children: &mut dyn FnMut(&mut dyn widget::Operation)) {
        children(self);
    }

    fn text(&mut self, _: Option<&widget::Id>, _: Rectangle, value: &str) {
        self.labels.push(value.to_owned());
    }

    fn container(&mut self, id: Option<&widget::Id>, bounds: Rectangle) {
        if id == Some(&widget::Id::from("fixture-timezone")) {
            self.field = Some(bounds);
        }
    }
}

#[tokio::test]
async fn help_rollout_01_renders_contextual_help() {
    use super::super::{settings_flexible_row_with_help, settings_group_heading_with_help};
    use crate::app::i18n::I18n;
    use vw_config_types::ui::DesktopLocale;

    {
        let mut fonts = iced::advanced::graphics::text::font_system().write().unwrap();
        for font in crate::fonts::load_all() {
            fonts.load_font(font);
        }
    }

    for (locale, width, theme) in [
        (DesktopLocale::English, 720.0, Theme::Light),
        (DesktopLocale::SimplifiedChinese, 320.0, Theme::Dark),
        (DesktopLocale::TraditionalChinese, 320.0, Theme::Light),
    ] {
        let i18n = I18n::new(locale);
        let title = i18n.__("界面");
        let intro = i18n.__("应用外观与预览行为。");
        let field = i18n.__("聊天时区");
        let description = i18n.__("用于模型时间上下文，新会话生效。");
        let error = "Fixture error";
        let select =
            settings_select(["UTC", "Asia/Tokyo"], Some("UTC"), |value| selection(value.into()));
        let control = column![select, settings_error_banner(error)].spacing(6);
        let form = column![
            settings_group_heading_with_help(i18n, &theme, title, intro),
            settings_flexible_row_with_help(i18n, &theme, field, description, 280.0, control)
                .id("fixture-timezone"),
        ]
        .spacing(12);
        let mut scene = Scene::new(form.into(), width, 320.0).await;
        let mut contents = FormContents::default();
        scene.element.as_widget_mut().operate(
            &mut scene.tree,
            Layout::new(&scene.node),
            &scene.renderer,
            &mut contents,
        );
        assert!(contents.labels.iter().any(|value| value == error));
        let field_bounds = contents.field.expect("field id for search navigation");
        let prefix = format!("form-help-{locale:?}-{width}");
        let idle =
            scene.snapshot(&theme, mouse::Cursor::Unavailable, &format!("{prefix}-idle.png"));
        assert!(
            !contents.labels.iter().any(|value| value == intro || value == description),
            "optional explanations must only appear inside help cards: {:?}",
            contents.labels
        );

        let root = Layout::new(&scene.node);
        let heading = root.children().next().unwrap();
        let fields = root.children().nth(1).unwrap().children().next().unwrap();
        assert_eq!(field_bounds, root.children().nth(1).unwrap().bounds());
        let field_label = fields.children().next().unwrap();
        let control = fields.children().nth(1).unwrap();
        let select = control
            .children()
            .next()
            .unwrap()
            .children()
            .next()
            .unwrap()
            .children()
            .next()
            .unwrap();
        let select_point = select.bounds().center();
        let help_points = [heading, field_label].map(|label| {
            let bounds = label.children().nth(1).expect("independent help target").bounds();
            assert_eq!(bounds.size(), Size::new(22.0, 22.0));
            assert!(bounds.x + bounds.width <= scene.viewport.width);
            bounds.center()
        });
        for (index, point) in help_points.into_iter().enumerate() {
            let cursor = mouse::Cursor::Available(point);
            scene.dispatch(Event::Mouse(mouse::Event::CursorMoved { position: point }), cursor);
            assert!(scene.overlay().is_some(), "hover opens the help overlay");
            let opened = scene.snapshot(&theme, cursor, &format!("{prefix}-open-{index}.png"));
            assert_ne!(opened, idle, "opening help paints a card");
            scene.dispatch(Event::Mouse(mouse::Event::CursorLeft), mouse::Cursor::Unavailable);
            assert!(scene.overlay().is_none());
            assert!(scene.messages.is_empty());
        }
        scene.click(select_point);
        assert!(scene.overlay().is_some(), "the original selector still opens");
        scene.key(Named::ArrowDown);
        scene.key(Named::Enter);
        assert!(matches!(
            scene.messages.as_slice(),
            [Message::Settings(SettingsMessage::General(GeneralSettingsMessage::TimezoneSelected(value)))]
                if value == "Asia/Tokyo"
        ));
    }
}

#[tokio::test]
async fn ui_refresh_03_layout_and_interaction_keeps_actions_fixed() {
    let options = (0..12).map(|i| format!("时区选项 {i} / Test timezone")).collect::<Vec<_>>();
    let first = options[0].clone();
    let second = options[1].clone();
    let select = settings_select(options, Some(first), selection).width(Length::Fill);
    let body = column((0..20).map(|i| text(format!("配置示例 {i}")).size(16).height(48).into()))
        .push(settings_error_banner("示例加载失败，请重试"));
    let form = column![select, body].spacing(12);
    let header = settings_modal_header(
        text("配置範例 / Settings").size(16),
        super::settings_close_button(cancel()),
    );
    let actions = row![
        button(text("取消")).on_press(cancel()).padding([8, 14]).style(rounded_action_btn_style),
        button(text("保存"))
            .on_press(Message::Settings(SettingsMessage::CustomProviderModelModalSave))
            .padding([8, 14])
            .style(primary_action_btn_style),
    ]
    .spacing(10);
    let card = settings_modal_layout(header, form, Some(actions.into())).width(380).height(350);
    let base = full_window(button(text("Background")).on_press(Message::GatewayHealthTick));
    let mut scene =
        Scene::new(settings_modal_overlay(Some(base), cancel(), card), 420.0, 440.0).await;
    let card = modal_card(&scene.node);
    assert_eq!(card.bounds().center(), scene.viewport.center());
    let parts = card.children().next().unwrap();
    let header = parts.children().next().unwrap().bounds();
    let scroll = parts.children().nth(1).unwrap();
    let footer = parts.children().nth(2).unwrap();
    let footer_bounds = footer.bounds();
    let cancel_point = footer.children().next().unwrap().bounds().center();
    let save_point = footer.children().nth(1).unwrap().bounds().center();
    let scroll_bounds = scroll.bounds();
    let scroll_content = scroll.children().next().unwrap();
    let form = scroll_content.children().next().unwrap();
    let trigger = form.children().next().unwrap().bounds();
    assert!(form.bounds().x - scroll_bounds.x >= 7.0);
    assert!(scroll_bounds.x + scroll_bounds.width - form.bounds().x - form.bounds().width >= 7.0);
    assert!(header.y + header.height < scroll_bounds.y);
    assert!(scroll_bounds.y + scroll_bounds.height < footer_bounds.y);

    // Focus navigation must reach the selector inside the modal.
    let mut focus: Box<dyn widget::Operation> =
        Box::new(widget::operation::focusable::focus_next());
    loop {
        scene.element.as_widget_mut().operate(
            &mut scene.tree,
            Layout::new(&scene.node),
            &scene.renderer,
            focus.as_mut(),
        );
        match focus.finish() {
            widget::operation::Outcome::Chain(next) => focus = next,
            _ => break,
        }
    }
    scene.key(Named::Enter);
    assert!(scene.overlay().is_some());
    scene.key(Named::ArrowDown);
    scene.key(Named::Escape);
    assert!(scene.overlay().is_none());
    assert!(scene.messages.is_empty());
    scene.key(Named::Enter);
    scene.key(Named::ArrowDown);
    scene.key(Named::Enter);
    assert!(matches!(
        scene.messages.as_slice(),
        [Message::Settings(SettingsMessage::General(GeneralSettingsMessage::TimezoneSelected(value)))]
        if value == &second
    ));
    scene.messages.clear();

    scene.click(trigger.center());
    let viewport = scene.viewport;
    let menu = {
        let mut overlay = scene
            .element
            .as_widget_mut()
            .overlay(
                &mut scene.tree,
                Layout::new(&scene.node),
                &scene.renderer,
                &viewport,
                Vector::ZERO,
            )
            .expect("nested select overlay");
        let node = overlay.as_overlay_mut().layout(&scene.renderer, viewport.size());
        let mut menu = Layout::new(&node);
        while menu.bounds() == viewport {
            menu = menu.children().next().unwrap();
        }
        menu.bounds()
    };
    assert!(viewport.contains(menu.position()));
    assert!(menu.y + menu.height <= viewport.height);
    assert!(menu.height >= 86.0, "nested menu is too short for two rows: {menu:?}");
    let hovered = Point::new(menu.center_x(), menu.y + 60.0);
    let cursor = mouse::Cursor::Available(hovered);
    scene.dispatch(Event::Mouse(mouse::Event::CursorMoved { position: hovered }), cursor);
    scene.snapshot(&Theme::Dark, cursor, "modal-menu-hover-dark-420.png");
    scene.dispatch(Event::Mouse(mouse::Event::CursorLeft), mouse::Cursor::Unavailable);
    scene.click(hovered);
    assert!(matches!(
        scene.messages.as_slice(),
        [Message::Settings(SettingsMessage::General(GeneralSettingsMessage::TimezoneSelected(value)))]
        if value == &second
    ));
    scene.messages.clear();

    let before =
        scene.snapshot(&Theme::Light, mouse::Cursor::Unavailable, "modal-top-light-420.png");
    scene.dispatch(
        Event::Mouse(mouse::Event::WheelScrolled {
            delta: mouse::ScrollDelta::Pixels { x: 0.0, y: -180.0 },
        }),
        mouse::Cursor::Available(scroll_bounds.center()),
    );
    let after =
        scene.snapshot(&Theme::Light, mouse::Cursor::Unavailable, "modal-scrolled-light-420.png");
    assert_eq!(crop(&before, header), crop(&after, header));
    assert_eq!(crop(&before, footer_bounds), crop(&after, footer_bounds));
    assert_ne!(crop(&before, scroll_bounds), crop(&after, scroll_bounds));

    scene.click(Point::new(12.0, 12.0)); // Backdrop over the background button.
    assert!(scene.messages.is_empty());
    scene.click(cancel_point);
    assert!(matches!(
        scene.messages.as_slice(),
        [Message::Settings(SettingsMessage::CustomProviderModelClose)]
    ));
    scene.messages.clear();
    scene.click(save_point);
    assert!(matches!(
        scene.messages.as_slice(),
        [Message::Settings(SettingsMessage::CustomProviderModelModalSave)]
    ));
}

#[tokio::test]
async fn ui_refresh_03_preserves_values_and_disabled_states() {
    let description = "套用目前裝置的偏好設定。".repeat(6);
    for (width, theme, name) in
        [(640.0, Theme::Light, "light-640"), (960.0, Theme::Dark, "dark-960")]
    {
        let page =
            settings_row("开机启动 / Launch at login", &description, 280.0, settings_toggle(true));
        let mut scene = Scene::new(page, width, 140.0).await;
        let root = Layout::new(&scene.node).children().next().unwrap();
        let label = root.children().next().unwrap().bounds();
        let control = root.children().nth(1).unwrap();
        assert!((label.center_y() - control.bounds().center_y()).abs() < 0.5);
        let toggle = control.children().next().unwrap().bounds();
        assert!((label.center_y() - toggle.center_y()).abs() < 0.5);
        assert!(label.x + label.width < toggle.x);
        scene.click(toggle.center());
        scene.key(Named::Enter);
        assert!(scene.messages.is_empty());
        scene.snapshot(&theme, mouse::Cursor::Unavailable, &format!("row-disabled-{name}.png"));
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn ui_refresh_03_renders_localized_help_and_keeps_copy_and_close() {
    use vw_config_types::ui::DesktopLocale;

    crate::app::config::with_editor_settings_gateway(|mut app, _, _| {
        iced::futures::executor::block_on(async {
            for (locale, name, theme, title, body) in [
                (
                    DesktopLocale::SimplifiedChinese,
                    "zh-cn",
                    Theme::Light,
                    "配置帮助",
                    "这是一条虚构的配置说明。",
                ),
                (
                    DesktopLocale::TraditionalChinese,
                    "zh-tw",
                    Theme::Dark,
                    "設定說明",
                    "這是一條虛構的設定說明。",
                ),
                (
                    DesktopLocale::English,
                    "en",
                    Theme::Light,
                    "Configuration help",
                    "Example configuration documentation.",
                ),
            ] {
                app.desktop_locale = locale;
                let help = format!(
                    "{body}\n\n/virtual/example/workspaces/fake-profile/settings/defaults.toml\n\n"
                )
                .repeat(30);
                let mut scene = Scene::new(
                    super::with_settings_help_modal(
                        &app,
                        full_window(text("Settings")),
                        title,
                        help.clone(),
                        cancel(),
                    ),
                    420.0,
                    460.0,
                )
                .await;
                let header =
                    modal_card(&scene.node).children().next().unwrap().children().next().unwrap();
                let actions = header.children().nth(1).unwrap();
                let copy = actions.children().next().unwrap().bounds();
                let close = actions.children().nth(1).unwrap().bounds();
                assert!((copy.center_y() - close.center_y()).abs() < 0.5);
                scene.snapshot(&theme, mouse::Cursor::Unavailable, &format!("help-{name}-420.png"));
                scene.click(copy.center());
                assert!(
                    matches!(scene.messages.as_slice(), [Message::CopyCode(value)] if value == &help)
                );
                scene.messages.clear();
                scene.click(close.center());
                assert!(matches!(
                    scene.messages.as_slice(),
                    [Message::Settings(SettingsMessage::CustomProviderModelClose)]
                ));
            }
        });
    });
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn ui_refresh_03_renders_model_draft_and_error_with_original_actions() {
    use crate::app::components::system_settings_providers::custom_model_modal;
    use crate::app::state::CustomProviderModelModalState;
    use vw_config_types::ui::DesktopLocale;

    crate::app::config::with_editor_settings_gateway(|mut app, _, _| {
        iced::futures::executor::block_on(async {
            app.desktop_locale = DesktopLocale::TraditionalChinese;
            app.provider_settings.custom_model_modal = Some(CustomProviderModelModalState {
                edit_index: Some(0),
                model_id: "example-org/test-model-with-a-long-identifier-2026".to_owned(),
                display_name: "本機測試模型".to_owned(),
                ..Default::default()
            });
            app.provider_settings.save_error = Some("範例模型儲存失敗，請重試。".into());
            let mut scene = Scene::new(
                custom_model_modal::view_overlays(&app, full_window(text("Example providers"))),
                420.0,
                460.0,
            )
            .await;
            let inset = modal_card(&scene.node);
            let card = inset.children().next().expect("inset model card");
            let contents = card.children().next().expect("model card contents");
            let footer = contents.children().last().expect("model card actions");
            let cancel_point = footer.children().nth(1).unwrap().bounds().center();
            let save_point = footer.children().nth(2).unwrap().bounds().center();
            scene.snapshot(
                &Theme::Dark,
                mouse::Cursor::Unavailable,
                "model-error-zh-tw-dark-420.png",
            );
            scene.click(Point::new(10.0, 4.0));
            assert!(scene.messages.is_empty(), "backdrop must not discard the draft");
            scene.click(cancel_point);
            assert!(matches!(
                scene.messages.as_slice(),
                [Message::Settings(SettingsMessage::CustomProviderModelClose)]
            ));
            scene.messages.clear();
            scene.click(save_point);
            assert!(matches!(
                scene.messages.as_slice(),
                [Message::Settings(SettingsMessage::CustomProviderModelModalSave)]
            ));
        });
    });
}
