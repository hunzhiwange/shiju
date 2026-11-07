use super::super::{claude, codex, edit, gemini, grok, opencode};
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
use iced::advanced::renderer::{Headless, Style};
use iced::advanced::widget::operation::{Operation, Scrollable};
use iced::advanced::{Layout, clipboard, layout, widget::Tree};
use iced::keyboard::{self, Key, key::Named};
use iced::widget::Space;
use iced::widget::text_editor;
use iced::{Element, Event, Font, Length, Point, Rectangle, Renderer, Size, Theme, Vector, mouse};
use iced_runtime::user_interface::{Cache, UserInterface};
use vw_gateway_client::vw_api_types::{
    agent_apps::AppId,
    app_providers::{
        basic::ProviderBasic,
        claude::ClaudeFormKind,
        codex_catalog::{CodexFormConfig, CodexModelMapping},
        gemini::GeminiFormKind,
        grok::GrokFormConfig,
        opencode::{OpenCodeFormConfig, OpenCodeFormat},
    },
};

const PROVIDERS: [(AppId, &str); 5] = [
    (AppId::Claude, "claude"),
    (AppId::Codex, "codex"),
    (AppId::Gemini, "gemini"),
    (AppId::GrokBuild, "grok"),
    (AppId::OpenCode, "opencode"),
];
const MODEL: &str = "fixture-model";
const KEY: &str = "fixture-secret";
const TOML: &str = "model = \"fixture-model\"\nbase_url = \"https://example.invalid/v1\"\nenv_key = \"FIXTURE_KEY\"\n# 简体／繁體 · Example\n";
const DECLARATION: &str = "{\n  // 简体／繁體 · Example\n  \"models\": {\"fixture-model\": {}},\n  \"options\": {\"baseURL\": \"https://example.invalid/v1\"}\n}";
const MODAL: &[usize] = &[1, 1, 0, 0, 0];

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

struct Scene<'a> {
    ui: UserInterface<'a, Message, Theme, Renderer>,
    layout: layout::Node,
    size: Size<u32>,
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
        Self { ui, layout, size }
    }

    fn bounds(&self, path: &[usize]) -> Rectangle {
        path.iter()
            .fold(Layout::new(&self.layout), |node, &index| {
                node.children().nth(index).unwrap_or_else(|| panic!("missing {path:?}"))
            })
            .bounds()
    }

    fn modal(&self, path: &[usize]) -> Rectangle {
        self.bounds(&[MODAL, path].concat())
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

    fn capture(&mut self, renderer: &mut Renderer, theme: &Theme, name: &str) {
        self.capture_with_cursor(renderer, theme, name, mouse::Cursor::Unavailable);
    }

    fn capture_with_cursor(
        &mut self,
        renderer: &mut Renderer,
        theme: &Theme,
        name: &str,
        cursor: mouse::Cursor,
    ) {
        assert!(
            self.send(
                renderer,
                &[Event::Window(iced::window::Event::RedrawRequested(iced::time::Instant::now()))],
                cursor,
            )
            .is_empty()
        );
        self.ui.draw(renderer, theme, &Style { text_color: theme.palette().text }, cursor);
        let pixels = renderer.screenshot(self.size, 1.0, theme.palette().background);
        if let Some(directory) = std::env::var_os("SHIJU_SETTINGS_PREVIEW_DIR") {
            image::save_buffer(
                std::path::PathBuf::from(directory).join(format!("{name}.png")),
                &pixels,
                self.size.width,
                self.size.height,
                image::ColorType::Rgba8,
            )
            .expect("save provider screenshot");
        }
    }
}

fn fixture(provider: AppId, mode: Mode) -> EditForm {
    let create = matches!(mode, Mode::Create);
    let url = format!("https://example.invalid/{}", "fixture-path/".repeat(14));
    let mut form = EditForm::new(
        mode,
        "fixture-id".into(),
        ProviderBasic {
            name: "测试供应商／測試供應商 · Example provider".into(),
            category: Some("分类／分類".into()),
            notes: None,
            website_url: Some(url.clone()),
        },
        None,
    );
    let inactive = vec![vec!["config".into(), "配置／組態".into(), "fixture_path".repeat(7)]];
    match provider {
        AppId::Claude => {
            form.claude = Some(if create {
                state::claude::Draft::new()
            } else {
                state::claude::Draft::from_view(
                    ClaudeFormKind::ThirdParty { base_url: url, api_key_header: true },
                    Some(MODEL.into()),
                )
            });
        }
        AppId::Gemini => {
            form.gemini = Some(if create {
                state::gemini::Draft::new()
            } else {
                state::gemini::Draft::from_view(
                    GeminiFormKind::ThirdParty { base_url: url },
                    Some(MODEL.into()),
                )
            });
        }
        AppId::Codex => {
            form.codex = Some(if create {
                state::codex::Draft::new()
            } else {
                state::codex::Draft::from_view(CodexFormConfig {
                    toml: TOML.into(),
                    has_saved_key: true,
                    inactive,
                    models: Some(vec![CodexModelMapping {
                        slug: MODEL.into(),
                        display_name: "显示名／顯示名稱 · Example model".into(),
                    }]),
                })
            });
        }
        AppId::GrokBuild => {
            form.grok = Some(if create {
                state::grok::Draft::new()
            } else {
                state::grok::Draft::from_view(GrokFormConfig {
                    toml: TOML.into(),
                    has_saved_key: true,
                    inactive,
                })
            });
        }
        AppId::OpenCode => {
            form.opencode = Some(if create {
                state::opencode::Draft::new()
            } else {
                state::opencode::Draft::from_view(
                    OpenCodeFormConfig {
                        declaration: DECLARATION.into(),
                        format: OpenCodeFormat::V1,
                        has_saved_key: true,
                        revision: "fixture-revision".into(),
                    },
                    true,
                )
            });
        }
        _ => unreachable!(),
    }
    form
}

fn fields(form: &EditForm, provider: AppId) -> Element<'_, Message> {
    match provider {
        AppId::Claude => claude::fields(form, form.claude.as_ref().unwrap()),
        AppId::Codex => codex::fields(form, form.codex.as_ref().unwrap()),
        AppId::Gemini => gemini::fields(form, form.gemini.as_ref().unwrap()),
        AppId::GrokBuild => grok::fields(form, form.grok.as_ref().unwrap()),
        AppId::OpenCode => opencode::fields(form, form.opencode.as_ref().unwrap()),
        _ => unreachable!(),
    }
}

fn base() -> Element<'static, Message> {
    Space::new().width(Length::Fill).height(Length::Fill).into()
}

fn assert_centered(a: Rectangle, b: Rectangle) {
    assert!((a.center().y - b.center().y).abs() < 0.5, "{a:?} vs {b:?}");
}

#[tokio::test]
async fn ui_refresh_13_layout_and_interaction() {
    let mut renderer = renderer().await;
    for (provider, name) in PROVIDERS {
        for (mode, phase) in [(Mode::Create, "create"), (Mode::Update, "update")] {
            let mut form = fixture(provider, mode);
            form.key = KEY.into();
            form.replace_key = true;
            for (theme, width, theme_name) in [
                (Theme::Light, 420, "light"),
                (Theme::Dark, 420, "dark"),
                (Theme::Light, 640, "light"),
                (Theme::Dark, 640, "dark"),
                (Theme::Light, 960, "light"),
                (Theme::Dark, 960, "dark"),
            ] {
                let mut scene =
                    Scene::new(fields(&form, provider), &mut renderer, Size::new(width, 760));
                if let (AppId::Codex, "update") = (provider, phase) {
                    let name_input = scene.bounds(&[1, 0, 9, 0]);
                    let model_input = scene.bounds(&[1, 0, 9, 1]);
                    let remove = scene.bounds(&[1, 0, 9, 2]);
                    assert_centered(name_input, scene.bounds(&[1, 0, 9, 0, 0]));
                    assert_centered(name_input, model_input);
                    assert_centered(model_input, remove);
                    assert_centered(remove, scene.bounds(&[1, 0, 9, 2, 0]));
                    let panel_content = scene.bounds(&[1, 0]);
                    assert!(remove.x + remove.width <= panel_content.x + panel_content.width);
                    assert!(matches!(
                        scene.click(&mut renderer, remove.center()).as_slice(),
                        [Message::Settings(SettingsMessage::AppProviderEdit(EditMessage::Codex(
                            events::codex::Field::RemoveModel(0)
                        )))]
                    ));
                }
                if let (AppId::OpenCode, "create") = (provider, phase) {
                    let format = scene.bounds(&[1, 0, 0, 0]);
                    assert!(matches!(
                        scene.click(&mut renderer, format.center()).as_slice(),
                        [Message::Settings(SettingsMessage::AppProviderEdit(
                            EditMessage::OpenCode(events::opencode::Field::V2(true))
                        ))]
                    ));
                }
                scene.capture(
                    &mut renderer,
                    &theme,
                    &format!("{name}-{phase}-{theme_name}-{width}"),
                );
            }
            let mut scene =
                Scene::new(edit::overlay(&form, base()), &mut renderer, Size::new(420, 760));
            assert_centered(scene.modal(&[0, 0]), scene.modal(&[0, 1]));
            assert!(scene.modal(&[1]).y + scene.modal(&[1]).height < scene.modal(&[2]).y);
            for index in 0..2 {
                assert_centered(scene.modal(&[2, index]), scene.modal(&[2, index, 0]));
            }
            scene.capture(&mut renderer, &Theme::Light, &format!("{name}-{phase}-modal"));

            let cursor = mouse::Cursor::Available(scene.modal(&[1]).center());
            assert!(
                scene
                    .send(
                        &mut renderer,
                        &[Event::Mouse(mouse::Event::WheelScrolled {
                            delta: mouse::ScrollDelta::Pixels { x: 0.0, y: -1800.0 },
                        })],
                        cursor
                    )
                    .is_empty()
            );
            let mut scroll = BodyScroll::default();
            scene.ui.operate(&renderer, &mut scroll);
            assert!(scroll.0.unwrap().y > 0.0);
            scene.capture(&mut renderer, &Theme::Dark, &format!("{name}-{phase}-modal-scrolled"));

            let save = scene.modal(&[2, 1]).center();
            let cursor = mouse::Cursor::Available(save);
            assert!(
                scene
                    .send(
                        &mut renderer,
                        &[Event::Mouse(mouse::Event::CursorMoved { position: save })],
                        cursor
                    )
                    .is_empty()
            );
            scene.capture_with_cursor(
                &mut renderer,
                &Theme::Dark,
                &format!("{name}-{phase}-hover"),
                cursor,
            );
            assert!(
                scene
                    .send(
                        &mut renderer,
                        &[Event::Mouse(mouse::Event::CursorLeft)],
                        mouse::Cursor::Unavailable
                    )
                    .is_empty()
            );
            assert!(matches!(
                scene.click(&mut renderer, save).as_slice(),
                [Message::Settings(SettingsMessage::AppProviderEdit(EditMessage::Save))]
            ));
            assert!(matches!(
                scene.click(&mut renderer, scene.modal(&[2, 0]).center()).as_slice(),
                [Message::Settings(SettingsMessage::AppProviderEdit(EditMessage::Cancel))]
            ));
        }
    }
}

#[derive(Default)]
struct BodyScroll(Option<Vector>);

impl Operation for BodyScroll {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
        operate(self);
    }

    fn scrollable(
        &mut self,
        _id: Option<&iced::advanced::widget::Id>,
        _bounds: Rectangle,
        _content: Rectangle,
        translation: Vector,
        _state: &mut dyn Scrollable,
    ) {
        self.0.get_or_insert(translation);
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

fn apply(app: &mut App, messages: Vec<Message>) {
    for message in messages {
        let Message::Settings(SettingsMessage::AppProviderEdit(event)) = message else {
            panic!("unexpected message outside provider form");
        };
        assert!(!matches!(event, EditMessage::Save));
        drop(events::edit::update(app, event));
    }
}

fn type_in(
    form: &EditForm,
    provider: AppId,
    renderer: &mut Renderer,
    path: &[usize],
    suffix: &str,
) -> Vec<Message> {
    let mut scene = Scene::new(fields(form, provider), renderer, Size::new(420, 760));
    let input = scene.bounds(path);
    assert_centered(input, scene.bounds(&[path, &[0]].concat()));
    assert!(scene.click(renderer, input.center()).is_empty());
    scene.send(
        renderer,
        &[key(Named::End), commit(suffix), key(Named::Enter), key(Named::Escape)],
        mouse::Cursor::Available(input.center()),
    )
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn ui_refresh_13_preserves_values_and_disabled_states() {
    let mut renderer = renderer().await;
    crate::app::config::with_editor_settings_gateway(|mut app, _, _| {
        for (provider, name) in PROVIDERS {
            for create in [true, false] {
                let mode = if create { Mode::Create } else { Mode::Update };
                let mut form = fixture(provider, mode);
                form.key = KEY.into();
                form.replace_key = !create;
                app.app_providers.form = Some(form);

                if create && matches!(provider, AppId::Claude | AppId::Gemini) {
                    let form = app.app_providers.form.as_ref().unwrap();
                    let mut scene =
                        Scene::new(fields(form, provider), &mut renderer, Size::new(420, 760));
                    let point = scene.bounds(&[1, 0, 0, 1]).center();
                    let messages = scene.click(&mut renderer, point);
                    assert!(matches!(
                        messages.as_slice(),
                        [Message::Settings(SettingsMessage::AppProviderEdit(
                            EditMessage::Claude(events::claude::Field::ThirdParty(true))
                                | EditMessage::Gemini(events::gemini::Field::ThirdParty(true))
                        ))]
                    ));
                    drop(scene);
                    apply(&mut app, messages);
                }

                let key_path: &[usize] = match (provider, create) {
                    (AppId::Claude, true) => &[1, 0, 3, 1],
                    (AppId::Claude, false) => &[1, 0, 4, 1],
                    (AppId::Gemini, true) => &[1, 0, 2, 1],
                    (AppId::Gemini, false) => &[1, 0, 3, 1],
                    (AppId::Codex | AppId::GrokBuild, true) => &[1, 0, 2],
                    (AppId::Codex | AppId::GrokBuild, false) => &[1, 0, 5],
                    (AppId::OpenCode, true) => &[1, 0, 3],
                    (AppId::OpenCode, false) => &[1, 0, 6],
                    _ => unreachable!(),
                };
                let form = app.app_providers.form.as_ref().unwrap();
                let messages = type_in(form, provider, &mut renderer, key_path, "-updated");
                let expected_key = format!("{KEY}-updated");
                assert!(matches!(messages.as_slice(), [
                    Message::Settings(SettingsMessage::AppProviderEdit(EditMessage::Key(value)))
                ] if value == &expected_key));
                apply(&mut app, messages);

                let form = app.app_providers.form.as_ref().unwrap();
                assert_eq!(form.key, expected_key);
                assert_eq!(form.id, "fixture-id");

                match provider {
                    AppId::Claude | AppId::Gemini => {
                        let model_path = [1, 0, key_path[2] + 1, 1, 0];
                        let expected_model =
                            if create { "-updated".into() } else { format!("{MODEL}-updated") };
                        let messages =
                            type_in(form, provider, &mut renderer, &model_path, "-updated");
                        assert!(matches!(messages.as_slice(), [
                            Message::Settings(SettingsMessage::AppProviderEdit(
                                EditMessage::Claude(events::claude::Field::Model(value))
                                | EditMessage::Gemini(events::gemini::Field::Model(value))
                            ))
                        ] if value == &expected_model));
                        apply(&mut app, messages);

                        let form = app.app_providers.form.as_ref().unwrap();
                        let expected_url = if create {
                            "/updated".into()
                        } else {
                            format!("{}/updated", form.fields.website_url.as_ref().unwrap())
                        };
                        let messages =
                            type_in(form, provider, &mut renderer, &[1, 0, 1, 1], "/updated");
                        assert!(matches!(messages.as_slice(), [
                            Message::Settings(SettingsMessage::AppProviderEdit(
                                EditMessage::Claude(events::claude::Field::BaseUrl(value))
                                | EditMessage::Gemini(events::gemini::Field::BaseUrl(value))
                            ))
                        ] if value == &expected_url));
                        apply(&mut app, messages);

                        let form = app.app_providers.form.as_ref().unwrap();
                        let (url, model) = if let Some(draft) = &form.claude {
                            assert_eq!(draft.api_key_header, !create);
                            (&draft.base_url, &draft.model)
                        } else {
                            let draft = form.gemini.as_ref().unwrap();
                            (&draft.base_url, &draft.model)
                        };
                        assert_eq!(url, &expected_url);
                        assert_eq!(model, &expected_model);
                    }
                    _ => {
                        let editor_index = usize::from(provider == AppId::OpenCode);
                        let contents = |form: &EditForm| match provider {
                            AppId::Codex => form.codex.as_ref().unwrap().toml.text(),
                            AppId::GrokBuild => form.grok.as_ref().unwrap().toml.text(),
                            AppId::OpenCode => form.opencode.as_ref().unwrap().declaration.text(),
                            _ => unreachable!(),
                        };
                        let original = contents(form);
                        let mut scene =
                            Scene::new(fields(form, provider), &mut renderer, Size::new(420, 760));
                        let editor = scene.bounds(&[1, 0, editor_index]);
                        let point = Point::new(editor.x + 16.0, editor.y + 14.0);
                        let click = scene.click(&mut renderer, point);
                        let inserted = if provider == AppId::OpenCode {
                            "// fixture\n"
                        } else {
                            "# fixture\n"
                        };
                        let messages = scene.send(
                            &mut renderer,
                            &[commit(inserted)],
                            mouse::Cursor::Available(point),
                        );
                        assert!(matches!(messages.as_slice(), [
                            Message::Settings(SettingsMessage::AppProviderEdit(
                                EditMessage::Codex(events::codex::Field::Toml(text_editor::Action::Edit(text_editor::Edit::Paste(value))))
                                | EditMessage::Grok(events::grok::Field::Toml(text_editor::Action::Edit(text_editor::Edit::Paste(value))))
                                | EditMessage::OpenCode(events::opencode::Field::Declaration(text_editor::Action::Edit(text_editor::Edit::Paste(value))))
                            ))
                        ] if value.as_str() == inserted));
                        drop(scene);
                        apply(&mut app, click);
                        apply(&mut app, messages);
                        let actual = contents(app.app_providers.form.as_ref().unwrap());
                        assert!(actual.contains(inserted));
                        assert_eq!(actual.replacen(inserted, "", 1), original);
                    }
                }

                let mut form = app.app_providers.form.take().unwrap();
                form.saving = true;
                assert!(type_in(&form, provider, &mut renderer, key_path, "-discard").is_empty());
                let mut scene =
                    Scene::new(edit::overlay(&form, base()), &mut renderer, Size::new(420, 760));
                assert!(scene.click(&mut renderer, scene.modal(&[2, 0]).center()).is_empty());
                assert!(scene.click(&mut renderer, scene.modal(&[2, 1]).center()).is_empty());
                scene.capture(&mut renderer, &Theme::Dark, &format!("{name}-busy"));
                drop(scene);

                form.saving = false;
                form.fields.name.clear();
                let mut scene =
                    Scene::new(edit::overlay(&form, base()), &mut renderer, Size::new(420, 760));
                assert!(scene.click(&mut renderer, scene.modal(&[2, 1]).center()).is_empty());
                let cancel = scene.click(&mut renderer, scene.modal(&[2, 0]).center());
                assert!(matches!(
                    cancel.as_slice(),
                    [Message::Settings(SettingsMessage::AppProviderEdit(EditMessage::Cancel))]
                ));
                drop(scene);
                app.app_providers.form = Some(form);
                apply(&mut app, cancel);
                assert!(app.app_providers.form.is_none());
            }
        }
    });
}
