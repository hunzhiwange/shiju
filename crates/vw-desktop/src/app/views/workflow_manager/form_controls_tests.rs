use super::{WorkflowDraft, WorkflowDraftStatus, WorkflowManagerMessage, build_draft_editor};
use crate::app::{Message, i18n::I18n, task::StatusRole};
use iced::advanced::renderer::{Headless, Style};
use iced::advanced::widget::operation::{Focusable, Operation};
use iced::advanced::{Layout, clipboard, layout, widget::Tree};
use iced::keyboard::{self, Key, key::Named};
use iced::{Element, Event, Font, Rectangle, Renderer, Size, Theme, mouse};
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
        let mut messages = Vec::new();
        self.ui.update(
            &[event],
            mouse::Cursor::Unavailable,
            renderer,
            &mut clipboard::Null,
            &mut messages,
        );
        messages
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
                let route_name = field(&scene, &[5, 1, 0, 2]);
                let route_key = field(&scene, &[5, 1, 1, 2]);
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
