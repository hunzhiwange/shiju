//! Exercise the actual chat configuration views with an isolated desktop state.

use super::with_modal;
use crate::app::components::input_panel::model_popover;
use crate::app::components::overlays::AboveOverlay;
use crate::app::i18n::I18n;
use crate::app::message::{SettingsMessage, ViewMessage, chat};
use crate::app::state::{ModelSummary, ProviderModelsSummary};
use crate::app::{App, Message, message::ChatMessage};
use iced::advanced::widget::{Id, Operation, operation};
use iced::advanced::{clipboard, renderer::Headless};
use iced::keyboard::{self, Key, key::Named};
use iced::widget::{Space, button as action, column, container, row, text};
use iced::{Element, Length};
use iced::{Event, Font, Point, Rectangle, Renderer, Size, Theme, Vector, mouse};
use iced_runtime::futures::futures::StreamExt;
use iced_runtime::user_interface::{Cache, UserInterface};
use config_ui::GatewayClientSystemSettingsConfig;
use vw_api_acp_model::acp_model_source::{
    AcpModelSource, AcpModelSourceCapability, AcpModelSourceRejectionReason,
};
use vw_api_acp_model::acp_session::{
    AcpChoice, AcpOptionKind, AcpSessionOption,
    AcpSessionSelection,
};

type Ui<'a> = UserInterface<'a, Message, Theme, Renderer>;
const MODEL_SCROLL: &str = "input_panel_model_popover_scroll";
const CONFIG_ID: &str = "fixture/backend-option";
const NEXT_VALUE: &str = "fixture/backend-value-2";

fn isolated(test: &str) -> bool {
    isolated_exact(&format!("{}::{test}", module_path!()))
}

fn isolated_exact(test: &str) -> bool {
    const CHILD: &str = "SHIJU_UI_REFRESH_42_CHILD";
    if std::env::var_os(CHILD).is_some() {
        return true;
    }
    let home = tempfile::tempdir().unwrap();
    let config = GatewayClientSystemSettingsConfig {
        host: "[".into(),
        servers: Vec::new(),
        ..Default::default()
    };
    std::fs::write(
        home.path().join("gateway-client-bootstrap.json"),
        serde_json::to_vec(&config).unwrap(),
    )
    .unwrap();
    let (_, test) = test.split_once("::").unwrap();
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", test])
        .env(CHILD, "1")
        .env("VIBEWINDOW_CONFIG_DIR", home.path())
        .env("VIBEWINDOW_WORKSPACE", home.path())
        .current_dir(home.path())
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success() && stdout.contains("1 passed; 0 failed"),
        "{stdout}\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    false
}

fn fixture() -> App {
    assert_eq!(crate::app::config::gateway_client_endpoint().normalized_host(), "[");
    let mut app = App::new().0;
    app.active_session_id = Some("fixture-a".into());
    let runtime = app.current_session_runtime_mut();
    runtime.draft = Some(crate::app::state::SessionDraft {
        project_path: "/workspace".into(),
        directory: "/workspace".into(),
        history_len: 0,
    });
    runtime.acp_agent = Some("codex".into());
    runtime.acp_model_source = AcpModelSource::SessionModel;
    runtime.acp_options_state.options.connected = true;
    runtime.acp_options_state.options.model_source_capabilities = vec![
        AcpModelSourceCapability::Supported { source: AcpModelSource::SessionModel },
        AcpModelSourceCapability::Unsupported {
            source: AcpModelSource::CliLocal,
            reason: AcpModelSourceRejectionReason::MissingCredentials,
        },
    ];
    runtime.model = format!("fixture/{}", "long-path/".repeat(12));
    runtime.auto_model = false;
    runtime.task_mode_subtasks = vec!["多行任務\nSecond line".into(), "Fallback editor".into()];
    runtime.task_mode_subtask_editors =
        vec![iced::widget::text_editor::Content::with_text(&runtime.task_mode_subtasks[0])];
    runtime.acp_options_state.options.options = vec![AcpSessionOption {
        kind: AcpOptionKind::Config { id: CONFIG_ID.into() },
        name: "動態配置項目與較長的後端顯示名稱".into(),
        current: "fixture/backend-value-1".into(),
        choices: vec![
            AcpChoice { id: "fixture/backend-value-1".into(), name: "當前選項".into() },
            AcpChoice { id: NEXT_VALUE.into(), name: "待選選項".into() },
        ],
    }];
    app.model_settings.providers = vec![ProviderModelsSummary {
        id: "fixture".into(),
        name: "測試供應商".into(),
        models: (0..15)
            .map(|index| ModelSummary {
                id: format!("model-{index:02}"),
                name: if index == 13 {
                    "測試長名稱".repeat(10)
                } else {
                    format!("測試模型 {index:02}")
                },
                enabled: true,
                toolcall: true,
                attachment: false,
                context_limit: 4096,
                detail: serde_json::json!({}),
            })
            .collect(),
    }];
    app
}

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
    UserInterface::build(view, viewport(size).size(), Cache::new(), renderer)
}

fn viewport(size: Size<u32>) -> Rectangle {
    Rectangle::with_size(Size::new(size.width as f32, size.height as f32))
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

fn key(
    ui: &mut Ui<'_>,
    renderer: &mut Renderer,
    point: Point,
    key: Key,
    text: Option<&str>,
) -> Vec<Message> {
    send(
        ui,
        renderer,
        point,
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



fn scroll(ui: &mut Ui<'_>, renderer: &mut Renderer, point: Point, y: f32) {
    let messages = send(
        ui,
        renderer,
        point,
        &[Event::Mouse(mouse::Event::WheelScrolled {
            delta: mouse::ScrollDelta::Pixels { x: 0.0, y },
        })],
    );
    assert!(messages.is_empty(), "{messages:?}");
}

fn capture(
    ui: &mut Ui<'_>,
    renderer: &mut Renderer,
    theme: &Theme,
    size: Size<u32>,
    name: &str,
    point: Point,
) -> Vec<u8> {
    let messages = send(
        ui,
        renderer,
        point,
        &[Event::Window(iced::window::Event::RedrawRequested(iced::time::Instant::now()))],
    );
    assert!(messages.is_empty(), "{messages:?}");
    // Iced rebuilds the overlay between update and draw, as it does in the application.
    ui.draw(
        renderer,
        theme,
        &iced::advanced::renderer::Style { text_color: theme.palette().text },
        mouse::Cursor::Available(point),
    );
    let pixels = renderer.screenshot(size, 1.0, theme.palette().background);
    if let Some(directory) = std::env::var_os("SHIJU_UI_REFRESH_42_SCREENSHOTS") {
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

#[derive(Default)]
struct Bounds {
    labels: Vec<(String, Rectangle)>,
    inputs: Vec<(String, Rectangle)>,
    scrolls: Vec<(Option<Id>, Rectangle, Vector)>,
    focusable: Vec<Rectangle>,
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
        id: Option<&Id>,
        bounds: Rectangle,
        _: Rectangle,
        offset: Vector,
        _: &mut dyn operation::Scrollable,
    ) {
        self.scrolls.push((id.cloned(), bounds, offset));
    }

    fn focusable(&mut self, _: Option<&Id>, bounds: Rectangle, _: &mut dyn operation::Focusable) {
        self.focusable.push(bounds);
    }
}

fn bounds(ui: &mut Ui<'_>, renderer: &Renderer) -> Bounds {
    let mut result = Bounds::default();
    ui.operate(renderer, &mut result);
    result
}

fn model_scroll(layout: &Bounds) -> &(Option<Id>, Rectangle, Vector) {
    layout
        .scrolls
        .iter()
        .find(|(id, _, _)| id.as_ref() == Some(&Id::from(MODEL_SCROLL)))
        .expect("scrollable model results")
}

fn label(layout: &Bounds, name: &str) -> Rectangle {
    layout
        .labels
        .iter()
        .find(|(text, _)| text == name)
        .unwrap_or_else(|| panic!("missing {name:?} in {:?}", layout.labels))
        .1
}

fn inside(inner: Rectangle, outer: Rectangle) {
    assert!(
        inner.width > 0.0
            && inner.height > 0.0
            && inner.x >= outer.x
            && inner.y >= outer.y
            && inner.x + inner.width <= outer.x + outer.width
            && inner.y + inner.height <= outer.y + outer.height,
        "{inner:?} outside {outer:?}",
    );
}

fn model_overlay(app: &App) -> Element<'_, Message> {
    let overlay = AboveOverlay::new(
        model_popover::model_toggle_button(app, false, "fixture/model-00", true),
        model_popover::model_popover_content(app, false, "fixture/model-00"),
    )
    .show(true)
    .block_background_events(true)
    .gap(6.0)
    .on_close(Message::View(ViewMessage::CloseModelPopover));
    column![
        row![
            Space::new().width(Length::Fill),
            action(text("fixture send")).on_press(Message::Chat(ChatMessage::SendPressed))
        ],
        Space::new().height(Length::Fill),
        overlay,
    ]
    .padding(10)
    .into()
}

// 宿主保留：跨域 ACP 能力读回、设置更新与宿主浮层交互契约。
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ui_refresh_42_model_search_stays_visible_in_short_overlay() {
    if !isolated("ui_refresh_42_model_search_stays_visible_in_short_overlay") {
        return;
    }
    let mut app = fixture();
    app.popover_visibility.model = true;
    let mut renderer = renderer().await;
    let size = Size::new(420, 300);
    let mut ui = scene(model_overlay(&app), &mut renderer, size);
    capture(&mut ui, &mut renderer, &Theme::Dark, size, "model-compact", Point::ORIGIN);
    let layout = bounds(&mut ui, &renderer);
    let search = layout
        .inputs
        .iter()
        .find(|(value, _)| value == I18n::new(app.desktop_locale).__("搜索模型…"))
        .expect("search input is reachable above the composer")
        .1;
    let body = model_scroll(&layout).1;
    inside(search, viewport(size));
    assert!(body.height >= 72.0, "only {}px for results", body.height);
    scroll(&mut ui, &mut renderer, body.center(), -1500.0);
    assert!(model_scroll(&bounds(&mut ui, &renderer)).2.y > 0.0);
    assert!(click(&mut ui, &mut renderer, search.center()).is_empty());
    let messages =
        key(&mut ui, &mut renderer, search.center(), Key::Character("m".into()), Some("m"));
    let [Message::Settings(SettingsMessage::ModelQueryChanged(query))] = messages.as_slice() else {
        panic!("expected a model query, got {messages:?}");
    };
    assert_eq!(query, "m");
    assert!(
        key(&mut ui, &mut renderer, search.center(), Key::Named(Named::Enter), None).is_empty()
    );
    let cache = ui.into_cache();
    let task = crate::app::message::settings::update(
        &mut app,
        SettingsMessage::ModelQueryChanged(query.clone()),
    );
    let mut ui =
        UserInterface::build(model_overlay(&app), viewport(size).size(), cache, &mut renderer);
    let mut actions = iced_runtime::task::into_stream(task).expect("query scrolls to the results");
    let Some(iced_runtime::Action::Widget(mut operation)) = actions.next().await else {
        panic!("query should reset the model scroll position");
    };
    ui.operate(&renderer, operation.as_mut());
    let filtered = bounds(&mut ui, &renderer);
    let body = model_scroll(&filtered);
    assert_eq!(body.2.y, 0.0);
    inside(label(&filtered, "測試供應商"), body.1);
    capture(&mut ui, &mut renderer, &Theme::Dark, size, "model-compact-filtered", Point::ORIGIN);
    let send = label(&filtered, "fixture send").center();
    assert!(matches!(
        click(&mut ui, &mut renderer, send).as_slice(),
        [Message::View(ViewMessage::CloseModelPopover)]
    ));
}

// 当前会话关闭不得影响其它会话的后端选项状态。
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ui_refresh_42_close_modal_is_scoped_to_current_session() {
    if !isolated("ui_refresh_42_close_modal_is_scoped_to_current_session") { return; }
    let mut app = fixture();
    app.current_session_runtime_mut().acp_options_state.open = true;
    app.current_session_runtime_mut().acp_options_state.pending = true;
    app.active_session_id = Some("fixture-b".into());
    app.current_session_runtime_mut().acp_options_state.open = true;
    let _ = chat::update(&mut app, ChatMessage::AcpOptionsClose);
    assert!(!app.current_session_runtime().acp_options_state.open);
    app.active_session_id = Some("fixture-a".into());
    let runtime = app.current_session_runtime();
    assert!(runtime.acp_options_state.open && runtime.acp_options_state.pending);
    assert_eq!(runtime.acp_options_state.options.options[0].current, "fixture/backend-value-1");
}



// 宿主保留：跨域 ACP 能力读回、设置更新与宿主浮层交互契约。
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ui_refresh_42_acp_effort_is_editable_between_turns() {
    if !isolated("ui_refresh_42_acp_effort_is_editable_between_turns") {
        return;
    }
    let mut app = fixture();
    let runtime = app.current_session_runtime_mut();
    runtime.acp_options_state.open = true;
    runtime.acp_options_state.options.connected = false;
    runtime.acp_options_state.options.options[0].kind = AcpOptionKind::Config { id: "reasoning_effort".into() };

    let mut renderer = renderer().await;
    let base = container(Space::new()).width(Length::Fill).height(Length::Fill).into();
    let mut ui = scene(with_modal(&app, base), &mut renderer, Size::new(520, 560));
    let body = bounds(&mut ui, &renderer).scrolls[0].1;
    scroll(&mut ui, &mut renderer, body.center(), -1500.0);
    let layout = bounds(&mut ui, &renderer);
    let choice = label(&layout, "待選選項").center() - layout.scrolls[0].2;
    assert!(body.contains(choice));
    let mut messages = click(&mut ui, &mut renderer, choice);
    assert_eq!(messages.len(), 1);
    let Message::Chat(message @ ChatMessage::AcpOptionSelected(_)) = messages.remove(0) else {
        panic!("expected an ACP option selection");
    };
    drop(ui);
    let _ = chat::update(&mut app, message);
    assert!(app.current_session_runtime().acp_options_state.pending);
}



pub(in crate::app::components::input_panel) async fn verify_effort_entry(test: &str) {
    if !isolated_exact(test) {
        return;
    }
    let mut app = fixture();
    let runtime = app.current_session_runtime_mut();
    runtime.draft = None;
    runtime.acp_agent = Some("copilot".into());
    let kind = AcpOptionKind::Config { id: "reasoning_effort".into() };
    runtime.acp_options_state.options.options[0].kind = kind.clone();
    let mut renderer = renderer().await;
    let size = Size::new(1120, 700);
    let panel = || container(Space::new()).width(Length::Fill).height(Length::Fill).into();
    {
        let mut ui = scene(super::super::view(&app), &mut renderer, size);
        // The input panel's drag wrapper forwards clicks but does not expose widget bounds.
        let entry = (65..180).step_by(5).any(|y| {
            (220..370).step_by(10).any(|x| {
                click(&mut ui, &mut renderer, Point::new(x as f32, y as f32))
                    .iter()
                    .any(|message| matches!(message, Message::Chat(ChatMessage::AcpOptionsOpen)))
            })
        });
        assert!(entry, "main input panel has no ACP options entry");
    }
    let _ = chat::update(&mut app, ChatMessage::AcpOptionsOpen);
    assert!(app.current_session_runtime().acp_options_state.open);

    let selection = |value: &str| {
        ChatMessage::AcpOptionSelected(AcpSessionSelection {
            kind: kind.clone(),
            value: value.into(),
        })
    };
    let options = app.current_session_runtime().acp_options_state.options;
    let _ = chat::update(
        &mut app,
        ChatMessage::AcpOptionsLoaded { session_id: "fixture-a".into(), result: Ok(options) },
    );
    let _ = chat::update(&mut app, selection("not-advertised"));
    assert!(!app.current_session_runtime().acp_options_state.pending);
    app.current_session_runtime_mut().request_lifecycle.is_requesting = true;
    let _ = chat::update(&mut app, selection(NEXT_VALUE));
    assert!(!app.current_session_runtime().acp_options_state.pending);
    app.current_session_runtime_mut().request_lifecycle.is_requesting = false;
    {
        let mut ui = scene(with_modal(&app, panel()), &mut renderer, size);
        let body = bounds(&mut ui, &renderer).scrolls[0].1;
        scroll(&mut ui, &mut renderer, body.center(), -1500.0);
        let layout = bounds(&mut ui, &renderer);
        let offset = layout.scrolls[0].2;
        for (name, value) in [("✓ 當前選項", "fixture/backend-value-1"), ("待選選項", NEXT_VALUE)]
        {
            let choice = label(&layout, name).center() - offset;
            assert!(matches!(
                click(&mut ui, &mut renderer, choice).as_slice(),
                [Message::Chat(ChatMessage::AcpOptionSelected(selection))] if selection.value == value
            ));
        }
    }
    let _ = chat::update(&mut app, selection(NEXT_VALUE));
    assert!(app.current_session_runtime().acp_options_state.pending);
    {
        let mut ui = scene(with_modal(&app, panel()), &mut renderer, size);
        let body = bounds(&mut ui, &renderer).scrolls[0].1;
        scroll(&mut ui, &mut renderer, body.center(), -1500.0);
        let layout = bounds(&mut ui, &renderer);
        let choice = label(&layout, "待選選項").center() - layout.scrolls[0].2;
        assert!(click(&mut ui, &mut renderer, choice).is_empty());
    }
    let _ = chat::update(
        &mut app,
        ChatMessage::AcpOptionsLoaded {
            session_id: "fixture-a".into(),
            result: Err("fixture rejection".into()),
        },
    );
    assert!(!app.current_session_runtime().acp_options_state.pending);
    assert_eq!(
        app.current_session_runtime().acp_options_state.options.options[0].current,
        "fixture/backend-value-1"
    );
    assert_eq!(app.active_toast.as_ref().unwrap().message, "fixture rejection");
    let mut ui = scene(with_modal(&app, panel()), &mut renderer, size);
    assert!(matches!(
        key(&mut ui, &mut renderer, Point::ORIGIN, Key::Named(Named::Escape), None).as_slice(),
        [Message::Chat(ChatMessage::AcpOptionsClose)]
    ));
}
