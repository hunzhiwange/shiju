use super::super::list::Help;
use super::super::{edit, list};
use super::Form;
use crate::app::message::{
    SettingsMessage,
    app_providers::{
        desktop::{self, Event as DesktopEvent},
        edit::EditMessage,
        list::ListMessage,
    },
};
use crate::app::{
    Message,
    i18n::I18n,
    state::{
        AgentAppsContext,
        app_providers::{
            edit::{EditForm, Mode},
            list::{Editor, ListState, Scope, Snapshot},
        },
    },
};
use iced::advanced::renderer::{Headless, Style};
use iced::advanced::widget::operation::{Operation, Scrollable};
use iced::advanced::{Layout, clipboard, layout, widget::Tree};
use iced::keyboard::{self, Key, key::Named};
use iced::widget::Space;
use iced::{Element, Event, Font, Point, Rectangle, Renderer, Size, Theme, Vector, mouse};
use iced_runtime::user_interface::{Cache, UserInterface};
use vw_config_types::ui::DesktopLocale;
use vw_gateway_client::vw_api_types::{
    agent_apps::AppId,
    app_providers::{
        AppProviderSummary,
        basic::ProviderBasic,
        catalog::{ProviderCatalog, ProviderCatalogRow, ProviderMembership, ProviderSelection},
        desktop_mapping::DesktopModelRoute,
    },
};

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

pub(in crate::app::components::app_providers) struct Scene<'a> {
    ui: UserInterface<'a, Message, Theme, Renderer>,
    layout: layout::Node,
    size: Size<u32>,
}

impl<'a> Scene<'a> {
    pub(in crate::app::components::app_providers) fn new(
        mut element: Element<'a, Message>,
        renderer: &mut Renderer,
        size: Size<u32>,
    ) -> Self {
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
            .fold(Layout::new(&self.layout), |layout, &index| {
                layout
                    .children()
                    .nth(index)
                    .unwrap_or_else(|| panic!("missing layout path {path:?}"))
            })
            .bounds()
    }

    fn send(
        &mut self,
        renderer: &mut Renderer,
        events: &[Event],
        cursor: mouse::Cursor,
    ) -> Vec<Message> {
        let mut messages = Vec::new();
        self.ui.update(events, cursor, renderer, &mut clipboard::Null, &mut messages);
        messages.retain(|message| !matches!(message, Message::None));
        messages
    }

    fn modal(&self, path: &[usize]) -> Rectangle {
        self.bounds(&[&[1, 1, 0, 0, 0][..], path].concat())
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

    pub(in crate::app::components::app_providers) fn press(
        &mut self,
        renderer: &mut Renderer,
        path: &[usize],
    ) -> Vec<Message> {
        self.click(renderer, self.modal(path).center())
    }

    fn capture(
        &mut self,
        renderer: &mut Renderer,
        theme: &Theme,
        cursor: mouse::Cursor,
        name: &str,
    ) -> Vec<u8> {
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
            .expect("save screenshot");
        }
        pixels
    }
}

fn base() -> Element<'static, Message> {
    Space::new().width(iced::Length::Fill).height(iced::Length::Fill).into()
}

fn provider_name() -> String {
    "测试供应商／測試供應商 · Example provider".repeat(3)
}

fn list_fixture(app: AppId) -> ListState {
    ListState {
        scope: Some(Scope {
            context: AgentAppsContext {
                gateway: "http://127.0.0.1:1".into(),
                tenant: Default::default(),
                project: None,
            },
            host: "fixture-host".into(),
            app,
        }),
        snapshot: Some(Snapshot {
            catalog: ProviderCatalog {
                app_id: app,
                rows: vec![ProviderCatalogRow {
                    provider: AppProviderSummary {
                        id: "fixture-id".into(),
                        app_id: app,
                        name: provider_name(),
                        configured: true,
                    },
                    native_member: ProviderMembership::Present,
                }],
                direct: ProviderSelection::Selected { id: "fixture-id".into() },
                in_use: ProviderSelection::Unconfigured,
                native_read: None,
            },
            presentation: vec![],
        }),
        ..ListState::default()
    }
}

fn edit_fixture(mode: Mode) -> EditForm {
    EditForm::new(
        mode,
        "fixture-id".into(),
        ProviderBasic {
            name: provider_name(),
            category: Some("分类／分類".into()),
            notes: Some("Example notes".into()),
            website_url: Some(format!("https://example.invalid/{}", "long-path/".repeat(24))),
        },
        None,
    )
}

fn desktop_fixture() -> Form {
    let mut form = Form::new("fixture-id".into());
    form.create = false;
    form.name = provider_name();
    form.mapping_editable = true;
    form.key = "fixture-secret".into();
    form.base_url = format!("https://example.invalid/{}", "模型/".repeat(25));
    form.mapping.routes.insert(
        "fixture-route".into(),
        DesktopModelRoute {
            model: "fixture-model".into(),
            label_override: Some("显示名／顯示名稱 · Menu label".into()),
            supports_1m: true,
        },
    );
    form
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

#[derive(Default)]
struct FormScroll(Option<(Rectangle, Vector)>);

impl Operation for FormScroll {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
        operate(self);
    }

    fn scrollable(
        &mut self,
        id: Option<&iced::advanced::widget::Id>,
        bounds: Rectangle,
        _content: Rectangle,
        translation: Vector,
        _state: &mut dyn Scrollable,
    ) {
        if id == Some(&iced::widget::Id::new("desktop-provider-form")) {
            self.0 = Some((bounds, translation));
        }
    }
}

fn assert_centered(a: Rectangle, b: Rectangle) {
    assert!((a.center().y - b.center().y).abs() < 0.5, "{a:?} vs {b:?}");
}

fn assert_footer(scene: &Scene<'_>, buttons: usize) {
    let content = scene.modal(&[1]);
    let footer = scene.modal(&[2]);
    assert!(content.y + content.height < footer.y);
    for index in 0..buttons {
        let button = scene.modal(&[2, index]);
        let label = scene.modal(&[2, index, 0]);
        assert_centered(button, label);
        assert!(label.height < 24.0, "action label wrapped instead of button: {label:?}");
        assert!(button.x >= footer.x && button.x + button.width <= footer.x + footer.width + 0.01);
        assert!(
            button.y >= footer.y && button.y + button.height <= footer.y + footer.height + 0.01
        );
    }
}

#[tokio::test]
async fn ui_refresh_12_layout_and_interaction() {
    let mut renderer = renderer().await;
    let list = list_fixture(AppId::ClaudeDesktop);
    let edit = edit_fixture(Mode::Update);
    let desktop = desktop_fixture();
    for (theme, width, name) in [
        (Theme::Light, 420, "light-420"),
        (Theme::Dark, 640, "dark-640"),
        (Theme::Light, 960, "light-960"),
        (Theme::Dark, 420, "dark-420"),
        (Theme::Light, 640, "light-640"),
        (Theme::Dark, 960, "dark-960"),
    ] {
        for (view, page, buttons) in [
            (list::overlay(&list, Help::test(), base()), "list", 5),
            (edit::overlay(&edit, Help::test(), base()), "edit", 2),
            (super::overlay(&desktop, Help::test(), base()), "desktop", 4),
        ] {
            let mut scene = Scene::new(view, &mut renderer, Size::new(width, 760));
            assert_centered(scene.modal(&[0, 0]), scene.modal(&[0, 1]));
            assert_footer(&scene, buttons);
            scene.capture(
                &mut renderer,
                &theme,
                mouse::Cursor::Unavailable,
                &format!("{page}-{name}"),
            );
        }
    }

    let mut scene =
        Scene::new(list::overlay(&list, Help::test(), base()), &mut renderer, Size::new(420, 760));
    let actions = [1, 0, 0, 1, 0, 2];
    for (index, copy, delete) in [(0, false, false), (1, true, false), (2, false, true)] {
        let path = [&actions[..], &[index]].concat();
        let cursor = mouse::Cursor::Available(scene.modal(&path).center());
        assert!(
            scene
                .send(
                    &mut renderer,
                    &[Event::Mouse(mouse::Event::CursorMoved {
                        position: scene.modal(&path).center(),
                    })],
                    cursor
                )
                .is_empty()
        );
        scene.capture(&mut renderer, &Theme::Light, cursor, &format!("list-hover-{index}"));
        assert!(matches!(
            scene.press(&mut renderer, &path).as_slice(),
            [Message::Settings(SettingsMessage::AppProviderEdit(EditMessage::Open {
                id, copy: actual_copy, delete: actual_delete,
            }))] if id == "fixture-id" && *actual_copy == copy && *actual_delete == delete
        ));
    }
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
        scene.press(&mut renderer, &[2, 0]).as_slice(),
        [Message::Settings(SettingsMessage::AppProviders(ListMessage::Desktop(
            DesktopEvent::Open(None)
        )))]
    ));
    assert!(matches!(
        scene.press(&mut renderer, &[2, 1]).as_slice(),
        [Message::Settings(SettingsMessage::AppProviders(ListMessage::Refresh))]
    ));

    let mut scene = Scene::new(
        super::overlay(&desktop, Help::test(), base()),
        &mut renderer,
        Size::new(420, 760),
    );
    let mut scroll = FormScroll::default();
    scene.ui.operate(&renderer, &mut scroll);
    let (body, offset) = scroll.0.expect("desktop form has one scroll viewport");
    assert_eq!(offset.y, 0.0);
    let cursor = mouse::Cursor::Available(body.center());
    assert!(
        scene
            .send(
                &mut renderer,
                &[Event::Mouse(mouse::Event::WheelScrolled {
                    delta: mouse::ScrollDelta::Pixels { x: 0.0, y: -1500.0 },
                })],
                cursor
            )
            .is_empty()
    );
    scene.ui.operate(&renderer, &mut scroll);
    let (scrolled_body, offset) = scroll.0.unwrap();
    assert_eq!(body, scrolled_body);
    assert!(offset.y > 0.0);
    scene.capture(&mut renderer, &Theme::Dark, cursor, "desktop-dark-scrolled");
    let model_path = [1, 0, 0, 6, 0, 2];
    assert_centered(scene.modal(&model_path), scene.modal(&[1, 0, 0, 6, 0, 2, 0]));
    let mut model = scene.modal(&model_path);
    model.y -= offset.y;
    assert!(body.contains(model.center()), "{body:?} vs {model:?}");
    assert!(scene.click(&mut renderer, model.center()).is_empty());
    let value = scene.send(
        &mut renderer,
        &[
            key(Named::End),
            Event::InputMethod(iced::advanced::input_method::Event::Commit("-updated".into())),
            key(Named::Enter),
            key(Named::Escape),
        ],
        mouse::Cursor::Available(model.center()),
    );
    assert!(
        matches!(value.as_slice(),
            [Message::Settings(SettingsMessage::AppProviders(ListMessage::Desktop(DesktopEvent::Model(id, model))))]
            if id == "fixture-route" && model == "fixture-model-updated"
        ),
        "{value:?}"
    );
    assert!(matches!(
        scene.press(&mut renderer, &[2, 2]).as_slice(),
        [Message::Settings(SettingsMessage::AppProviders(ListMessage::Desktop(
            DesktopEvent::SaveMapping
        )))]
    ));
    assert!(matches!(
        scene.press(&mut renderer, &[2, 3]).as_slice(),
        [Message::Settings(SettingsMessage::AppProviders(ListMessage::Desktop(
            DesktopEvent::Cancel
        )))]
    ));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn ui_refresh_12_preserves_values_and_disabled_states() {
    let mut renderer = renderer().await;
    let size = Size::new(420, 760);
    let mut app = None;
    crate::app::config::with_editor_settings_gateway(|fixture, _, _| app = Some(fixture));
    let mut app = app.expect("isolated settings fixture");
    let original = desktop_fixture();
    app.app_providers.desktop = Some(original.clone());
    for official in [true, false] {
        let form = app.app_providers.desktop.as_ref().unwrap();
        let mut scene = Scene::new(super::overlay(form, Help::test(), base()), &mut renderer, size);
        let events = scene.press(&mut renderer, &[1, 0, 0, 3, 0, 2]);
        assert!(matches!(&events[..],
            [Message::Settings(SettingsMessage::AppProviders(ListMessage::Desktop(DesktopEvent::Official(value))))]
            if *value == official));
        drop(scene);
        let Message::Settings(SettingsMessage::AppProviders(ListMessage::Desktop(event))) =
            events.into_iter().next().unwrap()
        else {
            unreachable!()
        };
        drop(desktop::update(&mut app, event));
        let form = app.app_providers.desktop.as_ref().unwrap();
        assert_eq!(form.official, official);
        assert_eq!(form.mapping, original.mapping);
        assert_eq!(form.key, original.key);
        assert_eq!(form.base_url, original.base_url);
    }

    for (busy, name) in [(true, "busy"), (false, "unsupported")] {
        let mut form = original.clone();
        form.busy = busy;
        form.direct_editable = busy;
        form.mapping_editable = busy;
        form.notice = Some("映射兼容校验失败／映射相容驗證失敗".into());
        let mut scene =
            Scene::new(super::overlay(&form, Help::test(), base()), &mut renderer, size);
        for index in [1, 2] {
            assert!(scene.press(&mut renderer, &[2, index]).is_empty());
        }
        if busy {
            assert!(scene.press(&mut renderer, &[2, 0]).is_empty());
        }
        scene.capture(
            &mut renderer,
            &Theme::Dark,
            mouse::Cursor::Unavailable,
            &format!("desktop-{name}"),
        );
    }

    let mut list = list_fixture(AppId::Claude);
    for name in ["loading", "failure", "empty"] {
        list.loading = name == "loading";
        list.error = (name == "failure").then(|| "读取失败／讀取失敗 · Request failed".into());
        list.snapshot.as_mut().unwrap().catalog.rows.clear();
        let mut scene = Scene::new(list::overlay(&list, Help::test(), base()), &mut renderer, size);
        let events = scene.press(&mut renderer, &[2, 0]);
        if list.loading {
            assert!(events.is_empty());
        } else {
            assert!(matches!(
                events.as_slice(),
                [Message::Settings(SettingsMessage::AppProviderEdit(EditMessage::Create))]
            ));
        }
        scene.capture(
            &mut renderer,
            &Theme::Light,
            mouse::Cursor::Unavailable,
            &format!("list-{name}"),
        );
    }

    list.editor = Some(Editor {
        id: "fixture-id".into(),
        sort: "12".into(),
        category: "测试／測試".into(),
        notes: "Fixture notes".into(),
        saving: false,
    });
    let mut scene = Scene::new(list::overlay(&list, Help::test(), base()), &mut renderer, size);
    assert!(matches!(
        scene.press(&mut renderer, &[2, 0]).as_slice(),
        [Message::Settings(SettingsMessage::AppProviders(ListMessage::Cancel))]
    ));
    scene.capture(&mut renderer, &Theme::Dark, mouse::Cursor::Unavailable, "list-presentation");

    let form = edit_fixture(Mode::Delete);
    let mut scene = Scene::new(edit::overlay(&form, Help::test(), base()), &mut renderer, size);
    assert!(matches!(
        scene.press(&mut renderer, &[2, 0]).as_slice(),
        [Message::Settings(SettingsMessage::AppProviderEdit(EditMessage::Cancel))]
    ));
    assert!(matches!(
        scene.press(&mut renderer, &[2, 1]).as_slice(),
        [Message::Settings(SettingsMessage::AppProviderEdit(EditMessage::Save))]
    ));
    scene.capture(&mut renderer, &Theme::Dark, mouse::Cursor::Unavailable, "edit-delete");
    let mut form = edit_fixture(Mode::Copy { source_id: "fixture-id".into() });
    form.saving = true;
    let mut scene = Scene::new(edit::overlay(&form, Help::test(), base()), &mut renderer, size);
    assert!(scene.press(&mut renderer, &[2, 1]).is_empty());
    assert!(scene.press(&mut renderer, &[2, 0]).is_empty());
    scene.capture(&mut renderer, &Theme::Light, mouse::Cursor::Unavailable, "edit-saving");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn help_rollout_06_renders_contextual_help() {
    let mut renderer = renderer().await;
    let mut claude = edit_fixture(Mode::Update);
    claude.claude = Some(crate::app::state::app_providers::claude::Draft::new());
    let mut agy = edit_fixture(Mode::Update);
    agy.agy = Some(crate::app::state::app_providers::agy::Draft::new());
    let desktop = desktop_fixture();
    for (locale, theme, width) in [
        (DesktopLocale::SimplifiedChinese, Theme::Light, 420),
        (DesktopLocale::English, Theme::Dark, 620),
        (DesktopLocale::TraditionalChinese, Theme::Light, 420),
    ] {
        let help = Help::new(I18n::new(locale), &theme);
        for (name, element, expected) in [
            ("claude", edit::overlay(&claude, help, base()), 2),
            ("desktop", super::overlay(&desktop, help, base()), 1),
            ("agy", edit::overlay(&agy, help, base()), 2),
        ] {
            assert_contextual_help(
                element,
                &mut renderer,
                &theme,
                Size::new(width, 760),
                &format!("{name}-{locale:?}"),
                expected,
            );
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn help_rollout_06_renders_utility_overlays() {
    use crate::app::state::app_providers::{env, mode, paths, preview, snippets, universal};

    let mut renderer = renderer().await;
    let theme = Theme::Dark;
    let help = Help::new(I18n::new(DesktopLocale::English), &theme);
    for name in ["paths", "env", "snippets", "preview", "mode", "universal"] {
        let mut state = list_fixture(AppId::Claude);
        let expected = match name {
            "paths" => {
                state.paths = Some(paths::Form {
                    draft: "/fixture/private-path".into(),
                    error: Some("无法确认执行主机的配置目录。请重新读取。"),
                    ..Default::default()
                });
                1
            }
            "env" => {
                state.env =
                    Some(env::Form { backup_id: "fixture-backup-id".into(), ..Default::default() });
                2
            }
            "snippets" => {
                state.snippets = Some(snippets::Form { loaded: true, ..Default::default() });
                2
            }
            "preview" => {
                state.preview = Some(preview::Form::new("fixture-id".into(), provider_name()));
                1
            }
            "mode" => {
                state.mode = Some(mode::Form::default());
                1
            }
            "universal" => {
                state.universal = Some(universal::Form::default());
                1
            }
            _ => unreachable!(),
        };
        assert_contextual_help(
            list::overlay(&state, help, base()),
            &mut renderer,
            &theme,
            Size::new(420, 760),
            name,
            expected,
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn help_rollout_06_keeps_values_outside_help_actions() {
    use crate::app::message::{ChatMessage, ViewMessage};
    let mut renderer = renderer().await;
    let theme = Theme::Dark;
    let i18n = I18n::new(DesktopLocale::English);
    let mut form = edit_fixture(Mode::Update);
    form.key = "fixture-private-key".into();
    form.replace_key = true;
    let view = edit::overlay(&form, Help::new(i18n, &theme), base());
    let mut scene = Scene::new(view, &mut renderer, Size::new(850, 760));

    let mut icons = Vec::new();
    help_icons(Layout::new(&scene.layout), &mut icons);
    let [icon] = icons.as_slice() else { panic!("website help target: {icons:?}") };
    assert!(scene.click(&mut renderer, icon.center()).is_empty());
    let card = Point::new(icon.x + icon.width + 4.0 + 12.5, icon.y + 16.0);
    let row_height =
        f32::from(iced::advanced::text::LineHeight::default().to_absolute(12.0.into())) + 10.0;
    let description = i18n.__("展示网址，与 API 地址分开保存");
    let menu_item = |row: f32| Point::new(card.x + 24.0, card.y + 3.0 + row_height * (row + 0.5));
    let send_mouse = |scene: &mut Scene<'_>, renderer: &mut Renderer, point, event| {
        scene.send(renderer, &[Event::Mouse(event)], mouse::Cursor::Available(point))
    };
    let right = mouse::Event::ButtonPressed(mouse::Button::Right);

    for (row, prefix) in [
        (0, ""),
        (1, ""),
        (2, "https://www.baidu.com/s?wd="),
        (3, "https://www.google.com/search?q="),
        (4, "https://www.bing.com/search?q="),
    ] {
        assert!(
            send_mouse(&mut scene, &mut renderer, card, right).is_empty(),
            "opening the menu does not publish an action",
        );
        let messages = scene.click(&mut renderer, menu_item(row as f32));
        assert!(
            match (row, messages.as_slice()) {
                (0, [Message::CopyCode(text)]) => text == description,
                (1, [Message::Chat(ChatMessage::AppendText(text))]) => text == description,
                (2..=4, [Message::View(ViewMessage::OpenUrlExternal(url))]) => {
                    url == &format!("{prefix}{}", urlencoding::encode(description))
                }
                _ => false,
            },
            "{messages:?}"
        );
    }

    let end = Point::new(card.x + 240.0, card.y + 100.0);
    for (point, event) in [
        (card, mouse::Event::ButtonPressed(mouse::Button::Left)),
        (end, mouse::Event::CursorMoved { position: end }),
        (end, mouse::Event::ButtonReleased(mouse::Button::Left)),
    ] {
        assert!(send_mouse(&mut scene, &mut renderer, point, event).is_empty());
    }
    assert!(send_mouse(&mut scene, &mut renderer, card, right).is_empty());
    let expected = format!("{}\n{description}", i18n.__("网址"));
    assert!(matches!(
        scene.click(&mut renderer, menu_item(0.0)).as_slice(),
        [Message::CopyCode(text)] if text == &expected
    ));
    assert!(
        scene.send(&mut renderer, &[key(Named::Escape)], mouse::Cursor::Unavailable).is_empty()
    );
    assert!(matches!(
        scene.press(&mut renderer, &[2, 1]).as_slice(),
        [Message::Settings(SettingsMessage::AppProviderEdit(EditMessage::Save))]
    ));
    assert!(matches!(
        scene.press(&mut renderer, &[2, 0]).as_slice(),
        [Message::Settings(SettingsMessage::AppProviderEdit(EditMessage::Cancel))]
    ));
}

pub(in crate::app::components::app_providers) fn assert_contextual_help(
    element: Element<'_, Message>,
    renderer: &mut Renderer,
    theme: &Theme,
    size: Size<u32>,
    name: &str,
    expected: usize,
) {
    let mut scene = Scene::new(element, renderer, size);
    let mut icons = Vec::new();
    help_icons(Layout::new(&scene.layout), &mut icons);
    assert_eq!(icons.len(), expected, "{name} contextual help targets");
    for (index, icon) in icons.into_iter().enumerate() {
        let name = format!("help-06-{name}-{index}");
        let point = visible_icon(&mut scene, renderer, icon);
        let idle =
            scene.capture(renderer, theme, mouse::Cursor::Unavailable, &format!("{name}-idle"));
        assert!(
            scene
                .send(
                    renderer,
                    &[Event::Mouse(mouse::Event::CursorMoved { position: point })],
                    mouse::Cursor::Available(point),
                )
                .is_empty()
        );
        let hover = scene.capture(
            renderer,
            theme,
            mouse::Cursor::Available(point),
            &format!("{name}-hover"),
        );
        assert!(idle != hover, "{name} previews a rendered help card");
        assert!(scene.click(renderer, point).is_empty());
        assert!(
            scene
                .send(
                    renderer,
                    &[Event::Mouse(mouse::Event::CursorLeft)],
                    mouse::Cursor::Unavailable,
                )
                .is_empty()
        );
        let pinned =
            scene.capture(renderer, theme, mouse::Cursor::Unavailable, &format!("{name}-pinned"));
        assert!(idle != pinned, "{name} keeps the card open away from the target");
        assert!(scene.send(renderer, &[key(Named::Escape)], mouse::Cursor::Unavailable).is_empty());
        assert!(
            idle == scene.capture(
                renderer,
                theme,
                mouse::Cursor::Unavailable,
                &format!("{name}-dismissed")
            ),
            "{name} dismisses without changing the parent",
        );
    }
}

fn visible_icon(scene: &mut Scene<'_>, renderer: &mut Renderer, icon: Rectangle) -> Point {
    let mut scroll = BodyViewport::default();
    scene.ui.operate(renderer, &mut scroll);
    let Some((bounds, offset)) = scroll.0 else { return icon.center() };
    if icon.y < bounds.y {
        return icon.center();
    }
    let point = icon.center() - offset;
    if bounds.contains(point) {
        return point;
    }
    assert!(
        scene
            .send(
                renderer,
                &[Event::Mouse(mouse::Event::WheelScrolled {
                    delta: mouse::ScrollDelta::Pixels { x: 0.0, y: bounds.center().y - point.y },
                })],
                mouse::Cursor::Available(bounds.center()),
            )
            .is_empty()
    );
    scroll.0 = None;
    scene.ui.operate(renderer, &mut scroll);
    let point = icon.center() - scroll.0.unwrap().1;
    assert!(bounds.contains(point), "{icon:?} stays reachable after scrolling {bounds:?}");
    point
}

#[derive(Default)]
struct BodyViewport(Option<(Rectangle, Vector)>);

impl Operation for BodyViewport {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
        operate(self);
    }

    fn scrollable(
        &mut self,
        _id: Option<&iced::advanced::widget::Id>,
        bounds: Rectangle,
        _content: Rectangle,
        translation: Vector,
        _state: &mut dyn Scrollable,
    ) {
        self.0.get_or_insert((bounds, translation));
    }
}

fn help_icons(layout: Layout<'_>, icons: &mut Vec<Rectangle>) {
    let bounds = layout.bounds();
    if bounds.size() == Size::new(22.0, 22.0) && !icons.contains(&bounds) {
        icons.push(bounds);
    }
    for child in layout.children() {
        help_icons(child, icons);
    }
}

#[derive(Default)]
struct BodyText(Vec<(String, Rectangle)>);

impl Operation for BodyText {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
        operate(self);
    }

    fn text(&mut self, _: Option<&iced::advanced::widget::Id>, bounds: Rectangle, value: &str) {
        self.0.push((value.to_owned(), bounds));
    }
}

impl Scene<'_> {
    pub(in crate::app::components::app_providers) fn assert_body(
        &mut self,
        renderer: &Renderer,
        hidden: &[&str],
        visible: &[&str],
    ) {
        let mut body = BodyText::default();
        self.ui.operate(renderer, &mut body);
        for phrase in hidden {
            assert!(
                !body.0.iter().any(|(text, _)| text.contains(phrase)),
                "still in body: {phrase}"
            );
        }
        for phrase in visible {
            assert!(body.0.iter().any(|(text, _)| text == phrase), "missing from body: {phrase}");
        }
    }

    pub(in crate::app::components::app_providers) fn click_label(
        &mut self,
        renderer: &mut Renderer,
        label: &str,
    ) -> Vec<Message> {
        let bounds = self.text_bounds(renderer, label);
        let point = visible_icon(self, renderer, bounds);
        self.click(renderer, point)
    }

    fn text_bounds(&mut self, renderer: &Renderer, label: &str) -> Rectangle {
        let mut body = BodyText::default();
        self.ui.operate(renderer, &mut body);
        body.0
            .into_iter()
            .find(|(text, _)| text == label)
            .unwrap_or_else(|| panic!("missing {label}"))
            .1
    }

    pub(in crate::app::components::app_providers) fn assert_help_copy(
        &mut self,
        renderer: &mut Renderer,
        theme: &Theme,
        label: &str,
        description: &str,
    ) {
        let bounds = self.text_bounds(renderer, label);
        let mut icons = Vec::new();
        help_icons(Layout::new(&self.layout), &mut icons);
        let icon = icons
            .into_iter()
            .find(|icon| {
                (icon.center_y() - bounds.center_y()).abs() < 1.0
                    && (icon.x - bounds.x - bounds.width - 4.0).abs() < 1.0
            })
            .unwrap_or_else(|| panic!("missing help icon for {label}: {bounds:?}"));
        let point = visible_icon(self, renderer, icon);
        let cursor = mouse::Cursor::Available(point);
        let idle = self.capture(renderer, theme, mouse::Cursor::Unavailable, "");
        assert!(
            self.send(
                renderer,
                &[Event::Mouse(mouse::Event::CursorMoved { position: point })],
                cursor
            )
            .is_empty()
        );
        assert!(
            idle != self.capture(renderer, theme, cursor, ""),
            "help for {label} renders on hover"
        );
        assert!(self.click(renderer, point).is_empty());
        assert!(
            self.send(
                renderer,
                &[Event::Mouse(mouse::Event::CursorLeft)],
                mouse::Cursor::Unavailable
            )
            .is_empty()
        );
        assert!(
            idle != self.capture(renderer, theme, mouse::Cursor::Unavailable, ""),
            "help for {label} stays pinned"
        );

        let right = point.x + icon.width / 2.0 + 4.0;
        let left = point.x - icon.width / 2.0 - 4.0;
        let right_space = self.size.width as f32 - right;
        let card_x = if right_space >= 300.0 || right_space >= left {
            right
        } else {
            (left - 300.0).max(0.0)
        };
        let card = Point::new(card_x + 12.5, point.y + 5.0);
        assert!(
            self.send(
                renderer,
                &[Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right))],
                mouse::Cursor::Available(card)
            )
            .is_empty()
        );
        let row_height =
            f32::from(iced::advanced::text::LineHeight::default().to_absolute(12.0.into())) + 10.0;
        let copy = Point::new(card.x + 24.0, card.y + 3.0 + row_height * 0.5);
        let messages = self.click(renderer, copy);
        assert!(
            matches!(messages.as_slice(), [Message::CopyCode(value)] if value == description),
            "{label}: {messages:?}"
        );
        assert!(self.send(renderer, &[key(Named::Escape)], mouse::Cursor::Unavailable).is_empty());
        assert!(
            idle == self.capture(renderer, theme, mouse::Cursor::Unavailable, ""),
            "Esc dismisses help for {label}"
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn settings_help_gap_16_catalog_alias_endpoint_and_discovery() {
    use crate::app::message::settings::providers::{catalog_api::Event as CatalogEvent, discovery};
    use crate::app::state::app_providers::model_catalog::{
        CatalogContext, CatalogForm, DiscoveryPhase,
    };
    use vw_gateway_client::vw_api_types::model_catalog::{
        ModelCatalogEntry, ModelCatalogProject, ModelCatalogReadResponse,
        ScopedModelCatalogIdentity,
    };

    let mut renderer = renderer().await;
    for locale in [
        DesktopLocale::SimplifiedChinese,
        DesktopLocale::TraditionalChinese,
        DesktopLocale::English,
    ] {
        let i18n = I18n::new(locale);
        for (theme, width) in [(Theme::Light, 860), (Theme::Dark, 420)] {
            for editing in [false, true] {
                let mut state = list_fixture(AppId::OpenCode);
                let mut form = CatalogForm::new(
                    CatalogContext {
                        scope: state.scope.clone().unwrap(),
                        project: ModelCatalogProject {
                            project_id: "fixture-project".into(),
                            directory: "/fixture/project".into(),
                        },
                        access_generation: 0,
                    },
                    "fixture-id".into(),
                );
                form.endpoint = "https://example.invalid/v1".into();
                let identity = ScopedModelCatalogIdentity {
                    scope: Default::default(),
                    deployment: form.deployment(),
                    model_id: "fixture-saved-model".into(),
                };
                form.catalog = Some(ModelCatalogReadResponse {
                    revision: "fixture-revision".into(),
                    default_model_id: None,
                    items: vec![ModelCatalogEntry {
                        identity: identity.clone(),
                        display_alias: Some("Fixture alias".into()),
                        metadata: Default::default(),
                    }],
                    next_cursor: None,
                });
                if editing {
                    form.edit_model(identity.model_id.clone());
                } else {
                    form.manual_id = "fixture-new-model".into();
                }
                form.discovery.results = vec![ScopedModelCatalogIdentity {
                    model_id: "fixture-model".into(),
                    ..identity
                }]
                .into();
                form.discovery.phase = DiscoveryPhase::Completed(form.discovery.results.clone());
                form.discovery.results_run = 7;
                state.model_catalog = Some(form);

                let mut scene = Scene::new(
                    list::overlay(&state, Help::new(i18n, &theme), base()),
                    &mut renderer,
                    Size::new(width, 920),
                );
                let help = [
                    ("显示别名", "别名仅用于显示，发送请求时使用原始模型 ID。"),
                    ("目标部署端点", "选择下方预设端点，也可以直接输入。"),
                    ("发现模型", "使用执行主机上已保存的供应商配置或 CLI。"),
                ];
                for (label, copy) in help {
                    scene.assert_body(&renderer, &[copy, i18n.__(copy)], &[i18n.__(label)]);
                }
                scene.assert_body(
                    &renderer,
                    &[],
                    &[
                        i18n.__("切换端点会清空当前草稿与已选模型。"),
                        i18n.__("勾选结果后可保存到此部署。"),
                        "发现已完成。",
                        "fixture-model",
                    ],
                );
                for (label, copy) in help {
                    scene.assert_help_copy(&mut renderer, &theme, i18n.__(label), i18n.__(copy));
                }
                assert!(matches!(
                    scene.click_label(&mut renderer, "刷新已保存模型").as_slice(),
                    [Message::Settings(SettingsMessage::AppProviders(ListMessage::ModelCatalog(
                        CatalogEvent::LoadCatalog
                    )))]
                ));
                assert!(matches!(
                    scene.click_label(&mut renderer, "fixture-model").as_slice(),
                    [Message::Settings(SettingsMessage::AppProviders(ListMessage::ModelCatalog(
                        CatalogEvent::Discovery(discovery::Event::Toggle {
                            run: 7,
                            index: 0,
                            checked: true
                        })
                    )))]
                ));
                let action = if editing { "取消编辑" } else { "编辑" };
                assert!(matches!(
                    (editing, scene.click_label(&mut renderer, action).as_slice()),
                    (
                        true,
                        [Message::Settings(SettingsMessage::AppProviders(
                            ListMessage::ModelCatalog(CatalogEvent::CancelEdit)
                        ))]
                    ) | (
                        false,
                        [Message::Settings(SettingsMessage::AppProviders(
                            ListMessage::ModelCatalog(CatalogEvent::EditModel { .. })
                        ))]
                    )
                ));
            }
        }
    }
}
