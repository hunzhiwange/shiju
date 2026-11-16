use desktop_settings_appearance::companion::size_modal::{Action, View, with_size_modal};
use desktop_task_board::pet_state::TaskPetAvatarKind;
#[derive(Clone, Debug)]
enum Message {
    Companion(desktop_settings_appearance::companion::Action<TaskPetAvatarKind>),
    Size(Action),
    Underlying,
}
fn preview(kind: TaskPetAvatarKind, size: f32) -> iced::Element<'static, Message> {
    desktop_task_board::task_pet::avatar_preview(kind, size).map(|_| Message::Underlying)
}
#[derive(Default)]
struct Labels(Vec<(String, iced::Rectangle)>);
impl iced::advanced::widget::Operation for Labels {
    fn traverse(&mut self, children: &mut dyn FnMut(&mut dyn iced::advanced::widget::Operation)) {
        children(self);
    }
    fn text(
        &mut self,
        _: Option<&iced::advanced::widget::Id>,
        bounds: iced::Rectangle,
        value: &str,
    ) {
        self.0.push((value.into(), bounds));
    }
}

#[tokio::test]
async fn mini_companion_size_modal_renders_themes_and_routes_cancel_reset() {
    use iced::advanced::{clipboard, renderer::Headless};
    use iced::{Event, Font, Size, Theme, mouse};
    use iced_runtime::user_interface::{Cache, UserInterface};
    {
        let mut fonts = iced::advanced::graphics::text::font_system().write().unwrap();
        for font in desktop_assets::fonts::load_all() {
            fonts.load_font(font);
        }
    }
    let mut renderer =
        iced::Renderer::new(Font::DEFAULT, 14.0.into(), Some("tiny-skia")).await.unwrap();
    for scale in [0.5, 1.0, 2.0] {
        let base =
            iced::widget::container(iced::widget::button("底层按钮").on_press(Message::Underlying))
                .width(iced::Length::Fill)
                .height(iced::Length::Fill)
                .into();
        let mut ui = UserInterface::build(
            with_size_modal(
                &View {
                    desktop_locale: config_ui::DesktopLocale::SimplifiedChinese,
                    task_pet_size_draft: Some(scale),
                    pending: false,
                    task_pet_avatar_kind: TaskPetAvatarKind::Robot,
                    none: TaskPetAvatarKind::None,
                },
                Message::Size,
                preview,
                base,
            ),
            Size::new(640.0, 520.0),
            Cache::new(),
            &mut renderer,
        );
        let mut labels = Labels::default();
        ui.operate(&renderer, &mut labels);
        for theme in [Theme::Light, Theme::Dark] {
            ui.draw(&mut renderer, &theme, &Default::default(), mouse::Cursor::Unavailable);
            let pixels = renderer.screenshot(Size::new(640, 520), 1.0, iced::Color::TRANSPARENT);
            assert!(pixels.chunks_exact(4).any(|p| p[3] != 0));
        }
        for label in ["取消", "重置"] {
            let bounds = labels.0.iter().find(|(s, _)| s == label).unwrap().1;
            let mut messages = Vec::new();
            ui.update(
                &[
                    Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
                    Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
                ],
                mouse::Cursor::Available(bounds.center()),
                &mut renderer,
                &mut clipboard::Null,
                &mut messages,
            );
            if label == "取消" {
                assert!(
                    matches!(messages.as_slice(), [Message::Size(Action::TaskPetSizeCancel)]),
                    "cancel bounds={bounds:?}, messages={messages:?}"
                );
            } else if scale == 1.0 {
                assert!(messages.is_empty());
            } else {
                assert!(matches!(messages.as_slice(), [Message::Size(Action::TaskPetSizeReset)]));
            }
        }
        let mut messages = Vec::new();
        ui.update(
            &[
                Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
                Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
            ],
            mouse::Cursor::Available(iced::Point::new(10.0, 10.0)),
            &mut renderer,
            &mut clipboard::Null,
            &mut messages,
        );
        assert!(messages.is_empty(), "modal must isolate the underlying button");
    }
}

use desktop_i18n::I18n;
use desktop_settings_appearance::companion::{self, Action as CompanionAction};
#[path = "../../desktop-settings-appearance/src/companion_test_support.rs"]
mod rendering;
const AVATARS: [TaskPetAvatarKind; 4] = [
    TaskPetAvatarKind::None,
    TaskPetAvatarKind::Robot,
    TaskPetAvatarKind::Beauty,
    TaskPetAvatarKind::Handsome,
];
const COMPANIONS: [(TaskPetAvatarKind, &str, &str); 4] = [
    (TaskPetAvatarKind::None, companion::LABELS[0].0, companion::LABELS[0].1),
    (TaskPetAvatarKind::Robot, companion::LABELS[1].0, companion::LABELS[1].1),
    (TaskPetAvatarKind::Beauty, companion::LABELS[2].0, companion::LABELS[2].1),
    (TaskPetAvatarKind::Handsome, companion::LABELS[3].0, companion::LABELS[3].1),
];
fn fixture() -> companion::View<TaskPetAvatarKind> {
    companion::View {
        desktop_locale: config_ui::DesktopLocale::SimplifiedChinese,
        task_pet_visible: true,
        task_pet_avatar_kind: TaskPetAvatarKind::Robot,
        preview_scale: 1.0,
    }
}
async fn scene(
    app: &companion::View<TaskPetAvatarKind>,
    width: f32,
    click: Option<&str>,
) -> (rendering::Scene, Vec<Message>) {
    let index = AVATARS.iter().position(|kind| *kind == app.task_pet_avatar_kind).unwrap();
    rendering::scene(
        companion::content(app, AVATARS, Message::Companion, preview),
        companion::preview_id(index),
        width,
        click,
    )
    .await
}
#[tokio::test]
async fn mini_companion_settings_cards_render_and_emit_real_selection_events() {
    let mut app = fixture();
    app.desktop_locale = config_ui::DesktopLocale::SimplifiedChinese;
    app.task_pet_visible = true;
    for &(avatar, name, _) in &COMPANIONS {
        let (_, messages) = scene(&app, 800.0, Some(name)).await;
        assert!(
            matches!(messages.as_slice(), [Message::Companion(CompanionAction::TaskPetAvatarSelected(value))] if *value == avatar)
        );
        // Host integration below separately checks selection update and window ownership.
        app.task_pet_avatar_kind = avatar;
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
    assert!(matches!(
        messages.as_slice(),
        [Message::Companion(CompanionAction::TaskPetToggleWindow)]
    ));
}

#[tokio::test]
async fn mini_companion_settings_english_labels_render_without_actions_in_preview() {
    let mut app = fixture();
    app.desktop_locale = config_ui::DesktopLocale::English;
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
