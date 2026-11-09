use super::{
    WorkflowDraft, WorkflowDraftStatus, WorkflowManagerMessage, build_draft_editor,
    owned_workflow_from_draft,
};
use crate::app::{Message, i18n::I18n, task::StatusRole};
use iced::advanced::renderer::{Headless, Style};
use iced::advanced::widget::operation::{Focusable, Operation};
use iced::advanced::{Layout, clipboard, layout, widget::Tree};
use iced::keyboard::{self, Key, key::Named};
use iced::{Element, Event, Font, Point, Rectangle, Renderer, Size, Theme, mouse};
use iced_runtime::user_interface::{Cache, UserInterface};
use vw_config_types::ui::DesktopLocale;

struct FocusControl(Rectangle);

impl Operation for FocusControl {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
        operate(self);
    }

    fn focusable(
        &mut self,
        _id: Option<&iced::advanced::widget::Id>,
        bounds: Rectangle,
        state: &mut dyn Focusable,
    ) {
        if bounds == self.0 {
            state.focus();
        } else {
            state.unfocus();
        }
    }
}

#[tokio::test]
async fn help_rollout_27_lifecycle_route_renders_contextual_help() {
    struct RouteLabel {
        explanation: &'static str,
    }

    impl Operation for RouteLabel {
        fn traverse(&mut self, children: &mut dyn FnMut(&mut dyn Operation)) {
            children(self);
        }

        fn text(&mut self, _: Option<&iced::advanced::widget::Id>, _: Rectangle, value: &str) {
            assert_ne!(value, self.explanation, "optional copy is hidden while idle");
        }
    }

    let mut renderer = Renderer::new(Font::DEFAULT, 14.0.into(), Some("tiny-skia")).await.unwrap();
    let i18n = I18n::new(DesktopLocale::English);
    let mut draft = WorkflowDraft::empty();
    let mut status = WorkflowDraftStatus::new("review", "Review", StatusRole::Queued);
    status.routes.clear();
    status.add_route();
    draft.statuses = vec![status];
    let mut scene = EditorScene::new(build_draft_editor(i18n, &draft), &mut renderer, 456);
    let mut label = RouteLabel { explanation: i18n.__("给人看的名称") };
    scene.ui.operate(&renderer, &mut label);
    fn help_bounds(layout: Layout<'_>) -> Option<Rectangle> {
        let bounds = layout.bounds();
        if bounds.size() == Size::new(22.0, 22.0) {
            return Some(bounds);
        }
        layout.children().find_map(help_bounds)
    }
    let route_name = [0, 4, 0, 0, 0, 0, 5, 0, 0, 0]
        .into_iter()
        .fold(Layout::new(&scene.layout), |node, index| node.children().nth(index).unwrap());
    let trigger = help_bounds(route_name).expect("workflow route help icon").center();
    let idle = scene.capture(&mut renderer, &Theme::Dark);
    assert!(scene.click(&mut renderer, trigger, mouse::Button::Left).is_empty());
    assert!(scene.capture(&mut renderer, &Theme::Dark) != idle);
    assert!(scene.key(&mut renderer, Key::Named(Named::Escape), None).is_empty());
    assert!(scene.capture(&mut renderer, &Theme::Dark) == idle);
}

struct FindText<'a>(&'a str, Option<Rectangle>);

impl Operation for FindText<'_> {
    fn traverse(&mut self, children: &mut dyn FnMut(&mut dyn Operation)) {
        children(self);
    }

    fn text(&mut self, _: Option<&iced::advanced::widget::Id>, bounds: Rectangle, value: &str) {
        if value == self.0 {
            self.1 = Some(bounds);
        }
    }
}

#[tokio::test]
async fn settings_help_gap_17_lifecycle_route_heading_and_controls() {
    {
        let mut fonts = iced::advanced::graphics::text::font_system().write().unwrap();
        for font in crate::fonts::load_all() {
            fonts.load_font(font);
        }
    }
    let mut renderer =
        Renderer::new(Font::with_name("Noto Sans CJK SC"), 14.0.into(), Some("tiny-skia"))
            .await
            .expect("tiny-skia renderer");

    let mut new_draft = WorkflowDraft::empty();
    new_draft.id = "review_flow".into();
    new_draft.label = "Review flow".into();
    let mut queued = WorkflowDraftStatus::new("queued", "Queued", StatusRole::Queued);
    queued.routes.truncate(1);
    queued.add_route();
    let mut done = WorkflowDraftStatus::new("done", "Done", StatusRole::Done);
    done.routes.clear();
    new_draft.statuses = vec![queued, done];

    let mut saved = new_draft.clone();
    saved.statuses[0].set_route_target_key(1, Some("done".into()));
    let saved = owned_workflow_from_draft(&saved).expect("valid lifecycle fixture");
    let saved_draft = WorkflowDraft::from_workflow(saved.into());

    for locale in [
        DesktopLocale::SimplifiedChinese,
        DesktopLocale::TraditionalChinese,
        DesktopLocale::English,
    ] {
        let i18n = I18n::new(locale);
        let explanation = i18n.__("结果 key 可自定义；目标留空表示此结果结束。");
        for (theme, width) in [(Theme::Light, 760), (Theme::Dark, 456)] {
            for draft in [&new_draft, &saved_draft] {
                let mut scene =
                    EditorScene::new(build_draft_editor(i18n, draft), &mut renderer, width);
                assert!(
                    scene.text_bounds(&renderer, explanation).is_none(),
                    "route explanation is still in the body: {explanation}"
                );
                let status = [0, 4, 0, 0, 0, 0];
                let field = |scene: &EditorScene<'_>, path: &[usize]| {
                    scene.bounds(&[status.as_slice(), path].concat())
                };
                assert!(scene.text_bounds(&renderer, i18n.__("结果去向")).is_some());
                let icon = field(&scene, &[4, 0, 1]);
                assert_eq!(icon.size(), Size::new(22.0, 22.0));
                let idle = scene.capture(&mut renderer, &theme);
                assert!(
                    scene
                        .send_at(
                            &mut renderer,
                            &[Event::Mouse(mouse::Event::CursorMoved { position: icon.center() })],
                            mouse::Cursor::Available(icon.center())
                        )
                        .is_empty()
                );
                assert!(scene.capture(&mut renderer, &theme) != idle, "help opens on hover");
                assert!(scene.click(&mut renderer, icon.center(), mouse::Button::Left).is_empty());
                let pinned = scene.capture(&mut renderer, &theme);
                assert!(pinned != idle, "help stays visible with the pointer unavailable");

                // The help card opens to the right of this icon. Copy its full body through
                // the selection menu without writing to the clipboard.
                let card = Point::new(icon.x + icon.width + 4.0 + 12.5, icon.center_y() + 5.0);
                assert!(scene.click(&mut renderer, card, mouse::Button::Right).is_empty());
                assert!(scene.capture(&mut renderer, &theme) != pinned);
                assert!(scene.key(&mut renderer, Key::Named(Named::Escape), None).is_empty());
                assert!(
                    scene.capture(&mut renderer, &theme) == pinned,
                    "Esc closes the menu first"
                );
                assert!(scene.click(&mut renderer, card, mouse::Button::Right).is_empty());
                let menu_row_height =
                    f32::from(iced::advanced::text::LineHeight::default().to_absolute(12.0.into()))
                        + 10.0;
                let copy = Point::new(card.x + 24.0, card.y + 3.0 + menu_row_height * 0.5);
                assert!(matches!(
                    scene.click(&mut renderer, copy, mouse::Button::Left).as_slice(),
                    [Message::CopyCode(value)] if value == explanation
                ));
                assert!(scene.key(&mut renderer, Key::Named(Named::Escape), None).is_empty());
                assert!(scene.capture(&mut renderer, &theme) == idle, "Esc closes the help card");

                for path in [&[5, 0, 0, 1][..], &[5, 0, 1, 1], &[5, 0, 3]] {
                    let point = field(&scene, path).center();
                    assert!(scene.click(&mut renderer, point, mouse::Button::Left).is_empty());
                    assert!(
                        scene.key(&mut renderer, Key::Character("x".into()), Some("x")).is_empty()
                    );
                }
                assert!(scene.text_bounds(&renderer, "succeed").is_some());
                let add = field(&scene, &[4, 2]).center();
                assert!(matches!(
                    scene.click(&mut renderer, add, mouse::Button::Left).as_slice(),
                    [Message::WorkflowManager(WorkflowManagerMessage::AddDraftStatusRoute(0))]
                ));
                let custom = draft.statuses[0].routes.len() - 1;
                let remove = field(&scene, &[5, custom, 3]).center();
                assert!(matches!(
                    scene.click(&mut renderer, remove, mouse::Button::Left).as_slice(),
                    [Message::WorkflowManager(WorkflowManagerMessage::RemoveDraftStatusRoute {
                        status_index: 0,
                        route_index,
                    })] if *route_index == custom
                ));

                let (custom_key, custom_target) = if draft.original_id.is_some() {
                    (Named::Home, None)
                } else {
                    (Named::End, Some("done"))
                };
                for (route_index, key, expected) in
                    [(0, Named::Home, None), (custom, custom_key, custom_target)]
                {
                    let target = field(&scene, &[5, route_index, 2]);
                    scene.ui.operate(&renderer, &mut FocusControl(target));
                    let closed = scene.capture(&mut renderer, &theme);
                    assert!(scene.key(&mut renderer, Key::Named(Named::Enter), None).is_empty());
                    assert!(scene.capture(&mut renderer, &theme) != closed, "route menu opens");
                    assert!(scene.key(&mut renderer, Key::Named(Named::Escape), None).is_empty());
                    assert!(scene.capture(&mut renderer, &theme) == closed, "route menu closes");
                    assert!(scene.key(&mut renderer, Key::Named(Named::Enter), None).is_empty());
                    assert!(scene.key(&mut renderer, Key::Named(key), None).is_empty());
                    assert!(matches!(
                        scene.key(&mut renderer, Key::Named(Named::Enter), None).as_slice(),
                        [Message::WorkflowManager(WorkflowManagerMessage::SetDraftStatusRoute {
                            status_index: 0,
                            route_index: index,
                            target_key,
                        })] if *index == route_index && target_key.as_deref() == expected
                    ));
                }
            }
        }
    }
}

struct EditorScene<'a> {
    ui: UserInterface<'a, Message, Theme, Renderer>,
    layout: layout::Node,
    size: Size<u32>,
}

impl<'a> EditorScene<'a> {
    fn new(mut view: Element<'a, Message>, renderer: &mut Renderer, width: u32) -> Self {
        let size = Size::new(width, 1280);
        let bounds = Size::new(size.width as f32, size.height as f32);
        let mut tree = Tree::new(view.as_widget());
        let layout = view.as_widget_mut().layout(
            &mut tree,
            renderer,
            &layout::Limits::new(Size::ZERO, bounds),
        );
        let ui = UserInterface::build(view, bounds, Cache::new(), renderer);
        Self { ui, layout, size }
    }

    fn bounds(&self, path: &[usize]) -> Rectangle {
        path.iter()
            .fold(Layout::new(&self.layout), |node, &index| {
                node.children().nth(index).unwrap_or_else(|| panic!("missing layout {path:?}"))
            })
            .bounds()
    }

    fn send(&mut self, renderer: &mut Renderer, event: Event) -> Vec<Message> {
        self.send_at(renderer, &[event], mouse::Cursor::Unavailable)
    }

    fn send_at(
        &mut self,
        renderer: &mut Renderer,
        events: &[Event],
        cursor: mouse::Cursor,
    ) -> Vec<Message> {
        let mut messages = Vec::new();
        self.ui.update(events, cursor, renderer, &mut clipboard::Null, &mut messages);
        messages
    }

    fn click(
        &mut self,
        renderer: &mut Renderer,
        point: Point,
        button: mouse::Button,
    ) -> Vec<Message> {
        self.send_at(
            renderer,
            &[
                Event::Mouse(mouse::Event::ButtonPressed(button)),
                Event::Mouse(mouse::Event::ButtonReleased(button)),
            ],
            mouse::Cursor::Available(point),
        )
    }

    fn text_bounds(&mut self, renderer: &Renderer, value: &str) -> Option<Rectangle> {
        let mut find = FindText(value, None);
        self.ui.operate(renderer, &mut find);
        find.1
    }

    fn key(&mut self, renderer: &mut Renderer, key: Key, text: Option<&str>) -> Vec<Message> {
        self.send(
            renderer,
            Event::Keyboard(keyboard::Event::KeyPressed {
                modified_key: key.clone(),
                key,
                physical_key: keyboard::key::Physical::Unidentified(
                    keyboard::key::NativeCode::Unidentified,
                ),
                location: keyboard::Location::Standard,
                modifiers: keyboard::Modifiers::empty(),
                text: text.map(Into::into),
                repeat: false,
            }),
        )
    }

    fn capture(&mut self, renderer: &mut Renderer, theme: &Theme) -> Vec<u8> {
        self.send(
            renderer,
            Event::Window(iced::window::Event::RedrawRequested(iced::time::Instant::now())),
        );
        self.ui.draw(
            renderer,
            theme,
            &Style { text_color: theme.palette().text },
            mouse::Cursor::Unavailable,
        );
        renderer.screenshot(self.size, 1.0, theme.palette().background)
    }

    fn open_menu(
        &mut self,
        renderer: &mut Renderer,
        theme: &Theme,
        trigger: Rectangle,
        name: &str,
    ) {
        self.ui.operate(renderer, &mut FocusControl(trigger));
        let closed = self.capture(renderer, theme);
        assert!(self.key(renderer, Key::Named(Named::Enter), None).is_empty());
        let open = self.capture(renderer, theme);

        // Both menus start on the first option. Sample the mark away from its label and borders.
        let mark_x = (trigger.x + trigger.width - 25.0) as usize;
        let mark_y = (trigger.y + trigger.height + 30.0) as usize;
        let pixel = |pixels: &[u8], x: usize, y: usize| {
            let offset = (y * self.size.width as usize + x) * 4;
            [pixels[offset], pixels[offset + 1], pixels[offset + 2], pixels[offset + 3]]
        };
        assert_ne!(
            pixel(&closed, mark_x, mark_y),
            pixel(&open, mark_x, mark_y),
            "{name} menu should open on Enter"
        );
        let background = pixel(&open, mark_x, mark_y - 8);
        let mark_pixels = (mark_y - 7..mark_y + 7)
            .flat_map(|y| (mark_x - 7..mark_x + 7).map(move |x| (x, y)))
            .filter(|&(x, y)| pixel(&open, x, y) != background)
            .count();
        assert!(mark_pixels > 10, "{name} menu should draw the selected checkmark");
    }
}

#[tokio::test]
async fn workflow_form_controls_open_shared_role_and_route_menus() {
    {
        let mut fonts = iced::advanced::graphics::text::font_system().write().unwrap();
        for font in crate::fonts::load_all() {
            fonts.load_font(font);
        }
    }
    let mut renderer =
        Renderer::new(Font::with_name("Noto Sans CJK SC"), 14.0.into(), Some("tiny-skia"))
            .await
            .expect("tiny-skia renderer");

    for theme in [Theme::Light, Theme::Dark] {
        for (locale, sample, typed) in [
            (DesktopLocale::SimplifiedChinese, "等待复核", "简"),
            (DesktopLocale::TraditionalChinese, "等待覆核", "繁"),
            (DesktopLocale::English, "Awaiting review", "E"),
        ] {
            let mut draft = WorkflowDraft::empty();
            draft.id = "review_flow".into();
            draft.label = sample.into();
            let mut previous = WorkflowDraftStatus::new("review", sample, StatusRole::Queued);
            previous.routes.clear();
            let mut edited = WorkflowDraftStatus::new("editing", "", StatusRole::Backlog);
            edited.routes.truncate(1);
            edited.add_route();
            edited.routes[1].label = sample.into();
            draft.statuses = vec![previous, edited];

            for width in [456, 760] {
                let view = build_draft_editor(I18n::new(locale), &draft);
                let mut scene = EditorScene::new(view, &mut renderer, width);
                let status = [0, 4, 0, 0, 1, 0];
                let field = |scene: &EditorScene<'_>, path: &[usize]| {
                    scene.bounds(&[status.as_slice(), path].concat())
                };
                let role = field(&scene, &[3, 1]);
                let route = field(&scene, &[5, 1, 2]);
                let route_name = field(&scene, &[5, 1, 0, 1]);
                let route_key = field(&scene, &[5, 1, 1, 1]);
                let remove = field(&scene, &[5, 1, 3]);
                for (left, right) in [(route_name, route_key), (route_key, route), (route, remove)]
                {
                    assert!(left.width >= 74.0 && left.x + left.width < right.x);
                }
                assert!(remove.x + remove.width <= width as f32);
                scene.open_menu(&mut renderer, &theme, role, "role");
                assert!(scene.key(&mut renderer, Key::Named(Named::ArrowDown), None).is_empty());
                assert!(matches!(
                    scene.key(&mut renderer, Key::Named(Named::Enter), None).as_slice(),
                    [Message::WorkflowManager(WorkflowManagerMessage::DraftStatusRoleChanged {
                        index: 1,
                        role: StatusRole::Queued,
                    })]
                ));

                scene.open_menu(&mut renderer, &theme, route, "route");
                assert!(scene.key(&mut renderer, Key::Named(Named::ArrowDown), None).is_empty());
                assert!(matches!(
                    scene.key(&mut renderer, Key::Named(Named::Enter), None).as_slice(),
                    [Message::WorkflowManager(WorkflowManagerMessage::SetDraftStatusRoute {
                        status_index: 1,
                        route_index: 1,
                        target_key: Some(key),
                    })] if key == "review"
                ));

                scene.open_menu(&mut renderer, &theme, route, "route");
                assert!(scene.key(&mut renderer, Key::Named(Named::End), None).is_empty());
                assert!(scene.key(&mut renderer, Key::Named(Named::Home), None).is_empty());
                assert!(matches!(
                    scene.key(&mut renderer, Key::Named(Named::Enter), None).as_slice(),
                    [Message::WorkflowManager(WorkflowManagerMessage::SetDraftStatusRoute {
                        status_index: 1,
                        route_index: 1,
                        target_key: None,
                    })]
                ));

                let label = field(&scene, &[2, 1]);
                scene.ui.operate(&renderer, &mut FocusControl(label));
                assert!(matches!(
                    scene.key(&mut renderer, Key::Character(typed.into()), Some(typed)).as_slice(),
                    [Message::WorkflowManager(WorkflowManagerMessage::DraftStatusLabelChanged {
                        index: 1,
                        label,
                    })] if label == typed
                ));
            }
        }
    }
}
