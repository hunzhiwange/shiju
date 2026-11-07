//! Exercise the workflow editor with the same layout, event and drawing paths as the desktop UI.

use super::*;
use crate::app::i18n::I18n;
use crate::apps::workflow::model::load_document_from_value;
use crate::apps::workflow::state::WorkflowStructuredNodeEdit;
use iced::advanced::widget::{Id, Operation, operation};
use iced::advanced::{clipboard, renderer::Headless};
use iced::keyboard::{self, Key, key::Named};
use iced::{Event, Font, Rectangle, Renderer, Size, mouse};
use iced_runtime::user_interface::{Cache, UserInterface};
use vw_config_types::ui::DesktopLocale;

type Ui<'a> = UserInterface<'a, Message, Theme, Renderer>;

async fn renderer() -> Renderer {
    {
        let mut fonts = iced::advanced::graphics::text::font_system().write().unwrap();
        for font in crate::fonts::load_all() {
            fonts.load_font(font);
        }
    }
    Renderer::new(Font::with_name("Noto Sans CJK SC"), 14.0.into(), Some("tiny-skia"))
        .await
        .unwrap()
}

fn scene<'a>(view: Element<'a, Message>, renderer: &mut Renderer, size: Size<u32>) -> Ui<'a> {
    UserInterface::build(
        view,
        Size::new(size.width as f32, size.height as f32),
        Cache::new(),
        renderer,
    )
}

fn send(ui: &mut Ui<'_>, renderer: &mut Renderer, point: Point, events: &[Event]) -> Vec<Message> {
    let mut messages = Vec::new();
    ui.update(
        events,
        mouse::Cursor::Available(point),
        renderer,
        &mut clipboard::Null,
        &mut messages,
    );
    messages
}

fn click(ui: &mut Ui<'_>, renderer: &mut Renderer, point: Point) -> Vec<Message> {
    send(
        ui,
        renderer,
        point,
        &[
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
        ],
    )
}

fn key(ui: &mut Ui<'_>, renderer: &mut Renderer, key: Key, text: Option<&str>) -> Vec<Message> {
    send(
        ui,
        renderer,
        Point::ORIGIN,
        &[Event::Keyboard(keyboard::Event::KeyPressed {
            modified_key: key.clone(),
            key,
            physical_key: keyboard::key::Physical::Unidentified(
                keyboard::key::NativeCode::Unidentified,
            ),
            location: keyboard::Location::Standard,
            modifiers: keyboard::Modifiers::empty(),
            text: text.map(Into::into),
            repeat: false,
        })],
    )
}

fn capture(
    ui: &mut Ui<'_>,
    renderer: &mut Renderer,
    theme: &Theme,
    size: Size<u32>,
    name: &str,
    point: Point,
) -> Vec<u8> {
    // draw() asks the widget tree for a fresh overlay after update().
    assert!(
        send(
            ui,
            renderer,
            point,
            &[Event::Window(iced::window::Event::RedrawRequested(iced::time::Instant::now()),)]
        )
        .is_empty()
    );
    ui.draw(
        renderer,
        theme,
        &iced::advanced::renderer::Style { text_color: theme.palette().text },
        mouse::Cursor::Available(point),
    );
    let pixels = renderer.screenshot(size, 1.0, theme.palette().background);
    if let Some(directory) = std::env::var_os("SHIJU_UI_REFRESH_36_SCREENSHOTS") {
        ::image::save_buffer(
            std::path::PathBuf::from(directory).join(format!("{name}.png")),
            &pixels,
            size.width,
            size.height,
            ::image::ColorType::Rgba8,
        )
        .unwrap();
    }
    pixels
}

#[derive(Default, Debug)]
struct LayoutBounds {
    labels: Vec<(String, Rectangle)>,
    inputs: Vec<(String, Rectangle)>,
    scrolls: Vec<(Rectangle, Vector)>,
}

impl Operation for LayoutBounds {
    fn traverse(&mut self, children: &mut dyn FnMut(&mut dyn Operation)) {
        children(self);
    }
    fn text(&mut self, _: Option<&Id>, bounds: Rectangle, value: &str) {
        self.labels.push((value.into(), bounds));
    }
    fn text_input(
        &mut self,
        _: Option<&Id>,
        bounds: Rectangle,
        input: &mut dyn operation::TextInput,
    ) {
        self.inputs.push((input.text().into(), bounds));
    }
    fn scrollable(
        &mut self,
        _: Option<&Id>,
        bounds: Rectangle,
        _: Rectangle,
        offset: Vector,
        _: &mut dyn operation::Scrollable,
    ) {
        self.scrolls.push((bounds, offset));
    }
}

fn bounds(ui: &mut Ui<'_>, renderer: &Renderer) -> LayoutBounds {
    let mut bounds = LayoutBounds::default();
    ui.operate(renderer, &mut bounds);
    bounds
}

fn position(controls: &[(String, Rectangle)], value: &str) -> Rectangle {
    controls
        .iter()
        .find(|(text, _)| text == value)
        .unwrap_or_else(|| panic!("missing {value:?} in {controls:?}"))
        .1
}

fn fixture() -> WorkflowState {
    let loaded = load_document_from_value(
        None,
        serde_yaml::from_str(
            r#"
app:
  name: Fixture
workflow:
  graph:
    nodes:
      - id: extractor
        position: {x: 120, y: 120}
        data:
          type: parameter-extractor
          title: "资料擷取 / Data extractor with a long title"
          desc: "流程测试 / 流程測試"
          query: [start, query]
          model:
            provider: "fixture/虚构供应商/模型系列/very-long-path/to/a/provider/for/fixture-models"
            name: fixture-model
            mode: chat
          parameters:
            - {name: parameter-key, type: string, description: Test}
    edges: []
"#,
        )
        .unwrap(),
    )
    .unwrap();
    let mut state = WorkflowState::default();
    state.apply_loaded(loaded, (960.0, 520.0));
    state.open_edit_node_editor(Some("extractor")).unwrap();
    state
}

#[tokio::test]
async fn ui_refresh_36_layout_and_interaction() {
    let mut renderer = renderer().await;
    let mut state = fixture();
    for (width, locale, mode, theme) in [
        (640, DesktopLocale::SimplifiedChinese, "zh-cn-light", Theme::Light),
        (960, DesktopLocale::TraditionalChinese, "zh-tw-dark", Theme::Dark),
        (1050, DesktopLocale::English, "en-light", Theme::Light),
        (1050, DesktopLocale::English, "en-dark", Theme::Dark),
    ] {
        let i18n = I18n::new(locale);
        let size = Size::new(width, 520);
        let mut ui = scene(view(i18n, &state), &mut renderer, size);
        capture(&mut ui, &mut renderer, &theme, size, mode, Point::ORIGIN);
        let geometry = bounds(&mut ui, &renderer);
        let save = position(&geometry.labels, i18n.__("保存"));
        let title = position(&geometry.inputs, &state.node_editor.as_ref().unwrap().title);
        assert!((title.center_y() - save.center_y()).abs() < 0.5);
        let (scroll, _) = geometry.scrolls.last().copied().expect("editor body");
        let query = position(&geometry.inputs, "start.query");
        assert!(query.x >= scroll.x + 7.0);
        assert!(query.x + query.width <= scroll.x + scroll.width - 7.0);
        for tab in ["配置", "描述", "下一步", "高级 DSL"] {
            let tab = position(&geometry.labels, i18n.__(tab));
            assert!(tab.x + tab.width <= width as f32 - 20.0 && tab.y < scroll.y);
        }
        let wheel = Event::Mouse(mouse::Event::WheelScrolled {
            delta: mouse::ScrollDelta::Pixels { x: 0.0, y: -280.0 },
        });
        let gutter = Point::new(scroll.x + scroll.width - 2.0, scroll.center_y());
        let messages = send(&mut ui, &mut renderer, gutter, std::slice::from_ref(&wheel));
        assert!(
            messages.iter().all(|message| matches!(
                message,
                Message::WorkflowTool(WorkflowMessage::NodeEditorScrollCaptured)
            )),
            "{messages:?}"
        );
        let after = bounds(&mut ui, &renderer);
        assert!(after.scrolls.last().unwrap().1.y > 0.0);
        assert_eq!(position(&after.labels, i18n.__("保存")), save);
        assert!(matches!(
            click(&mut ui, &mut renderer, save.center()).as_slice(),
            [Message::WorkflowTool(WorkflowMessage::SubmitNodeEditor)]
        ));
        assert!(matches!(
            send(&mut ui, &mut renderer, Point::new(100.0, 470.0), &[wheel]).as_slice(),
            [Message::WorkflowTool(WorkflowMessage::PanBy(_))]
        ));
        let drawer_margin = Point::new(width as f32 - 4.0, 450.0);
        assert!(click(&mut ui, &mut renderer, drawer_margin).is_empty());
        let close = Point::new(width as f32 - 34.0, save.center_y());
        assert!(matches!(
            click(&mut ui, &mut renderer, close).as_slice(),
            [Message::WorkflowTool(WorkflowMessage::CloseNodeEditor)]
        ));
    }

    let editor = state.node_editor.as_mut().unwrap();
    editor.active_tab = WorkflowNodeEditorTab::AdvancedDsl;
    editor.validation.field_errors.push(super::state::WorkflowNodeValidationError {
        path: "fixture".into(),
        message: "示例路径無效".into(),
    });
    let size = Size::new(1050, 520);
    let mut ui =
        scene(view(I18n::new(DesktopLocale::TraditionalChinese), &state), &mut renderer, size);
    capture(&mut ui, &mut renderer, &Theme::Dark, size, "dsl-validation", Point::ORIGIN);
    let point = Point::new(730.0, 350.0);
    let messages = send(
        &mut ui,
        &mut renderer,
        point,
        &[Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right))],
    );
    let [Message::WorkflowTool(WorkflowMessage::NodeEditorDslOpenContextMenu { x, y })] =
        messages.as_slice()
    else {
        panic!("expected DSL context menu, got {messages:?}");
    };
    let position = (*x, *y);
    let cache = ui.into_cache();
    state.node_editor_dsl_context_menu_open = true;
    state.node_editor_dsl_context_menu_pos = Some(position);
    let mut ui = UserInterface::build(
        view(I18n::new(DesktopLocale::TraditionalChinese), &state),
        Size::new(1050.0, 520.0),
        cache,
        &mut renderer,
    );
    capture(&mut ui, &mut renderer, &Theme::Dark, size, "dsl-context-menu", point);
    assert!(matches!(
        click(&mut ui, &mut renderer, Point::new(100.0, 350.0)).as_slice(),
        [Message::WorkflowTool(WorkflowMessage::NodeEditorDslCloseContextMenu)]
    ));
}

#[tokio::test]
async fn ui_refresh_36_preserves_values_and_disabled_states() {
    let i18n = I18n::new(DesktopLocale::English);
    let mut renderer = renderer().await;
    let state = fixture();
    let size = Size::new(1050, 520);
    let mut ui = scene(view(i18n, &state), &mut renderer, size);
    let geometry = bounds(&mut ui, &renderer);
    let query = position(&geometry.inputs, "start.query");
    let model = position(&geometry.inputs, "fixture-model");
    for (input, query_field) in [(query, true), (model, false)] {
        assert!(click(&mut ui, &mut renderer, input.center()).is_empty());
        assert!(key(&mut ui, &mut renderer, Key::Named(Named::End), None).is_empty());
        let messages = key(&mut ui, &mut renderer, Key::Character("x".into()), Some("x"));
        match (query_field, messages.as_slice()) {
            (
                true,
                [
                    Message::WorkflowTool(WorkflowMessage::NodeEditorStructuredFieldEdit(
                        WorkflowStructuredNodeEdit::Query(value),
                    )),
                ],
            ) => assert_eq!(value, "start.queryx"),
            (
                false,
                [
                    Message::WorkflowTool(WorkflowMessage::NodeEditorStructuredFieldEdit(
                        WorkflowStructuredNodeEdit::ModelName(value),
                    )),
                ],
            ) => assert_eq!(value, "fixture-modelx"),
            _ => panic!("unexpected field edit {messages:?}"),
        }
    }

    let parameter = position(&geometry.inputs, "parameter-key").center();
    let (scroll, _) = geometry.scrolls.last().copied().unwrap();
    if !scroll.contains(parameter) {
        send(
            &mut ui,
            &mut renderer,
            Point::new(scroll.x + scroll.width - 2.0, scroll.center_y()),
            &[Event::Mouse(mouse::Event::WheelScrolled {
                delta: mouse::ScrollDelta::Pixels { x: 0.0, y: scroll.center_y() - parameter.y },
            })],
        );
    }
    let offset = bounds(&mut ui, &renderer).scrolls.last().unwrap().1;
    let parameter = parameter - offset;
    assert!(scroll.contains(parameter), "{parameter:?} outside {scroll:?} after {offset:?}");
    assert!(click(&mut ui, &mut renderer, parameter).is_empty());
    assert!(key(&mut ui, &mut renderer, Key::Named(Named::End), None).is_empty());
    assert!(matches!(
        key(&mut ui, &mut renderer, Key::Character("x".into()), Some("x")).as_slice(),
        [Message::WorkflowTool(WorkflowMessage::NodeEditorStructuredFieldEdit(
            WorkflowStructuredNodeEdit::ParameterName(0, value)
        ))] if value == "parameter-keyx"
    ));

    let size = Size::new(420, 380);
    let mut empty = scene(build_app_switcher(i18n, &WorkflowState::default()), &mut renderer, size);
    assert!(click(&mut empty, &mut renderer, Point::new(40.0, 18.0)).is_empty());
    assert!(key(&mut empty, &mut renderer, Key::Named(Named::Enter), None).is_empty());

    let mut state = fixture();
    state.saved_apps = (0..20)
        .map(|index| super::state::WorkflowSavedAppSummary {
            uuid: format!("saved-{index}"),
            name: format!("App {index} {}", "很長的工作流名稱".repeat(16)),
            description: String::new(),
            created_at_ms: 0,
            updated_at_ms: 0,
        })
        .collect();
    state.local_uuid = Some("saved-0".into());
    let mut ui = scene(build_app_switcher(i18n, &state), &mut renderer, size);
    let trigger = Point::new(40.0, 18.0);
    assert!(click(&mut ui, &mut renderer, trigger).is_empty());
    assert!(
        send(&mut ui, &mut renderer, Point::ORIGIN, &[Event::Mouse(mouse::Event::CursorLeft),])
            .is_empty()
    );
    let idle = capture(&mut ui, &mut renderer, &Theme::Light, size, "switcher-open", Point::ORIGIN);
    let (menu, _) = bounds(&mut ui, &renderer).scrolls.last().copied().expect("select menu");
    assert!(menu.x >= 0.0 && menu.x + menu.width <= 420.0);
    assert!(menu.y >= 38.0 && menu.y + menu.height <= 380.0);
    let mut previous = idle.clone();
    for (y, name) in [(18.0, "switcher-hover-first"), (56.0, "switcher-hover-second")] {
        let point = Point::new(menu.center_x(), menu.y + y);
        assert!(
            send(
                &mut ui,
                &mut renderer,
                point,
                &[Event::Mouse(mouse::Event::CursorMoved { position: point })]
            )
            .is_empty()
        );
        let hovered = capture(&mut ui, &mut renderer, &Theme::Light, size, name, point);
        assert!(hovered != idle && hovered != previous);
        previous = hovered;
    }
    assert!(
        send(&mut ui, &mut renderer, Point::ORIGIN, &[Event::Mouse(mouse::Event::CursorLeft)])
            .is_empty()
    );
    let exited =
        capture(&mut ui, &mut renderer, &Theme::Light, size, "switcher-exit", Point::ORIGIN);
    assert!(exited == idle);
    assert!(
        send(
            &mut ui,
            &mut renderer,
            menu.center(),
            &[Event::Mouse(mouse::Event::WheelScrolled {
                delta: mouse::ScrollDelta::Pixels { x: 0.0, y: -270.0 },
            }),]
        )
        .is_empty()
    );
    assert!(bounds(&mut ui, &renderer).scrolls.last().unwrap().1.y > 0.0);
    capture(&mut ui, &mut renderer, &Theme::Dark, size, "switcher-scrolled", Point::ORIGIN);
    assert!(key(&mut ui, &mut renderer, Key::Named(Named::Escape), None).is_empty());
    assert!(click(&mut ui, &mut renderer, trigger).is_empty());
    assert!(key(&mut ui, &mut renderer, Key::Named(Named::Home), None).is_empty());
    assert!(
        matches!(
            key(&mut ui, &mut renderer, Key::Named(Named::Enter), None).as_slice(),
            [Message::WorkflowTool(WorkflowMessage::OpenSavedApp(id))] if id == "saved-0"
        ),
        "shared selector must submit the focused option exactly once"
    );
}
