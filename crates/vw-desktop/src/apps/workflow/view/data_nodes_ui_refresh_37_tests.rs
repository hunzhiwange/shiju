//! Render and exercise the structured workflow fields through the Iced runtime.

use super::*;
use crate::app::i18n::I18n;
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
    assert!(
        send(
            ui,
            renderer,
            point,
            &[Event::Window(iced::window::Event::RedrawRequested(iced::time::Instant::now(),))]
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
    if let Some(directory) = std::env::var_os("SHIJU_UI_REFRESH_37_SCREENSHOTS") {
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
struct Bounds {
    labels: Vec<(String, Rectangle)>,
    inputs: Vec<(String, Rectangle)>,
    scrolls: Vec<(Rectangle, Vector)>,
}

impl Operation for Bounds {
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

fn bounds(ui: &mut Ui<'_>, renderer: &Renderer) -> Bounds {
    let mut result = Bounds::default();
    ui.operate(renderer, &mut result);
    result
}

fn position(controls: &[(String, Rectangle)], value: &str) -> Rectangle {
    controls
        .iter()
        .find(|(text, _)| text == value)
        .unwrap_or_else(|| panic!("missing {value:?} in {controls:?}"))
        .1
}

fn insert(ui: &mut Ui<'_>, renderer: &mut Renderer, input: Rectangle, value: &str) -> Vec<Message> {
    assert!(click(ui, renderer, input.center()).is_empty());
    assert!(key(ui, renderer, Key::Named(Named::End), None).is_empty());
    key(ui, renderer, Key::Character(value.into()), Some(value))
}

#[tokio::test]
async fn ui_refresh_37_layout_and_interaction() {
    let mut renderer = renderer().await;
    let condition = state::WorkflowListConditionDraft {
        raw_condition: serde_yaml::Value::Null,
        key: "订单.名稱".into(),
        comparison_operator: "contains".into(),
        value: serde_yaml::Value::Null,
        value_input: "[alpha, beta]".into(),
    };
    for (width, locale, theme, name) in [
        (640, DesktopLocale::SimplifiedChinese, Theme::Light, "list-zh-cn"),
        (960, DesktopLocale::TraditionalChinese, Theme::Dark, "list-zh-tw"),
        (640, DesktopLocale::English, Theme::Dark, "list-en"),
    ] {
        let i18n = I18n::new(locale);
        let size = Size::new(width, 440);
        let mut ui = scene(
            build_list_operator_section(
                i18n,
                "start.items",
                std::slice::from_ref(&condition),
                true,
                false,
                "",
                "asc",
                false,
                "",
            ),
            &mut renderer,
            size,
        );
        capture(&mut ui, &mut renderer, &theme, size, name, Point::ORIGIN);
        let geometry = bounds(&mut ui, &renderer);
        let operator = position(&geometry.inputs, "contains");
        let value = position(&geometry.inputs, "[alpha, beta]");
        assert!((operator.center_y() - value.center_y()).abs() < 0.5);
        assert!(value.x + value.width <= width as f32);
        assert!(matches!(
            insert(&mut ui, &mut renderer, operator, "x").as_slice(),
            [Message::WorkflowTool(WorkflowMessage::NodeEditorDataNodeEdit(
                WorkflowDataNodeEdit::ListConditionOperator(0, value)
            ))] if value == "containsx"
        ));
        let remove = position(&geometry.labels, i18n.__("删除")).center();
        assert!(matches!(
            click(&mut ui, &mut renderer, remove).as_slice(),
            [Message::WorkflowTool(WorkflowMessage::NodeEditorDataNodeEdit(
                WorkflowDataNodeEdit::ListConditionRemove(0)
            ))]
        ));
    }
}

#[tokio::test]
async fn ui_refresh_37_preserves_values_and_disabled_states() {
    let mut renderer = renderer().await;
    let i18n = I18n::new(DesktopLocale::English);
    let state = WorkflowState {
        variable_editor: Some(state::WorkflowVariableEditorDraft {
            mode: state::WorkflowVariableEditorMode::EditConversation("fixture-id".into()),
            name: "fixture-variable".into(),
            description: "供節點讀取的長路徑 / fixtures/workflows/structured/vars/demo.yaml".into(),
            value_type: "array[object]".into(),
            raw_value_editor: text_editor::Content::with_text(&format!(
                "[{{\"商品\":\"{}\"}}]",
                "very-long-value-".repeat(30),
            )),
        }),
        ..WorkflowState::default()
    };
    let size = Size::new(420, 480);
    let mut ui = scene(build_variable_editor_modal(i18n, &state), &mut renderer, size);
    capture(&mut ui, &mut renderer, &Theme::Dark, size, "variable-narrow", Point::ORIGIN);
    let geometry = bounds(&mut ui, &renderer);
    let kind = position(&geometry.inputs, "array[object]");
    assert!(matches!(
        insert(&mut ui, &mut renderer, kind, "x").as_slice(),
        [Message::WorkflowTool(WorkflowMessage::VariableEditorTypeChanged(value))]
            if value == "array[object]x"
    ));
    let cancel = position(&geometry.labels, i18n.__("取消")).center();
    let save = position(&geometry.labels, i18n.__("保存变量")).center();
    assert!(cancel.y < size.height as f32 && save.y < size.height as f32);
    assert!(click(&mut ui, &mut renderer, Point::new(1.0, 1.0)).is_empty());
    assert!(matches!(
        click(&mut ui, &mut renderer, cancel).as_slice(),
        [Message::WorkflowTool(WorkflowMessage::CloseVariableEditor)]
    ));
    assert!(matches!(
        click(&mut ui, &mut renderer, save).as_slice(),
        [Message::WorkflowTool(WorkflowMessage::SubmitVariableEditor)]
    ));

    let mut start = workflow_fixture();
    start.open_edit_node_editor(Some("start")).unwrap();
    start.open_node_editor_start_variable_create();
    let draft = start.node_editor.as_ref().unwrap().start_variable_editor.as_ref().unwrap();
    let mut ui = scene(
        build_start_variable_select_default_field(i18n, &draft.variable),
        &mut renderer,
        size,
    );
    capture(&mut ui, &mut renderer, &Theme::Light, size, "default-without-options", Point::ORIGIN);
    assert!(click(&mut ui, &mut renderer, Point::new(48.0, 45.0)).is_empty());
    assert!(key(&mut ui, &mut renderer, Key::Named(Named::Enter), None).is_empty());
}

#[tokio::test]
async fn ui_refresh_37_select_keyboard_uses_stable_type_key() {
    let mut renderer = renderer().await;
    let i18n = I18n::new(DesktopLocale::TraditionalChinese);
    let size = Size::new(420, 380);
    let mut ui = scene(build_start_variable_type_selector(i18n, "text-input"), &mut renderer, size);
    let trigger = Point::new(30.0, 19.0);
    assert!(click(&mut ui, &mut renderer, trigger).is_empty());
    capture(&mut ui, &mut renderer, &Theme::Light, size, "type-open", trigger);
    assert!(key(&mut ui, &mut renderer, Key::Named(Named::Escape), None).is_empty());
    assert!(key(&mut ui, &mut renderer, Key::Named(Named::Enter), None).is_empty());
    assert!(key(&mut ui, &mut renderer, Key::Named(Named::ArrowDown), None).is_empty());
    assert!(matches!(
        key(&mut ui, &mut renderer, Key::Named(Named::Enter), None).as_slice(),
        [Message::WorkflowTool(WorkflowMessage::NodeEditorStartVariableEditorTypeChanged(value))]
            if value == "paragraph"
    ));
}

fn workflow_fixture() -> WorkflowState {
    let loaded = crate::apps::workflow::model::load_document_from_value(
        None,
        serde_yaml::from_str(
            r#"
app:
  name: Structured field fixtures
workflow:
  graph:
    nodes:
      - id: start
        position: {x: 100, y: 100}
        data:
          type: start
          title: "起始變數"
          variables: []
      - id: http
        position: {x: 250, y: 100}
        data:
          type: http-request
          title: Fixture HTTP
          method: post
          url: ""
    edges: []
"#,
        )
        .unwrap(),
    )
    .unwrap();
    let mut state = WorkflowState::default();
    state.apply_loaded(loaded, (960.0, 520.0));
    state
}

#[tokio::test]
async fn ui_refresh_37_start_file_editor_keeps_actions_visible() {
    let mut renderer = renderer().await;
    let mut state = workflow_fixture();
    state.open_edit_node_editor(Some("start")).unwrap();
    state.open_node_editor_start_variable_create();
    let draft = state.node_editor.as_mut().unwrap().start_variable_editor.as_mut().unwrap();
    draft.variable.input_type = "file-list".into();
    draft.variable.variable = "fixture_files".into();
    draft.variable.allowed_file_types = vec!["document".into(), "image".into(), "custom".into()];
    draft.variable.allowed_file_upload_methods = vec!["local_file".into(), "remote_url".into()];
    draft.variable.max_length_input = "6".into();
    draft.variable.default_file_values =
        vec![format!("https://example.invalid/測試/{}/fixture.pdf", "long-file-path/".repeat(16))];
    draft.show_default_file_url_input = true;
    let size = Size::new(420, 480);
    let i18n = I18n::new(DesktopLocale::TraditionalChinese);
    let mut ui = scene(build_start_variable_editor_modal(i18n, &state), &mut renderer, size);
    capture(&mut ui, &mut renderer, &Theme::Dark, size, "start-file-narrow", Point::ORIGIN);
    let geometry = bounds(&mut ui, &renderer);
    let (scroll, _) = geometry.scrolls.first().copied().expect("scrollable start variable form");
    let save = position(&geometry.labels, i18n.__("保存")).center();
    let cancel =
        geometry.labels.iter().rev().find(|(label, _)| label == i18n.__("取消")).unwrap().1;
    assert!(save.y > scroll.y + scroll.height && save.y < size.height as f32);
    let cursor = Point::new(scroll.x + scroll.width - 2.0, scroll.center_y());
    assert!(
        send(
            &mut ui,
            &mut renderer,
            cursor,
            &[Event::Mouse(mouse::Event::WheelScrolled {
                delta: mouse::ScrollDelta::Pixels { x: 0.0, y: -600.0 },
            })]
        )
        .is_empty()
    );
    let after_scroll = bounds(&mut ui, &renderer);
    assert!(after_scroll.scrolls[0].1.y > 0.0);
    assert_eq!(position(&after_scroll.labels, i18n.__("保存")).center(), save);
    capture(&mut ui, &mut renderer, &Theme::Dark, size, "start-file-scrolled", cursor);
    assert!(
        send(
            &mut ui,
            &mut renderer,
            cursor,
            &[Event::Mouse(mouse::Event::WheelScrolled {
                delta: mouse::ScrollDelta::Pixels { x: 0.0, y: -600.0 },
            })]
        )
        .is_empty()
    );
    capture(&mut ui, &mut renderer, &Theme::Dark, size, "start-file-path", cursor);
    assert!(matches!(
        click(&mut ui, &mut renderer, cancel.center()).as_slice(),
        [Message::WorkflowTool(WorkflowMessage::NodeEditorStartCloseVariableEditor)]
    ));
}

#[tokio::test]
async fn ui_refresh_37_http_body_and_invalid_save_keep_existing_messages() {
    let mut renderer = renderer().await;
    let i18n = I18n::new(DesktopLocale::English);
    let mut validation = state::WorkflowNodeEditorValidation::default();
    validation.field_errors.push(state::WorkflowNodeValidationError {
        path: "http.url".into(),
        message: "Fixture URL is invalid".into(),
    });
    let headers = text_editor::Content::with_text("- key: Accept\n  value: application/json");
    let params = text_editor::Content::with_text("- key: locale\n  value: 繁體中文");
    let json = format!("{{\"商品\":\"{}\"}}", "sample-".repeat(75));
    let mut body = text_editor::Content::with_text(&json);
    let size = Size::new(640, 1040);
    let mut ui = scene(
        build_http_request_section(
            i18n,
            &validation,
            "post",
            "https://example.invalid",
            "basic",
            "fixture-user",
            "fixture-pass",
            "",
            &headers,
            &params,
            "json",
            &body,
            "10",
            "60",
            "20",
            true,
            WorkflowNodeRetryDraft { enabled: false, max_retries: 2, retry_interval: 500 },
            "none",
        ),
        &mut renderer,
        size,
    );
    capture(&mut ui, &mut renderer, &Theme::Light, size, "http-long-json", Point::ORIGIN);
    let geometry = bounds(&mut ui, &renderer);
    let label = position(&geometry.labels, i18n.__("请求体 YAML"));
    let body_point = Point::new(label.x + 20.0, label.y + label.height + 20.0);
    let _ = click(&mut ui, &mut renderer, body_point);
    let messages = key(&mut ui, &mut renderer, Key::Character("x".into()), Some("x"));
    let [
        Message::WorkflowTool(WorkflowMessage::NodeEditorHttpRequestEdit(
            WorkflowHttpRequestEdit::Body(action),
        )),
    ] = messages.as_slice()
    else {
        panic!("expected one HTTP body edit, got {messages:?}");
    };
    assert!(action.is_edit());
    let action = action.clone();
    drop(ui);
    assert_eq!(body.text(), json);
    body.perform(action);
    assert_eq!(body.text(), format!("x{json}"));

    let mut state = workflow_fixture();
    state.open_edit_node_editor(Some("http")).unwrap();
    state.node_editor.as_mut().unwrap().validation = validation;
    let size = Size::new(960, 520);
    let mut ui = scene(build_node_editor_modal(i18n, &state), &mut renderer, size);
    capture(&mut ui, &mut renderer, &Theme::Dark, size, "http-invalid-save", Point::ORIGIN);
    let geometry = bounds(&mut ui, &renderer);
    let save = position(&geometry.labels, i18n.__("保存")).center();
    assert!(matches!(
        click(&mut ui, &mut renderer, save).as_slice(),
        [Message::WorkflowTool(WorkflowMessage::SubmitNodeEditor)]
    ));
    assert!(key(&mut ui, &mut renderer, Key::Named(Named::Enter), None).is_empty());
    drop(ui);
    assert!(state.submit_node_editor().is_err());
    assert!(state.node_editor.is_some());
}

#[tokio::test]
async fn ui_refresh_37_empty_condition_and_variable_panel_render() {
    let mut renderer = renderer().await;
    let validation = state::WorkflowNodeEditorValidation::default();
    let i18n = I18n::new(DesktopLocale::SimplifiedChinese);
    let size = Size::new(640, 420);
    let mut ui = scene(build_if_else_visual_section(i18n, &[], &validation), &mut renderer, size);
    capture(&mut ui, &mut renderer, &Theme::Dark, size, "condition-empty", Point::ORIGIN);
    let geometry = bounds(&mut ui, &renderer);
    let add = position(&geometry.labels, i18n.__("新增分支")).center();
    assert!(matches!(
        click(&mut ui, &mut renderer, add).as_slice(),
        [Message::WorkflowTool(WorkflowMessage::NodeEditorIfElseAddCase)]
    ));

    let state = WorkflowState {
        variable_panel: Some(WorkflowVariablePanelKind::Environment),
        ..WorkflowState::default()
    };
    let size = Size::new(420, 360);
    let mut ui = scene(build_variable_panel_modal(i18n, &state), &mut renderer, size);
    capture(&mut ui, &mut renderer, &Theme::Light, size, "environment-empty", Point::ORIGIN);
    let geometry = bounds(&mut ui, &renderer);
    let add = position(&geometry.labels, i18n.__("新增环境变量")).center();
    assert!(matches!(
        click(&mut ui, &mut renderer, add).as_slice(),
        [Message::WorkflowTool(WorkflowMessage::OpenCreateEnvironmentVariableEditor)]
    ));
}

#[tokio::test]
async fn ui_refresh_37_long_default_menu_keeps_hover_and_scroll_local() {
    let mut renderer = renderer().await;
    let mut state = workflow_fixture();
    state.open_edit_node_editor(Some("start")).unwrap();
    state.open_node_editor_start_variable_create();
    let variable =
        &mut state.node_editor.as_mut().unwrap().start_variable_editor.as_mut().unwrap().variable;
    variable.options =
        (0..25).map(|index| format!("{index} {}", "长选项長選項".repeat(22))).collect();
    variable.default_value = variable.options[0].clone();
    let i18n = I18n::new(DesktopLocale::SimplifiedChinese);
    let size = Size::new(420, 300);
    let mut ui =
        scene(build_start_variable_select_default_field(i18n, variable), &mut renderer, size);
    let geometry = bounds(&mut ui, &renderer);
    let clear = position(&geometry.labels, i18n.__("清空")).center();
    let trigger = Point::new(48.0, clear.y);
    assert!(click(&mut ui, &mut renderer, trigger).is_empty());

    assert!(
        send(&mut ui, &mut renderer, Point::ORIGIN, &[Event::Mouse(mouse::Event::CursorLeft)])
            .is_empty()
    );
    let idle =
        capture(&mut ui, &mut renderer, &Theme::Light, size, "long-menu-idle", Point::ORIGIN);
    let (menu, _) = bounds(&mut ui, &renderer).scrolls.last().copied().expect("default menu");
    assert!(menu.x >= 0.0 && menu.x + menu.width <= size.width as f32);
    assert!(menu.y > trigger.y && menu.y + menu.height <= size.height as f32);
    let mut previous = idle.clone();
    for (y, name) in [(18.0, "long-menu-first"), (56.0, "long-menu-second")] {
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
        assert!(hovered != idle && hovered != previous, "hover should follow the pointer");
        previous = hovered;
    }
    assert!(
        send(&mut ui, &mut renderer, Point::ORIGIN, &[Event::Mouse(mouse::Event::CursorLeft)])
            .is_empty()
    );
    let after_exit =
        capture(&mut ui, &mut renderer, &Theme::Light, size, "long-menu-exit", Point::ORIGIN);
    assert!(after_exit == idle, "highlight should clear when the cursor leaves");
    assert!(
        send(
            &mut ui,
            &mut renderer,
            menu.center(),
            &[Event::Mouse(mouse::Event::WheelScrolled {
                delta: mouse::ScrollDelta::Pixels { x: 0.0, y: -390.0 },
            })]
        )
        .is_empty()
    );
    assert!(bounds(&mut ui, &renderer).scrolls.last().unwrap().1.y > 0.0);
    capture(&mut ui, &mut renderer, &Theme::Light, size, "long-menu-scrolled", Point::ORIGIN);
    assert!(click(&mut ui, &mut renderer, clear).is_empty(), "menu must consume outside click");
    assert!(matches!(
        click(&mut ui, &mut renderer, clear).as_slice(),
        [Message::WorkflowTool(WorkflowMessage::NodeEditorStartVariableEditorDefaultChanged(value))]
            if value.is_empty()
    ));
}
