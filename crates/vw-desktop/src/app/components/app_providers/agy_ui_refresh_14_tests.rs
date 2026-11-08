use super::super::list::Help;
use super::super::{agy, dsh, edit, hermes, kimi, mcode, omp, openclaw, pi};
use crate::app::{
    App, Message,
    message::{
        SettingsMessage,
        app_providers::{self as events, edit::EditMessage},
    },
    state::app_providers::{
        self as state,
        edit::{EditForm, Mode},
    },
};
use iced::advanced::{
    Layout, clipboard, layout,
    renderer::{Headless, Style},
    widget::{
        Id, Tree,
        operation::{Operation, Scrollable},
    },
};
use iced::keyboard::{self, Key, key::Named};
use iced::widget::{Space, text_editor};
use iced::{Element, Event, Font, Length, Point, Rectangle, Renderer, Size, Theme, Vector, mouse};
use iced_runtime::user_interface::{Cache, UserInterface};
use vw_gateway_client::vw_api_types::app_providers::basic::ProviderBasic;

const PROVIDERS: [&str; 8] = ["agy", "dsh", "hermes", "kimi", "mcode", "omp", "openclaw", "pi"];
const MODEL: &str = "模型标识／模型識別 · fixture-model";
const KEY: &str = "fixture-key";
const REVISION: &str = "fixture-revision";

async fn renderer() -> Renderer {
    static FONTS: std::sync::Once = std::sync::Once::new();
    FONTS.call_once(|| {
        let mut fonts = iced::advanced::graphics::text::font_system().write().unwrap();
        for font in crate::fonts::load_all() {
            fonts.load_font(font);
        }
    });
    Renderer::new(Font::with_name("Noto Sans CJK SC"), 14.0.into(), Some("tiny-skia"))
        .await
        .expect("tiny-skia renderer")
}

fn fixture(provider: &str, create: bool, locale: &str) -> EditForm {
    let path = format!("https://example.invalid/{}", "fixture-path/".repeat(15));
    let mut form = EditForm::new(
        if create { Mode::Create } else { Mode::Update },
        "fixture-id".into(),
        ProviderBasic {
            name: format!("{locale} · {}", "fixture-provider ".repeat(7)),
            category: Some(locale.into()),
            notes: None,
            website_url: Some(path.clone()),
        },
        None,
    );
    form.key = KEY.into();
    form.replace_key = !create;

    macro_rules! declaration {
        ($provider:ident) => {{
            let mut draft = state::$provider::Draft::new();
            if !create {
                let imported = format!(
                    "{}\n# {locale}\n# {path}\n{}",
                    draft.declaration.text(),
                    "# fixture line\n".repeat(18),
                );
                draft.declaration = text_editor::Content::with_text(&imported);
                draft.has_saved_key = true;
                draft.revision = Some(REVISION.into());
                draft.apply = true;
            }
            form.$provider = Some(draft);
        }};
    }

    match provider {
        "agy" => {
            let mut draft = state::agy::Draft::new();
            if !create {
                draft.model = MODEL.into();
                draft.base_url = path;
                draft.has_saved_key = true;
                draft.revision = Some(REVISION.into());
            }
            form.agy = Some(draft);
        }
        "dsh" => declaration!(dsh),
        "hermes" => declaration!(hermes),
        "kimi" => declaration!(kimi),
        "mcode" => declaration!(mcode),
        "omp" => declaration!(omp),
        "openclaw" => declaration!(openclaw),
        "pi" => declaration!(pi),
        _ => unreachable!(),
    }
    form
}

fn fields(form: &EditForm) -> Element<'_, Message> {
    if let Some(draft) = &form.agy {
        agy::fields(form, draft, Help::test())
    } else if let Some(draft) = &form.dsh {
        dsh::fields(form, draft, Help::test())
    } else if let Some(draft) = &form.hermes {
        hermes::fields(form, draft, Help::test())
    } else if let Some(draft) = &form.kimi {
        kimi::fields(form, draft, Help::test())
    } else if let Some(draft) = &form.mcode {
        mcode::fields(form, draft, Help::test())
    } else if let Some(draft) = &form.omp {
        omp::fields(form, draft, Help::test())
    } else if let Some(draft) = &form.openclaw {
        openclaw::fields(form, draft, Help::test())
    } else {
        pi::fields(form, form.pi.as_ref().unwrap(), Help::test())
    }
}

fn base() -> Element<'static, Message> {
    Space::new().width(Length::Fill).height(Length::Fill).into()
}

fn overlay(form: &EditForm) -> Element<'_, Message> {
    edit::overlay(form, Help::test(), base())
}

struct Scene<'a> {
    ui: UserInterface<'a, Message, Theme, Renderer>,
    layout: layout::Node,
}

impl<'a> Scene<'a> {
    fn new(mut element: Element<'a, Message>, renderer: &mut Renderer, size: Size<u32>) -> Self {
        let viewport = Size::new(size.width as f32, size.height as f32);
        let mut tree = Tree::new(element.as_widget());
        let layout = element.as_widget_mut().layout(
            &mut tree,
            renderer,
            &layout::Limits::new(Size::ZERO, viewport),
        );
        let ui = UserInterface::build(element, viewport, Cache::new(), renderer);
        Self { ui, layout }
    }

    fn node(&self, path: &[usize]) -> Layout<'_> {
        path.iter().fold(Layout::new(&self.layout), |node, &index| {
            node.children().nth(index).unwrap_or_else(|| panic!("missing layout path {path:?}"))
        })
    }

    fn modal(&self, path: &[usize]) -> Rectangle {
        self.node(&[&[1, 1, 0, 0, 0], path].concat()).bounds()
    }

    fn send(
        &mut self,
        renderer: &mut Renderer,
        events: &[Event],
        cursor: mouse::Cursor,
    ) -> Vec<Message> {
        let mut messages = Vec::new();
        self.ui.update(events, cursor, renderer, &mut clipboard::Null, &mut messages);
        messages.retain(|event| !matches!(event, Message::None));
        messages
    }

    fn click(&mut self, renderer: &mut Renderer, point: Point) -> Vec<Message> {
        self.send(
            renderer,
            &[
                Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
                Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
            ],
            mouse::Cursor::Available(point),
        )
    }

    fn type_in(&mut self, renderer: &mut Renderer, point: Point, suffix: &str) -> Vec<Message> {
        assert!(self.click(renderer, point).is_empty());
        self.send(
            renderer,
            &[key(Named::End), commit(suffix), key(Named::Enter), key(Named::Escape)],
            mouse::Cursor::Available(point),
        )
    }
}

fn key(name: Named) -> Event {
    let key = Key::Named(name);
    Event::Keyboard(keyboard::Event::KeyPressed {
        modified_key: key.clone(),
        key,
        physical_key: keyboard::key::Physical::Unidentified(
            keyboard::key::NativeCode::Unidentified,
        ),
        location: keyboard::Location::Standard,
        modifiers: keyboard::Modifiers::empty(),
        text: None,
        repeat: false,
    })
}

fn commit(text: &str) -> Event {
    Event::InputMethod(iced::advanced::input_method::Event::Commit(text.into()))
}

fn assert_centered(a: Rectangle, b: Rectangle) {
    assert!((a.center().y - b.center().y).abs() < 0.5, "{a:?} vs {b:?}");
}

#[derive(Default)]
struct BodyScroll(Option<(Rectangle, Rectangle, Vector)>);

impl Operation for BodyScroll {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
        operate(self);
    }

    fn scrollable(
        &mut self,
        _id: Option<&Id>,
        bounds: Rectangle,
        content: Rectangle,
        translation: Vector,
        _state: &mut dyn Scrollable,
    ) {
        self.0.get_or_insert((bounds, content, translation));
    }
}

fn capture(
    ui: &mut UserInterface<'_, Message, Theme, Renderer>,
    renderer: &mut Renderer,
    theme: &Theme,
    size: Size<u32>,
    name: &str,
    cursor: mouse::Cursor,
) {
    let mut messages = Vec::new();
    ui.update(
        &[Event::Window(iced::window::Event::RedrawRequested(iced::time::Instant::now()))],
        cursor,
        renderer,
        &mut clipboard::Null,
        &mut messages,
    );
    assert!(messages.iter().all(|message| matches!(message, Message::None)));
    ui.draw(renderer, theme, &Style { text_color: theme.palette().text }, cursor);
    let pixels = renderer.screenshot(size, 1.0, theme.palette().background);
    if let Some(directory) = std::env::var_os("SHIJU_SETTINGS_PREVIEW_DIR") {
        image::save_buffer(
            std::path::PathBuf::from(directory).join(format!("{name}.png")),
            &pixels,
            size.width,
            size.height,
            image::ColorType::Rgba8,
        )
        .expect("save provider screenshot");
    }
}

fn snapshot(
    element: Element<'_, Message>,
    renderer: &mut Renderer,
    theme: &Theme,
    size: Size<u32>,
    name: &str,
) {
    let mut ui = UserInterface::build(
        element,
        Size::new(size.width as f32, size.height as f32),
        Cache::new(),
        renderer,
    );
    capture(&mut ui, renderer, theme, size, name, mouse::Cursor::Unavailable);
}

#[tokio::test]
async fn ui_refresh_14_layout_and_interaction() {
    let mut renderer = renderer().await;
    for provider in PROVIDERS {
        for (create, phase) in [(true, "create"), (false, "update")] {
            for (theme, theme_name) in [(Theme::Light, "light"), (Theme::Dark, "dark")] {
                for (width, locale) in
                    [(420, "测试配置"), (640, "測試設定"), (960, "Fixture settings")]
                {
                    let form = fixture(provider, create, locale);
                    snapshot(
                        fields(&form),
                        &mut renderer,
                        &theme,
                        Size::new(width, 690),
                        &format!("{provider}-{phase}-{theme_name}-fields-{width}"),
                    );
                    if width == 420 {
                        snapshot(
                            overlay(&form),
                            &mut renderer,
                            &theme,
                            Size::new(width, 690),
                            &format!("{provider}-{phase}-{theme_name}-dialog-{width}"),
                        );
                    }
                }
            }
            check_dialog(&mut renderer, provider, create, phase);
        }
    }
}

fn check_dialog(renderer: &mut Renderer, provider: &str, create: bool, phase: &str) {
    let mut form = fixture(provider, create, "简体／繁體 · Fixture");
    let size = Size::new(420, 690);
    let mut scene = Scene::new(overlay(&form), renderer, size);
    assert_centered(scene.modal(&[0, 0]), scene.modal(&[0, 1]));
    let body = scene.modal(&[1]);
    let footer = scene.modal(&[2]);
    assert!(body.y + body.height < footer.y);
    for index in 0..2 {
        assert_centered(scene.modal(&[2, index]), scene.modal(&[2, index, 0]));
    }

    let content = scene.modal(&[1, 0, 0]);
    assert!(content.x >= body.x + 7.0);
    assert!(content.x + content.width <= body.x + body.width - 7.0);
    let cursor = mouse::Cursor::Available(Point::new(body.x + 2.0, body.center().y));
    assert!(
        scene
            .send(
                renderer,
                &[Event::Mouse(mouse::Event::WheelScrolled {
                    delta: mouse::ScrollDelta::Pixels { x: 0.0, y: -1800.0 },
                })],
                cursor
            )
            .is_empty()
    );
    let mut scroll = BodyScroll::default();
    scene.ui.operate(renderer, &mut scroll);
    let (viewport, content, offset) = scroll.0.unwrap();
    assert!(content.height > viewport.height);
    assert!(offset.y > 0.0, "{provider} {phase}");
    capture(
        &mut scene.ui,
        renderer,
        &Theme::Dark,
        size,
        &format!("{provider}-{phase}-dialog-scrolled"),
        cursor,
    );

    let save = scene.modal(&[2, 1]).center();
    let cancel = scene.modal(&[2, 0]).center();
    for position in [save, cancel] {
        let cursor = mouse::Cursor::Available(position);
        assert!(
            scene
                .send(renderer, &[Event::Mouse(mouse::Event::CursorMoved { position })], cursor)
                .is_empty()
        );
        capture(
            &mut scene.ui,
            renderer,
            &Theme::Dark,
            size,
            &format!(
                "{provider}-{phase}-hover-{}",
                if position == save { "save" } else { "cancel" }
            ),
            cursor,
        );
    }
    assert!(
        scene
            .send(renderer, &[Event::Mouse(mouse::Event::CursorLeft)], mouse::Cursor::Unavailable)
            .is_empty()
    );
    assert!(matches!(
        scene.click(renderer, save).as_slice(),
        [Message::Settings(SettingsMessage::AppProviderEdit(EditMessage::Save))]
    ));
    assert!(matches!(
        scene.click(renderer, cancel).as_slice(),
        [Message::Settings(SettingsMessage::AppProviderEdit(EditMessage::Cancel))]
    ));
    drop(scene);

    form.saving = true;
    let mut busy = Scene::new(overlay(&form), renderer, size);
    assert!(busy.click(renderer, save).is_empty());
    assert!(busy.click(renderer, cancel).is_empty());
    capture(
        &mut busy.ui,
        renderer,
        &Theme::Dark,
        size,
        &format!("{provider}-{phase}-busy"),
        mouse::Cursor::Unavailable,
    );
}

fn handle_edit(app: &mut App, messages: Vec<Message>) {
    for message in messages {
        let Message::Settings(SettingsMessage::AppProviderEdit(event)) = message else {
            panic!("unexpected provider form message");
        };
        assert!(!matches!(event, EditMessage::Save));
        drop(events::edit::update(app, event));
    }
}

fn key_input<'a>(scene: &'a Scene<'_>, provider: &str) -> Layout<'a> {
    let panel = scene.node(&[1, 0]);
    let children = panel.children();
    let from_end = if provider == "agy" { 1 } else { 2 };
    let key_row = children.rev().nth(from_end).unwrap();
    key_row.children().nth(1).unwrap()
}

fn declaration_editor(scene: &Scene<'_>, provider: &str, create: bool) -> Rectangle {
    let index = match provider {
        "openclaw" => 4,
        "hermes" => 2,
        _ => 3 + usize::from(create),
    };
    scene.node(&[1, 0, index]).bounds()
}

fn check_declaration(app: &mut App, renderer: &mut Renderer, provider: &str, create: bool) {
    let form = app.app_providers.form.as_ref().unwrap();
    let size = Size::new(420, 690);
    let mut scene = Scene::new(fields(form), renderer, size);
    let editor = declaration_editor(&scene, provider, create);
    let point = Point::new(editor.x + 16.0, editor.y + 14.0);
    let focus = scene.click(renderer, point);
    let inserted = "fixture-edit 简体／繁體";
    let messages = scene.send(renderer, &[commit(inserted)], mouse::Cursor::Available(point));
    let apply = scene.node(&[1, 0]).children().next_back().unwrap();
    let toggle = apply.children().next().unwrap().bounds();
    let label = apply.children().nth(1).unwrap().bounds();
    assert_centered(toggle, label);
    let changes = scene.click(renderer, toggle.center());
    drop(scene);

    macro_rules! check {
        ($draft:ident, $variant:ident) => {{
            let contents = |app: &App| {
                app.app_providers.form.as_ref().unwrap().$draft.as_ref().unwrap().declaration.text()
            };
            let original = contents(app);
            assert!(matches!(messages.as_slice(), [
                Message::Settings(SettingsMessage::AppProviderEdit(EditMessage::$variant(
                    events::$draft::Field::Declaration(text_editor::Action::Edit(text_editor::Edit::Paste(value)))
                )))
            ] if value.as_str() == inserted));
            assert!(matches!(changes.as_slice(), [
                Message::Settings(SettingsMessage::AppProviderEdit(EditMessage::$variant(
                    events::$draft::Field::Apply(value)
                )))
            ] if *value == create));
            handle_edit(app, focus);
            handle_edit(app, messages);
            handle_edit(app, changes);
            let edited = contents(app);
            assert!(edited.contains(inserted));
            assert_eq!(edited.replacen(inserted, "", 1), original);
            let draft = app.app_providers.form.as_ref().unwrap().$draft.as_ref().unwrap();
            assert_eq!(draft.revision.as_deref(), (!create).then_some(REVISION));
            assert_eq!(draft.apply, create);
        }};
    }
    match provider {
        "dsh" => check!(dsh, Dsh),
        "hermes" => check!(hermes, Hermes),
        "kimi" => check!(kimi, Kimi),
        "mcode" => check!(mcode, Mcode),
        "omp" => check!(omp, Omp),
        "openclaw" => check!(openclaw, OpenClaw),
        "pi" => check!(pi, Pi),
        _ => unreachable!(),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn ui_refresh_14_preserves_values_and_disabled_states() {
    let mut renderer = renderer().await;
    let size = Size::new(420, 690);
    crate::app::config::with_editor_settings_gateway(|mut app, requests, _| {
        for provider in PROVIDERS {
            for create in [true, false] {
                app.app_providers.form = Some(fixture(provider, create, "简体／繁體 · Fixture"));
                let form = app.app_providers.form.as_ref().unwrap();
                let mut scene = Scene::new(fields(form), &mut renderer, size);
                let input = key_input(&scene, provider);
                assert_centered(input.bounds(), input.children().next().unwrap().bounds());
                let point = input.bounds().center();
                let updated = format!("{KEY}-updated");
                let edits = scene.type_in(&mut renderer, point, "-updated");
                assert!(matches!(edits.as_slice(), [
                    Message::Settings(SettingsMessage::AppProviderEdit(EditMessage::Key(value)))
                ] if value == &updated));
                drop(scene);
                handle_edit(&mut app, edits);
                let form = app.app_providers.form.as_ref().unwrap();
                assert_eq!(form.id, "fixture-id");
                assert_eq!(form.key, updated);

                if let Some(draft) = &form.agy {
                    let old_url = draft.base_url.clone();
                    let mut scene = Scene::new(fields(form), &mut renderer, size);
                    let input = scene.node(&[1, 0, 1, 1]).bounds().center();
                    let edits = scene.type_in(&mut renderer, input, "-new");
                    let model = if create { "-new".into() } else { format!("{MODEL}-new") };
                    assert!(matches!(edits.as_slice(), [
                        Message::Settings(SettingsMessage::AppProviderEdit(
                            EditMessage::Agy(events::agy::Field::Model(value))
                        ))
                    ] if value == &model));
                    let input = scene.node(&[1, 0, 2, 1]).bounds().center();
                    let url_edits = scene.type_in(&mut renderer, input, "/new");
                    assert!(matches!(url_edits.as_slice(), [
                        Message::Settings(SettingsMessage::AppProviderEdit(
                            EditMessage::Agy(events::agy::Field::BaseUrl(value))
                        ))
                    ] if value == &format!("{old_url}/new")));
                    drop(scene);
                    handle_edit(&mut app, edits);
                    handle_edit(&mut app, url_edits);
                } else {
                    check_declaration(&mut app, &mut renderer, provider, create);
                }

                if provider == "openclaw" {
                    let form = app.app_providers.form.as_ref().unwrap();
                    let mut scene = Scene::new(fields(form), &mut renderer, size);
                    let point = scene.node(&[1, 0, 3]).bounds().center();
                    let edits = scene.click(&mut renderer, point);
                    assert!(matches!(
                        edits.as_slice(),
                        [Message::Settings(SettingsMessage::AppProviderEdit(
                            EditMessage::OpenClaw(events::openclaw::Field::UserAgent(true))
                        ))]
                    ));
                    drop(scene);
                    handle_edit(&mut app, edits);
                    assert!(
                        app.app_providers
                            .form
                            .as_ref()
                            .unwrap()
                            .openclaw
                            .as_ref()
                            .unwrap()
                            .user_agent
                    );
                }

                let mut form = app.app_providers.form.take().unwrap();
                form.saving = true;
                let mut scene = Scene::new(fields(&form), &mut renderer, size);
                let point = key_input(&scene, provider).bounds().center();
                assert!(scene.type_in(&mut renderer, point, "-discard").is_empty());
                if provider != "agy" {
                    let point = declaration_editor(&scene, provider, create).center();
                    assert!(scene.click(&mut renderer, point).is_empty());
                    assert!(
                        scene
                            .send(
                                &mut renderer,
                                &[commit("discard")],
                                mouse::Cursor::Available(point)
                            )
                            .is_empty()
                    );
                    let apply = scene.node(&[1, 0]).children().next_back().unwrap().bounds();
                    assert!(scene.click(&mut renderer, apply.center()).is_empty());
                }
                drop(scene);

                form.saving = false;
                if !create {
                    app.app_providers.form = Some(form);
                    check_saved_key_controls(&mut app, &mut renderer, provider, size);
                    form = app.app_providers.form.take().unwrap();
                }
                form.fields.name.clear();
                let mut dialog = Scene::new(overlay(&form), &mut renderer, size);
                assert!(dialog.click(&mut renderer, dialog.modal(&[2, 1]).center()).is_empty());
                let cancel = dialog.click(&mut renderer, dialog.modal(&[2, 0]).center());
                assert!(matches!(
                    cancel.as_slice(),
                    [Message::Settings(SettingsMessage::AppProviderEdit(EditMessage::Cancel))]
                ));
                drop(dialog);
                app.app_providers.form = Some(form);
                handle_edit(&mut app, cancel);
                assert!(app.app_providers.form.is_none());
            }
        }
        assert!(requests.try_recv().is_err());
    });
}

fn check_saved_key_controls(
    app: &mut App,
    renderer: &mut Renderer,
    provider: &str,
    size: Size<u32>,
) {
    let form = app.app_providers.form.as_ref().unwrap();
    let mut scene = Scene::new(fields(form), renderer, size);
    let panel = scene.node(&[1, 0]);
    let child_count = panel.children().len();
    let trailing_rows = if provider == "agy" { 2 } else { 4 };
    let replace = panel.children().rev().nth(trailing_rows + 1).unwrap().bounds().center();
    let clear = panel.children().rev().nth(trailing_rows).unwrap().bounds().center();
    assert!(matches!(
        scene.click(renderer, replace).as_slice(),
        [Message::Settings(SettingsMessage::AppProviderEdit(EditMessage::Replace(false)))]
    ));
    let messages = scene.click(renderer, clear);
    assert!(matches!(
        messages.as_slice(),
        [Message::Settings(SettingsMessage::AppProviderEdit(EditMessage::Clear(true)))]
    ));
    drop(scene);
    handle_edit(app, messages);
    let form = app.app_providers.form.as_ref().unwrap();
    assert!(form.clear_key && !form.replace_key && form.key.is_empty());
    let scene = Scene::new(fields(form), renderer, size);
    assert_eq!(scene.node(&[1, 0]).children().len(), child_count - 1);
}

#[tokio::test]
async fn ui_refresh_14_renders_loading_error_and_empty_catalog() {
    use crate::app::state::{
        AgentAppsContext,
        app_providers::list::{ListState, Scope, Snapshot},
    };
    use vw_gateway_client::vw_api_types::{
        agent_apps::AppId,
        app_providers::catalog::{ProviderCatalog, ProviderSelection},
    };

    struct VisibleText<'a> {
        expected: &'a str,
        found: bool,
    }

    impl Operation for VisibleText<'_> {
        fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
            operate(self);
        }

        fn text(&mut self, _id: Option<&Id>, _bounds: Rectangle, value: &str) {
            self.found |= value == self.expected;
        }
    }

    let mut renderer = renderer().await;
    let scope = Scope {
        context: AgentAppsContext {
            gateway: "http://127.0.0.1:1".into(),
            tenant: Default::default(),
            project: None,
        },
        host: "fixture-host".into(),
        app: AppId::Dsh,
    };
    let mut state = ListState::default();
    let generation = state.begin(scope.clone());
    let size = Size::new(420, 690);
    let mut check = |state: &ListState, expected, name: &str| {
        let mut scene = Scene::new(
            super::super::list::overlay(state, Help::test(), base()),
            &mut renderer,
            size,
        );
        let mut text = VisibleText { expected, found: false };
        scene.ui.operate(&renderer, &mut text);
        assert!(text.found, "{name}");
        for (theme, mode) in [(Theme::Light, "light"), (Theme::Dark, "dark")] {
            capture(
                &mut scene.ui,
                &mut renderer,
                &theme,
                size,
                &format!("catalog-{name}-{mode}"),
                mouse::Cursor::Unavailable,
            );
        }
    };
    check(&state, "正在从执行主机读取…", "loading");

    let error = "读取失败／讀取失敗 · Fixture error";
    assert!(state.finish(&scope, generation, Err(error.into())));
    check(&state, error, "error");

    let generation = state.begin(scope.clone());
    assert!(state.finish(
        &scope,
        generation,
        Ok(Snapshot {
            catalog: ProviderCatalog {
                app_id: scope.app,
                rows: vec![],
                direct: ProviderSelection::Unconfigured,
                in_use: ProviderSelection::Unconfigured,
                native_read: None,
            },
            presentation: vec![],
        })
    ));
    check(&state, "没有已保存的供应商", "empty");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn help_rollout_06_renders_declaration_adapters() {
    use super::super::desktop::ui_refresh_12_tests::assert_contextual_help;
    use crate::app::i18n::I18n;
    use vw_config_types::ui::DesktopLocale;

    let mut renderer = renderer().await;
    let theme = Theme::Light;
    let help = Help::new(I18n::new(DesktopLocale::SimplifiedChinese), &theme);
    for provider in PROVIDERS {
        let form = fixture(provider, false, "虚构配置");
        let expected = if provider == "agy" { 2 } else { 3 };
        let name = provider;
        assert_contextual_help(
            edit::overlay(&form, help, base()),
            &mut renderer,
            &theme,
            Size::new(420, 760),
            &format!("declaration_adapters-{name}"),
            expected,
        );
    }
}
