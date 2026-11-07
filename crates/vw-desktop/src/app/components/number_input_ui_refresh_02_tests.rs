use super::NumberInput;
use crate::app::Message;
use crate::app::components::system_settings_common::{
    SETTINGS_CONTROL_PADDING, SETTINGS_CONTROL_TEXT_SIZE, settings_text_editor_style_for,
};
use iced::advanced::{
    Layout, Renderer as _, Shell, clipboard, layout, mouse,
    renderer::{self, Headless},
    widget::Tree,
};
use iced::widget::text_editor;
use iced::{Element, Event, Font, Pixels, Point, Rectangle, Renderer, Size, Theme};

struct Form<'a> {
    element: Element<'a, Message>,
    renderer: Renderer,
    viewport: Rectangle,
    tree: Tree,
    node: layout::Node,
    messages: Vec<Message>,
}

impl<'a> Form<'a> {
    async fn new(mut element: Element<'a, Message>, width: u32) -> Self {
        let renderer = <Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
            .await
            .expect("tiny-skia renderer");
        let viewport = Rectangle::with_size(Size::new(width as f32, 480.0));
        let mut tree = Tree::new(element.as_widget());
        let node = element.as_widget_mut().layout(
            &mut tree,
            &renderer,
            &layout::Limits::new(Size::ZERO, viewport.size()),
        );
        Self { element, renderer, viewport, tree, node, messages: Vec::new() }
    }

    fn update(&mut self, event: Event, cursor: mouse::Cursor) {
        self.element.as_widget_mut().update(
            &mut self.tree,
            &event,
            Layout::new(&self.node),
            cursor,
            &self.renderer,
            &mut clipboard::Null,
            &mut Shell::new(&mut self.messages),
            &self.viewport,
        );
    }

    fn screenshot(&mut self, theme: &Theme, cursor: mouse::Cursor) -> Vec<u8> {
        self.update(
            Event::Window(iced::window::Event::RedrawRequested(iced::time::Instant::now())),
            cursor,
        );
        self.renderer.reset(self.viewport);
        self.element.as_widget().draw(
            &self.tree,
            &mut self.renderer,
            theme,
            &renderer::Style { text_color: theme.palette().text },
            Layout::new(&self.node),
            cursor,
            &self.viewport,
        );
        self.renderer.screenshot(
            Size::new(self.viewport.width as u32, self.viewport.height as u32),
            1.0,
            theme.palette().background,
        )
    }

    fn pixel<'p>(&self, pixels: &'p [u8], x: usize, y: usize) -> &'p [u8] {
        let offset = (y * self.viewport.width as usize + x) * 4;
        &pixels[offset..offset + 4]
    }
}

#[tokio::test]
async fn ui_refresh_02_layout_and_interaction_editor_renders_hover_focus_and_disabled() {
    let content = text_editor::Content::with_text("设置 / 設定 / Settings");
    let make_editor = |enabled| {
        text_editor(&content)
            .padding(SETTINGS_CONTROL_PADDING)
            .size(SETTINGS_CONTROL_TEXT_SIZE)
            .style(settings_text_editor_style_for(enabled))
    };
    for theme in [Theme::Light, Theme::Dark] {
        let mut editor =
            Form::new(make_editor(true).on_action(|_| Message::None).into(), 420).await;
        let cursor = mouse::Cursor::Available(Point::new(210.0, 26.0));
        let active = editor.screenshot(&theme, mouse::Cursor::Unavailable);
        editor.update(
            Event::Mouse(mouse::Event::CursorMoved { position: Point::new(210.0, 26.0) }),
            cursor,
        );
        let hovered = editor.screenshot(&theme, cursor);
        editor.update(Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)), cursor);
        let focused = editor.screenshot(&theme, cursor);
        let mut disabled = Form::new(make_editor(false).into(), 420).await;
        disabled.update(Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)), cursor);
        let inactive = disabled.screenshot(&theme, cursor);

        assert_ne!(
            editor.pixel(&active, 210, 0),
            editor.pixel(&hovered, 210, 0),
            "{theme:?}: hover must change the editor border"
        );
        assert_ne!(
            editor.pixel(&hovered, 210, 0),
            editor.pixel(&focused, 210, 0),
            "{theme:?}: focus must change the editor border"
        );
        assert_ne!(
            editor.pixel(&active, 210, 0),
            disabled.pixel(&inactive, 210, 0),
            "{theme:?}: disabled editor must have a distinct fill"
        );
        assert!(disabled.messages.is_empty());
    }
}

#[tokio::test]
async fn ui_refresh_02_layout_and_interaction_number_aligns_with_shared_input() {
    use crate::app::components::system_settings_common::settings_text_input_style;
    use iced::widget::{row, text_input};

    let number = NumberInput::new(2.0, 0.0, 10.0, 1.0, 0, 1.0, |_| Message::None).settings_style();
    let input = text_input("", "2")
        .on_input(|_| Message::None)
        .padding(SETTINGS_CONTROL_PADDING)
        .size(SETTINGS_CONTROL_TEXT_SIZE)
        .style(settings_text_input_style);
    let form = Form::new(row![number, input].align_y(iced::Alignment::Center).into(), 420).await;
    let mut fields = Layout::new(&form.node).children();
    let number = fields.next().unwrap();
    let input = fields.next().unwrap();
    assert!((number.bounds().height - input.bounds().height).abs() < 0.5);
    assert!((number.bounds().center_y() - input.bounds().center_y()).abs() < 0.5);
}

fn click(form: &mut Form<'_>, point: Point) {
    let cursor = mouse::Cursor::Available(point);
    for event in [
        mouse::Event::ButtonPressed(mouse::Button::Left),
        mouse::Event::ButtonReleased(mouse::Button::Left),
    ] {
        form.update(Event::Mouse(event), cursor);
    }
}

#[tokio::test]
async fn ui_refresh_02_preserves_values_and_disabled_states() {
    use crate::app::components::system_settings_common::{
        danger_action_btn_style, primary_action_btn_style, rounded_action_btn_style,
        settings_checkbox_style, settings_toggle,
    };
    use crate::app::message::{GeneralSettingsMessage, SettingsMessage};
    use iced::widget::{button, checkbox, column, text};

    let toggle_message = |enabled| {
        Message::Settings(SettingsMessage::General(GeneralSettingsMessage::LaunchAtLoginToggled(
            enabled,
        )))
    };
    let mut form = Form::new(
        column![
            settings_toggle(false).on_toggle(toggle_message),
            settings_toggle(true),
            checkbox(true)
                .label("功能 / Capability")
                .on_toggle(|checked| Message::CopyCode(checked.to_string()))
                .style(settings_checkbox_style),
            button(text("确认")).padding([8, 14]).style(primary_action_btn_style),
            button(text("取消"))
                .on_press(Message::None)
                .padding([8, 14])
                .style(rounded_action_btn_style),
            button(text("删除")).padding([8, 14]).style(danger_action_btn_style),
            button(text("确认"))
                .on_press(Message::CopyCode("fixture confirmation".into()))
                .padding([8, 14])
                .style(primary_action_btn_style),
        ]
        .spacing(12)
        .into(),
        420,
    )
    .await;
    let centers: Vec<_> =
        Layout::new(&form.node).children().map(|child| child.bounds().center()).collect();

    click(&mut form, centers[0]);
    assert!(matches!(
        form.messages.as_slice(),
        [Message::Settings(SettingsMessage::General(
            GeneralSettingsMessage::LaunchAtLoginToggled(true)
        ))]
    ));
    form.messages.clear();
    for index in [1, 3, 5] {
        click(&mut form, centers[index]);
    }
    assert!(form.messages.is_empty(), "disabled controls must not publish messages");
    click(&mut form, centers[2]);
    click(&mut form, centers[4]);
    assert!(
        matches!(form.messages.as_slice(), [Message::CopyCode(value), Message::None] if value == "false")
    );
    form.messages.clear();
    click(&mut form, centers[6]);
    assert!(
        matches!(form.messages.as_slice(), [Message::CopyCode(value)] if value == "fixture confirmation")
    );
}

#[tokio::test]
async fn ui_refresh_02_preserves_password_masking() {
    use crate::app::components::system_settings_common::settings_text_input_style;
    use iced::widget::text_input;

    for theme in [Theme::Light, Theme::Dark] {
        let make = |fixture, secure| {
            text_input("", fixture)
                .on_input(Message::CopyCode)
                .secure(secure)
                .padding(SETTINGS_CONTROL_PADDING)
                .size(SETTINGS_CONTROL_TEXT_SIZE)
                .style(settings_text_input_style)
                .into()
        };
        let mut first = Form::new(make("test-fixture-ab", true), 420).await;
        let mut second = Form::new(make("test-fixture-xy", true), 420).await;
        let mut clear = Form::new(make("test-fixture-ab", false), 420).await;
        let masked = first.screenshot(&theme, mouse::Cursor::Unavailable);
        assert_eq!(masked, second.screenshot(&theme, mouse::Cursor::Unavailable));
        assert_ne!(masked, clear.screenshot(&theme, mouse::Cursor::Unavailable));
    }
}

#[tokio::test]
async fn ui_refresh_02_layout_and_interaction_renders_fixture_form() {
    use crate::app::assets::Icon;
    use crate::app::components::system_settings_common::{
        danger_action_btn_style, icon_btn, primary_action_btn_style, rounded_action_btn_style,
        settings_checkbox_style, settings_field_label, settings_select, settings_text_input_style,
        settings_toggle,
    };
    use iced::widget::{button, checkbox, column, container, row, text, text_input};
    use iced::{Alignment, Length};

    let long_path = "/fixtures/目录/設定/workspace/".repeat(6);
    let content = text_editor::Content::with_text(&format!("fixture_path = \"{long_path}\""));
    for (theme, theme_name) in [(Theme::Light, "light"), (Theme::Dark, "dark")] {
        for (width, label) in [
            (420, "简体中文 · 一个用于检查窄弹窗换行和文本截断的较长设置标签"),
            (640, "繁體中文 · 設定路徑與驗證狀態"),
            (960, "English · Settings path and validation status"),
        ] {
            let number =
                NumberInput::new(4.0, 0.0, 10.0, 1.0, 0, 1.0, |_| Message::None).settings_style();
            let element = container(
                column![
                    settings_field_label(label).width(Length::Fill),
                    row![
                        icon_btn(Icon::Check, "Validate path", Some(Message::None)),
                        text_input("", &long_path)
                            .on_input(Message::CopyCode)
                            .padding(SETTINGS_CONTROL_PADDING)
                            .size(SETTINGS_CONTROL_TEXT_SIZE)
                            .style(settings_text_input_style),
                    ]
                    .spacing(8)
                    .align_y(Alignment::Center),
                    number,
                    text_editor(&content)
                        .height(96)
                        .padding(SETTINGS_CONTROL_PADDING)
                        .size(SETTINGS_CONTROL_TEXT_SIZE)
                        .style(settings_text_editor_style_for(false)),
                    settings_select(Vec::<String>::new(), None, Message::CopyCode)
                        .placeholder("载入中 / 載入中 / Loading")
                        .width(Length::Fill),
                    settings_select(Vec::<String>::new(), None, Message::CopyCode)
                        .placeholder("载入失败 / 載入失敗 / Could not load")
                        .width(Length::Fill),
                    row![
                        settings_toggle(true).on_toggle(|_| Message::None),
                        settings_toggle(false),
                        checkbox(true)
                            .label("权限 / 權限 / Capability")
                            .on_toggle(|_| Message::None)
                            .style(settings_checkbox_style),
                    ]
                    .spacing(12)
                    .align_y(Alignment::Center),
                    row![
                        button(text("保存 / Save"))
                            .on_press(Message::None)
                            .padding([8, 14])
                            .style(primary_action_btn_style),
                        button(text("取消 / Cancel"))
                            .on_press(Message::None)
                            .padding([8, 14])
                            .style(rounded_action_btn_style),
                        button(text("删除 / Delete"))
                            .padding([8, 14])
                            .style(danger_action_btn_style),
                    ]
                    .spacing(8)
                    .align_y(Alignment::Center),
                ]
                .spacing(10),
            )
            .padding(16)
            .width(Length::Fill)
            .into();
            let mut form = Form::new(element, width).await;
            let column = Layout::new(&form.node).children().next().unwrap();
            let bounds = column.bounds();
            let input_row = column.children().nth(1).unwrap();
            let icon = input_row.children().next().unwrap().children().next().unwrap().bounds();
            let input = input_row.children().nth(1).unwrap();
            let input_bounds = input.bounds();
            let text_bounds = input.children().next().unwrap().bounds();
            for child in [icon, text_bounds] {
                assert!((input_bounds.center_y() - child.center_y()).abs() < 0.5);
            }
            for field in column.children() {
                let field = field.bounds();
                assert!(field.x >= bounds.x && field.x + field.width <= bounds.x + bounds.width);
                assert!(field.y >= bounds.y && field.y + field.height <= form.viewport.height);
            }

            let cursor = mouse::Cursor::Available(input_bounds.center());
            click(&mut form, input_bounds.center());
            let pixels = form.screenshot(&theme, cursor);
            let background = form.pixel(&pixels, 0, 0)[0];
            assert!(
                (icon.y as usize..(icon.y + icon.height) as usize).any(|y| {
                    (icon.x as usize..(icon.x + icon.width) as usize)
                        .any(|x| form.pixel(&pixels, x, y)[0].abs_diff(background) > 100)
                }),
                "{theme:?}: the form icon must contrast with the page"
            );
            if let Some(path) = std::env::var_os("SHIJU_SETTINGS_PREVIEW_DIR") {
                image::save_buffer(
                    std::path::Path::new(&path).join(format!("controls-{theme_name}-{width}.png")),
                    &pixels,
                    width,
                    form.viewport.height as u32,
                    image::ColorType::Rgba8,
                )
                .unwrap();
            }
        }
    }
}
