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

    fn press(&mut self, renderer: &mut Renderer, path: &[usize]) -> Vec<Message> {
        self.click(renderer, self.modal(path).center())
    }

    fn capture(
        &mut self,
        renderer: &mut Renderer,
        theme: &Theme,
        cursor: mouse::Cursor,
        name: &str,
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
            .expect("save screenshot");
        }
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
            (list::overlay(&list, base()), "list", 5),
            (edit::overlay(&edit, base()), "edit", 2),
            (super::overlay(&desktop, base()), "desktop", 4),
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

    let mut scene = Scene::new(list::overlay(&list, base()), &mut renderer, Size::new(420, 760));
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

    let mut scene =
        Scene::new(super::overlay(&desktop, base()), &mut renderer, Size::new(420, 760));
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
        let mut scene = Scene::new(super::overlay(form, base()), &mut renderer, size);
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
        let mut scene = Scene::new(super::overlay(&form, base()), &mut renderer, size);
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
        let mut scene = Scene::new(list::overlay(&list, base()), &mut renderer, size);
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
    let mut scene = Scene::new(list::overlay(&list, base()), &mut renderer, size);
    assert!(matches!(
        scene.press(&mut renderer, &[2, 0]).as_slice(),
        [Message::Settings(SettingsMessage::AppProviders(ListMessage::Cancel))]
    ));
    scene.capture(&mut renderer, &Theme::Dark, mouse::Cursor::Unavailable, "list-presentation");

    let form = edit_fixture(Mode::Delete);
    let mut scene = Scene::new(edit::overlay(&form, base()), &mut renderer, size);
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
    let mut scene = Scene::new(edit::overlay(&form, base()), &mut renderer, size);
    assert!(scene.press(&mut renderer, &[2, 1]).is_empty());
    assert!(scene.press(&mut renderer, &[2, 0]).is_empty());
    scene.capture(&mut renderer, &Theme::Light, mouse::Cursor::Unavailable, "edit-saving");
}
