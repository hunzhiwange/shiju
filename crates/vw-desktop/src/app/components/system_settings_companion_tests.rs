use super::*;
use crate::app::App;
use crate::app::components::system_settings_search::{self, SystemSettingsSearchTarget};
use crate::app::message::{settings, view};
use crate::app::state::{GeneralSettingsTab, TaskPetAvatarKind};

#[test]
fn mini_companion_settings_selection_updates_preview_and_window() {
    let mut app = App::new().0;
    for avatar in [TaskPetAvatarKind::Robot, TaskPetAvatarKind::Beauty, TaskPetAvatarKind::Handsome]
    {
        let _ = view::update(&mut app, view::ViewMessage::TaskPetAvatarSelected(avatar));
        assert_eq!(app.task_pet_avatar_kind, avatar);
        std::hint::black_box(content(&app));
    }
}

#[test]
fn mini_companion_settings_none_keeps_mini_visible() {
    let mut app = App::new().0;
    app.task_pet_visible = true;
    let id = iced::window::Id::unique();
    app.task_pet_window_id = Some(id);
    let _ =
        view::update(&mut app, view::ViewMessage::TaskPetAvatarSelected(TaskPetAvatarKind::None));
    assert!(app.task_pet_visible);
    assert_eq!(app.task_pet_window_id, Some(id));
    std::hint::black_box(content(&app));
}

#[test]
fn mini_companion_settings_search_reaches_page() {
    let mut app = App::new().0;
    for query in ["伙伴", "companion", "mini"] {
        let entry = system_settings_search::catalog(&app)
            .into_iter()
            .find(|entry| {
                entry.matches(query)
                    && entry.target
                        == SystemSettingsSearchTarget::General(GeneralSettingsTab::Companion)
            })
            .expect("companion page must be searchable");
        let _ = settings::update(
            &mut app,
            settings::SettingsMessage::SystemSearchTargetSelected(entry.target),
        );
        assert_eq!(app.general_settings_tab, GeneralSettingsTab::Companion);
    }
    let _ = view::update(&mut app, view::ViewMessage::TaskPetOpenSettings);
    assert!(app.show_system_settings);
    assert_eq!(app.general_settings_tab, GeneralSettingsTab::Companion);
}

#[derive(Default)]
struct Scene {
    labels: Vec<(String, iced::Rectangle)>,
    preview: Option<iced::Rectangle>,
    expected_preview: &'static str,
}

impl iced::advanced::widget::Operation for Scene {
    fn traverse(&mut self, children: &mut dyn FnMut(&mut dyn iced::advanced::widget::Operation)) {
        children(self);
    }
    fn text(
        &mut self,
        _: Option<&iced::advanced::widget::Id>,
        bounds: iced::Rectangle,
        label: &str,
    ) {
        self.labels.push((label.into(), bounds));
    }
    fn container(&mut self, id: Option<&iced::advanced::widget::Id>, bounds: iced::Rectangle) {
        if id == Some(&iced::advanced::widget::Id::new(self.expected_preview)) {
            self.preview = Some(bounds);
        }
    }
}

impl Scene {
    fn bounds(&self, label: &str) -> iced::Rectangle {
        self.labels
            .iter()
            .find(|(value, _)| value == label)
            .unwrap_or_else(|| panic!("missing {label}"))
            .1
    }
}

async fn scene(app: &App, width: f32, click: Option<&str>) -> (Scene, Vec<crate::app::Message>) {
    use iced::advanced::{clipboard, renderer::Headless};
    use iced::{Color, Event, Font, Size, Theme, mouse};
    use iced_runtime::user_interface::{Cache, UserInterface};
    {
        let mut fonts = iced::advanced::graphics::text::font_system().write().unwrap();
        for font in crate::fonts::load_all() {
            fonts.load_font(font);
        }
    }
    let mut renderer =
        iced::Renderer::new(Font::DEFAULT, 14.0.into(), Some("tiny-skia")).await.unwrap();
    let size = Size::new(width, 1600.0);
    let mut ui = UserInterface::build(content(app), size, Cache::new(), &mut renderer);
    let mut scene =
        Scene { expected_preview: preview_id(app.task_pet_avatar_kind), ..Default::default() };
    ui.operate(&renderer, &mut scene);
    assert!(scene.preview.is_some(), "preview must use selected avatar");
    // Drawing traverses real font shaping, including Chinese text and both themes.
    for theme in [Theme::Light, Theme::Dark] {
        ui.draw(&mut renderer, &theme, &Default::default(), mouse::Cursor::Unavailable);
        let pixels = renderer.screenshot(Size::new(width as u32, 1600), 1.0, Color::TRANSPARENT);
        assert!(pixels.chunks_exact(4).any(|pixel| pixel[3] != 0));
        if let Some(dir) = std::env::var_os("SHIJU_MINI_TASK4_SCREENSHOTS") {
            std::fs::create_dir_all(&dir).unwrap();
            image::save_buffer(
                std::path::PathBuf::from(dir)
                    .join(format!("{width}-{:?}-{theme}.png", app.task_pet_avatar_kind)),
                &pixels,
                width as u32,
                1600,
                image::ColorType::Rgba8,
            )
            .unwrap();
        }
    }
    let mut messages = Vec::new();
    if let Some(label) = click {
        let point = scene.bounds(label).center();
        ui.update(
            &[
                Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
                Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
            ],
            mouse::Cursor::Available(point),
            &mut renderer,
            &mut clipboard::Null,
            &mut messages,
        );
    }
    (scene, messages)
}

#[tokio::test]
async fn mini_companion_settings_cards_render_and_emit_real_selection_events() {
    use crate::app::Message;
    let mut app = App::new().0;
    app.desktop_locale = vw_config_types::ui::DesktopLocale::SimplifiedChinese;
    app.task_pet_visible = true;
    let id = iced::window::Id::unique();
    app.task_pet_window_id = Some(id);
    for &(avatar, name, _) in &COMPANIONS {
        let (_, messages) = scene(&app, 800.0, Some(name)).await;
        assert!(
            matches!(messages.as_slice(), [Message::View(view::ViewMessage::TaskPetAvatarSelected(value))] if *value == avatar)
        );
        for message in messages {
            if let Message::View(message) = message {
                let _ = view::update(&mut app, message);
            }
        }
        assert_eq!(app.task_pet_avatar_kind, avatar);
        assert!(app.task_pet_visible);
        assert_eq!(app.task_pet_window_id, Some(id));
        let (rendered, _) = scene(&app, 800.0, None).await;
        let selected = rendered.bounds("已选择");
        assert!(selected.y > rendered.bounds(name).y);
        assert!((selected.center_x() - rendered.bounds(name).center_x()).abs() < 2.0);
        assert_eq!(rendered.labels.iter().filter(|(label, _)| label == "已选择").count(), 1);
        assert!(rendered.bounds("隐藏 Mini").width > 0.0);
    }
    for width in [320.0, 520.0, 800.0] {
        let (rendered, messages) = scene(&app, width, Some("迷你工具栏")).await;
        assert!(messages.is_empty(), "preview must not drag or open a window");
        let none = rendered.bounds("无");
        let robot = rendered.bounds("识矩小机器人");
        let beauty = rendered.bounds("识矩小雅");
        if width < 440.0 {
            assert!(robot.y > none.y);
        } else if width < 720.0 {
            assert_eq!(robot.y, none.y);
            assert!(beauty.y > robot.y);
        } else {
            assert_eq!(none.y, beauty.y);
        }
        for (_, bounds) in &rendered.labels {
            assert!(bounds.x >= 0.0 && bounds.x + bounds.width <= width + 1.0);
        }
    }
    app.task_pet_visible = false;
    let (rendered, messages) = scene(&app, 800.0, Some("显示 Mini")).await;
    assert!(rendered.bounds("显示 Mini").width > 0.0);
    assert!(matches!(messages.as_slice(), [Message::View(view::ViewMessage::TaskPetToggleWindow)]));
}

#[tokio::test]
async fn mini_companion_settings_english_labels_render_without_actions_in_preview() {
    let mut app = App::new().0;
    app.desktop_locale = vw_config_types::ui::DesktopLocale::English;
    app.task_pet_avatar_kind = TaskPetAvatarKind::Robot;
    app.task_pet_visible = false;
    let (rendered, messages) = scene(&app, 520.0, Some("Mini toolbar")).await;
    assert!(messages.is_empty());
    assert!(rendered.bounds("Mini and companions").width > 0.0);
    assert!(rendered.bounds("Show Mini").width > 0.0);
    for &(_, name, description) in &COMPANIONS {
        let i18n = I18n::new(app.desktop_locale);
        assert_ne!(i18n.__(name), name);
        assert_ne!(i18n.__(description), description);
        assert!(rendered.bounds(i18n.__(name)).width > 0.0);
    }
}
