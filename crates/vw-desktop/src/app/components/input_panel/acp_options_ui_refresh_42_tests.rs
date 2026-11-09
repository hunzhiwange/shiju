//! Exercise the actual chat configuration views with an isolated desktop state.

use super::with_modal;
use crate::app::components::input_panel::{model_popover, reasoning_selector, task_mode};
use crate::app::components::overlays::AboveOverlay;
use crate::app::i18n::I18n;
use crate::app::message::{SettingsMessage, ViewMessage, chat};
use crate::app::state::{ModelSummary, ProviderModelsSummary};
use crate::app::task::TaskSplitMode;
use crate::app::{App, Message, message::ChatMessage};
use iced::advanced::widget::{Id, Operation, operation};
use iced::advanced::{clipboard, renderer::Headless};
use iced::keyboard::{self, Key, key::Named};
use iced::widget::{Space, button as action, column, container, row, text};
use iced::{Element, Length};
use iced::{Event, Font, Point, Rectangle, Renderer, Size, Theme, Vector, mouse};
use iced_runtime::futures::futures::StreamExt;
use iced_runtime::user_interface::{Cache, UserInterface};
use vw_config_types::ui::{DesktopLocale, GatewayClientSystemSettingsConfig};
use vw_gateway_client::vw_api_types::ReasoningEffortDto;
use vw_gateway_client::vw_api_types::acp_model_source::{
    AcpModelSource, AcpModelSourceCapability, AcpModelSourceRejectionReason,
};
use vw_gateway_client::vw_api_types::acp_session::{
    AcpAccessStatus, AcpAccessSupport, AcpChoice, AcpOptionKind, AcpSessionOption,
};

type Ui<'a> = UserInterface<'a, Message, Theme, Renderer>;
const MODEL_SCROLL: &str = "input_panel_model_popover_scroll";
const CONFIG_ID: &str = "fixture/backend-option";
const NEXT_VALUE: &str = "fixture/backend-value-2";

fn isolated(test: &str) -> bool {
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
    let test = format!("{}::{test}", module_path!().split_once("::").unwrap().1);
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", &test])
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
    runtime.acp_options.connected = true;
    runtime.acp_options.model_source_capabilities = vec![
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
    runtime.acp_options.options = vec![AcpSessionOption {
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

fn hover(ui: &mut Ui<'_>, renderer: &mut Renderer, point: Point) {
    let messages =
        send(ui, renderer, point, &[Event::Mouse(mouse::Event::CursorMoved { position: point })]);
    assert!(messages.is_empty(), "{messages:?}");
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

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ui_refresh_42_model_search_stays_visible_in_short_overlay() {
    if !isolated("ui_refresh_42_model_search_stays_visible_in_short_overlay") {
        return;
    }
    let mut app = fixture();
    app.show_model_popover = true;
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

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ui_refresh_42_model_results_keep_groups_and_actions() {
    if !isolated("ui_refresh_42_model_results_keep_groups_and_actions") {
        return;
    }
    let mut app = fixture();
    let mut renderer = renderer().await;
    let size = Size::new(640, 480);
    app.desktop_locale = DesktopLocale::English;
    app.model_settings.query = "model-13".into();
    for pending in [false, true] {
        app.current_session_runtime_mut().acp_options_pending = pending;
        let mut ui = scene(model_overlay(&app), &mut renderer, size);
        let group = label(&bounds(&mut ui, &renderer), "測試供應商");
        let row_y = group.y + group.height + 4.0 + 8.0 + 18.0;
        let point = Point::new(group.x + 25.0, row_y);
        let rest = capture(
            &mut ui,
            &mut renderer,
            &Theme::Light,
            size,
            &format!("model-results-{pending}"),
            Point::ORIGIN,
        );
        hover(&mut ui, &mut renderer, point);
        let hovered = capture(
            &mut ui,
            &mut renderer,
            &Theme::Light,
            size,
            &format!("model-hover-{pending}"),
            point,
        );
        hover(&mut ui, &mut renderer, Point::ORIGIN);
        let left = capture(
            &mut ui,
            &mut renderer,
            &Theme::Light,
            size,
            &format!("model-left-{pending}"),
            Point::ORIGIN,
        );
        assert_eq!(rest, left);
        let messages = click(&mut ui, &mut renderer, point);
        if pending {
            assert_eq!(rest, hovered);
            assert!(messages.is_empty(), "{messages:?}");
        } else {
            assert_ne!(rest, hovered);
            assert!(matches!(
                messages.as_slice(),
                [Message::Chat(ChatMessage::ModelSelected(model))] if model == "fixture/model-13"
            ));
            let info = Point::new(group.x + group.width - 11.0, row_y);
            assert!(matches!(
                click(&mut ui, &mut renderer, info).as_slice(),
                [Message::View(ViewMessage::OpenSystemSettingsModelDetail(provider, model))]
                    if provider == "fixture" && model == "model-13"
            ));
        }
    }
    let i18n = I18n::new(app.desktop_locale);
    app.model_settings.providers.clear();
    for loading in [true, false] {
        app.model_settings.loading = loading;
        let mut ui = scene(model_overlay(&app), &mut renderer, size);
        let name = if loading {
            i18n.__("加载中…")
        } else {
            i18n.__("暂无可选模型（请先在系统设置里启用模型）")
        };
        inside(label(&bounds(&mut ui, &renderer), name), viewport(size));
        capture(
            &mut ui,
            &mut renderer,
            &Theme::Dark,
            size,
            &format!("model-loading-{loading}"),
            Point::ORIGIN,
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ui_refresh_42_layout_and_interaction() {
    if !isolated("ui_refresh_42_layout_and_interaction") {
        return;
    }
    let mut app = fixture();
    let mut renderer = renderer().await;
    for (width, viewport_width, locale, theme) in [
        (420, 420, DesktopLocale::TraditionalChinese, Theme::Dark),
        (640, 664, DesktopLocale::SimplifiedChinese, Theme::Light),
        (960, 984, DesktopLocale::English, Theme::Dark),
    ] {
        app.desktop_locale = locale;
        let i18n = I18n::new(locale);
        let size = Size::new(viewport_width, 560);
        let runtime = app.current_session_runtime_ref().unwrap();
        let form = task_mode::task_mode_form(
            &app,
            Some(runtime),
            true,
            "10",
            "fixture/model-00",
            TaskSplitMode::FollowSystem,
            Some("codex".into()),
            None,
            &runtime.task_mode_subtasks,
        );
        let mut ui = scene(container(form).padding(12).into(), &mut renderer, size);
        capture(&mut ui, &mut renderer, &theme, size, &format!("task-{width}"), Point::ORIGIN);
        let layout = bounds(&mut ui, &renderer);
        let split_label = label(&layout, i18n.__("任务拆分"));
        let split = layout
            .focusable
            .iter()
            .find(|bounds| (bounds.center_y() - split_label.center_y()).abs() < 0.5)
            .expect("task split selector aligned with its field label");
        inside(*split, viewport(size));
        let point = split.center();
        assert!(click(&mut ui, &mut renderer, point).is_empty());
        assert!(key(&mut ui, &mut renderer, point, Key::Named(Named::ArrowDown), None).is_empty());
        capture(&mut ui, &mut renderer, &theme, size, &format!("task-menu-{width}"), point);
        let menu = bounds(&mut ui, &renderer)
            .scrolls
            .into_iter()
            .find(|(id, _, _)| id.as_ref() == Some(&Id::from("settings-select-menu")))
            .unwrap()
            .1;
        inside(menu, viewport(size));
        assert!(key(&mut ui, &mut renderer, point, Key::Named(Named::Escape), None).is_empty());
        assert!(click(&mut ui, &mut renderer, point).is_empty());
        assert!(key(&mut ui, &mut renderer, point, Key::Named(Named::ArrowDown), None).is_empty());
        assert!(matches!(
            key(&mut ui, &mut renderer, point, Key::Named(Named::Enter), None).as_slice(),
            [Message::Chat(ChatMessage::TaskModeTaskSplitModeChanged(TaskSplitMode::Enabled))]
        ));

        let popover = reasoning_selector::reasoning_effort_popover(
            &app,
            "fixture/model-00",
            &[None, Some(ReasoningEffortDto::High)],
            None,
        );
        let mut ui = scene(popover, &mut renderer, size);
        capture(&mut ui, &mut renderer, &theme, size, &format!("reasoning-{width}"), Point::ORIGIN);
        assert!(matches!(
            click(&mut ui, &mut renderer, Point::new(234.0, 73.0)).as_slice(),
            [Message::Chat(ChatMessage::ReasoningEffortSelected(Some(ReasoningEffortDto::High)))]
        ));
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ui_refresh_42_preserves_values_and_disabled_states() {
    if !isolated("ui_refresh_42_preserves_values_and_disabled_states") {
        return;
    }
    let mut app = fixture();
    let mut renderer = renderer().await;
    let size = Size::new(420, 420);
    for (requesting, pending, theme) in
        [(false, false, Theme::Light), (true, false, Theme::Dark), (false, true, Theme::Dark)]
    {
        let runtime = app.current_session_runtime_mut();
        runtime.acp_options_open = true;
        runtime.is_requesting = requesting;
        runtime.acp_options_pending = pending;
        runtime.acp_options.access_status = Some(AcpAccessStatus {
            requested: false,
            backend: "fixture".into(),
            version: None,
            support: AcpAccessSupport::Failed,
            explanation: "測試失敗狀態".into(),
        });
        let base = container(Space::new()).width(Length::Fill).height(Length::Fill).into();
        let mut ui = scene(with_modal(&app, base), &mut renderer, size);
        let layout = bounds(&mut ui, &renderer);
        let viewport = viewport(size);
        let title = label(&layout, "ACP 会话选项");
        let close = label(&layout, "关闭");
        let body = layout.scrolls[0].1;
        inside(title, viewport);
        inside(close, viewport);
        assert!(title.y + title.height < body.y && close.y > body.y + body.height);
        capture(
            &mut ui,
            &mut renderer,
            &theme,
            size,
            &format!("acp-{requesting}-{pending}"),
            Point::ORIGIN,
        );
        scroll(&mut ui, &mut renderer, body.center(), -1500.0);
        let layout = bounds(&mut ui, &renderer);
        assert!(layout.scrolls[0].2.y > 0.0);
        assert_eq!(close, label(&layout, "关闭"));
        let offset = layout.scrolls[0].2;
        let selected = label(&layout, "✓ 當前選項").center() - offset;
        let choice = label(&layout, "待選選項").center() - offset;
        assert!(body.contains(selected) && body.contains(choice));
        hover(&mut ui, &mut renderer, choice);
        capture(
            &mut ui,
            &mut renderer,
            &theme,
            size,
            &format!("acp-scrolled-{requesting}-{pending}"),
            choice,
        );
        assert!(click(&mut ui, &mut renderer, selected).is_empty());
        let selected = click(&mut ui, &mut renderer, choice);
        if requesting || pending {
            assert!(selected.is_empty(), "{selected:?}");
        } else {
            assert!(matches!(
                selected.as_slice(),
                [Message::Chat(ChatMessage::AcpOptionSelected(selection))]
                    if selection.kind == (AcpOptionKind::Config { id: CONFIG_ID.into() })
                        && selection.value == NEXT_VALUE
            ));
        }
        assert!(matches!(
            click(&mut ui, &mut renderer, close.center()).as_slice(),
            [Message::Chat(ChatMessage::AcpOptionsClose)]
        ));
    }
    app.active_session_id = Some("fixture-b".into());
    app.current_session_runtime_mut().acp_options_open = true;
    let _ = chat::update(&mut app, ChatMessage::AcpOptionsClose);
    assert!(!app.current_session_runtime().acp_options_open);
    app.active_session_id = Some("fixture-a".into());
    let runtime = app.current_session_runtime();
    assert!(runtime.acp_options_open && runtime.acp_options_pending);
    assert_eq!(runtime.acp_options.options[0].current, "fixture/backend-value-1");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn help_rollout_09_session_model_source_keeps_status_visible() {
    if !isolated("help_rollout_09_session_model_source_keeps_status_visible") {
        return;
    }
    let mut app = fixture();
    let mut renderer = renderer().await;
    let size = Size::new(420, 600);
    for (locale, theme) in
        [(DesktopLocale::English, Theme::Dark), (DesktopLocale::SimplifiedChinese, Theme::Light)]
    {
        app.desktop_locale = locale;
        app.app_theme = theme.clone();
        let runtime = app.current_session_runtime_mut();
        runtime.acp_options_open = true;
        runtime.acp_model_source = AcpModelSource::SessionModel;
        let base = container(Space::new()).width(Length::Fill).height(Length::Fill).into();
        let mut ui = scene(with_modal(&app, base), &mut renderer, size);
        let layout = bounds(&mut ui, &renderer);
        label(&layout, "完全访问授权：已关闭");
        label(&layout, "请先配置供应商凭据");
        label(&layout, "切换来源将创建新的 ACP 会话；原生上下文不继承");
        label(
            &layout,
            &format!("当前客户端 Provider/model：{}", app.current_session_runtime().model),
        );
        let heading = label(&layout, "模型来源");
        let icon = Point::new(heading.x + heading.width + 15.0, heading.center_y());
        let frame = |ui: &mut Ui<'_>, renderer: &mut Renderer, suffix: &str| {
            capture(
                ui,
                renderer,
                &theme,
                size,
                &format!("help-09-{locale:?}-model-source-{suffix}"),
                Point::ORIGIN,
            )
        };
        let idle = frame(&mut ui, &mut renderer, "idle");
        assert!(click(&mut ui, &mut renderer, icon).is_empty());
        assert!(idle != frame(&mut ui, &mut renderer, "open"), "model source help must open");
        assert!(!layout.labels.iter().any(|(text, _)| text == "使用执行主机上的配置与登录状态"));
        assert!(
            key(&mut ui, &mut renderer, Point::ORIGIN, Key::Named(Named::Escape), None).is_empty()
        );
        assert!(
            idle == frame(&mut ui, &mut renderer, "closed"),
            "Escape must dismiss model source help"
        );
        assert!(
            click(&mut ui, &mut renderer, label(&layout, "智能体原生模型").center()).is_empty()
        );
        assert!(matches!(
            click(&mut ui, &mut renderer, label(&layout, "关闭").center()).as_slice(),
            [Message::Chat(ChatMessage::AcpOptionsClose)]
        ));
        drop(ui);

        app.current_session_runtime_mut().acp_model_source = AcpModelSource::CliLocal;
        let mut popover = scene(model_overlay(&app), &mut renderer, size);
        let layout = bounds(&mut popover, &renderer);
        let heading = label(&layout, "模型来源");
        let icon = Point::new(heading.x + heading.width + 15.0, heading.center_y());
        let idle = frame(&mut popover, &mut renderer, "popover-idle");
        assert!(click(&mut popover, &mut renderer, icon).is_empty());
        assert!(
            idle != frame(&mut popover, &mut renderer, "popover-open"),
            "model popover help must open"
        );
        assert!(
            key(&mut popover, &mut renderer, Point::ORIGIN, Key::Named(Named::Escape), None)
                .is_empty()
        );
        assert!(idle == frame(&mut popover, &mut renderer, "popover-closed"));
        assert!(matches!(
            click(&mut popover, &mut renderer, label(&layout, "客户端模型").center()).as_slice(),
            [Message::Chat(ChatMessage::AcpModelSourceSelected(AcpModelSource::SessionModel))]
        ));
        drop(popover);

        let draft = app.current_session_runtime_mut().draft.take();
        let mut locked = scene(model_overlay(&app), &mut renderer, size);
        let layout = bounds(&mut locked, &renderer);
        label(&layout, "会话开始后不能切换模型");
        let locked_choice = label(&layout, "客户端模型").center();
        let idle = frame(&mut locked, &mut renderer, "locked-idle");
        hover(&mut locked, &mut renderer, locked_choice);
        let hovered = capture(
            &mut locked,
            &mut renderer,
            &theme,
            size,
            &format!("help-09-{locale:?}-model-source-locked-hover"),
            locked_choice,
        );
        assert!(idle != hovered, "locked model source must show a tooltip on hover");
        assert!(click(&mut locked, &mut renderer, locked_choice).is_empty());
        hover(&mut locked, &mut renderer, Point::ORIGIN);
        assert!(idle == frame(&mut locked, &mut renderer, "locked-leave"));
        drop(locked);
        app.current_session_runtime_mut().draft = draft;
    }
}
