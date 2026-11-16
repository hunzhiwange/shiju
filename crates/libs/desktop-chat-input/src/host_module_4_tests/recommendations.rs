use crate::input_panel::{quick_message::Message, quick_messages as domain};
use iced::advanced::widget::Tree;
#[derive(Default)]
struct Fixture {
    prompts: Vec<String>,
    popovers: desktop_session::message::chat::input::popovers::InputPopoverState,
    runtime: desktop_session::session::agent::SessionRuntimeState<()>,
    hub: desktop_skill_hub::skill_hub::ExpertHubState,
}
impl Fixture {
    fn input(&self) -> domain::QuickInput<'_> {
        domain::QuickInput {
            i18n: desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
            quick_messages: &[],
            quick_message_group: self.popovers.quick_message_group,
            recommended_prompt: self.popovers.recommended_prompt,
            recommended_prompts: Some(&self.prompts),
            show_workspace: false,
            project_is_git_repo: false,
            recent_projects: &[],
            recent_projects_edits: &[],
            project_path: None,
            work_space_picker_open: false,
            shiju_skill: false,
            installed: &self.hub.installed,
            package_status: &self.hub.package_status,
            tool_selector: None,
        }
    }
    fn update(&mut self, message: Message) {
        match message {
            Message::RecommendedPromptMenuChanged(open) => {
                self.popovers.set_recommended_prompt(open)
            }
            Message::AgentRecommendedPromptSelected(index) => {
                self.popovers.recommended_prompt = false;
                let _ = crate::input_panel::quick_actions::fill_recommendation(
                    &mut self.runtime.input_editor,
                    self.prompts.get(index).map(String::as_str),
                )
                .unwrap();
            }
            _ => panic!("unexpected recommendation event"),
        }
    }
}
#[tokio::test(flavor = "multi_thread")]
async fn session_entry_recommendations_click_select_and_close() {
    use iced::advanced::renderer::Headless;
    use iced::advanced::{Layout, Shell, layout, mouse};
    use iced::{Event, Font, Pixels, Point, Rectangle, Size, Vector};
    let mut app = Fixture::default();
    app.prompts = vec![" ".into(), "点击后只填入草稿".into()];
    let renderer =
        <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
            .await
            .unwrap();
    let bounds = Size::new(260.0, 400.0);
    let viewport = Rectangle::with_size(bounds);
    // Exercise the real trigger twice: open, then close.
    for open in [true, false, true] {
        let mut element =
            domain::recommendations_menu(app.input(), &domain::recommended_indices(app.input()));
        let mut tree = Tree::new(element.as_widget());
        let node = element.as_widget_mut().layout(
            &mut tree,
            &renderer,
            &layout::Limits::new(Size::ZERO, bounds),
        );
        let mut messages = Vec::new();
        for event in [
            mouse::Event::ButtonPressed(mouse::Button::Left),
            mouse::Event::ButtonReleased(mouse::Button::Left),
        ] {
            element.as_widget_mut().update(
                &mut tree,
                &Event::Mouse(event),
                Layout::new(&node),
                mouse::Cursor::Available(node.bounds().center()),
                &renderer,
                &mut iced::advanced::clipboard::Null,
                &mut Shell::new(&mut messages),
                &viewport,
            );
        }
        drop(element);
        assert_eq!(messages.len(), 1);
        let message = messages.remove(0);
        assert!(matches!(message, Message::RecommendedPromptMenuChanged(value) if value == open));
        app.update(message);
        assert_eq!(app.popovers.recommended_prompt, open);
    }
    // Place the trigger at the bottom, as in the chat input strip.
    let mut element =
        domain::recommendations_menu(app.input(), &domain::recommended_indices(app.input()));
    let mut tree = Tree::new(element.as_widget());
    let node = element
        .as_widget_mut()
        .layout(&mut tree, &renderer, &layout::Limits::new(Size::ZERO, bounds))
        .move_to(Point::new(0.0, 360.0));
    let mut overlay = element
        .as_widget_mut()
        .overlay(&mut tree, Layout::new(&node), &renderer, &viewport, Vector::ZERO)
        .expect("open recommendation overlay");
    let overlay_node = overlay.as_overlay_mut().layout(&renderer, bounds);
    assert!(overlay_node.bounds().y >= 0.0);
    assert!(overlay_node.bounds().x + overlay_node.bounds().width <= bounds.width);
    let mut messages = Vec::new();
    let position = Point::new(
        overlay_node.children()[0].bounds().x + 25.0,
        overlay_node.children()[0].bounds().y + 20.0,
    );
    for event in [
        mouse::Event::ButtonPressed(mouse::Button::Left),
        mouse::Event::ButtonReleased(mouse::Button::Left),
    ] {
        overlay.as_overlay_mut().update(
            &Event::Mouse(event),
            Layout::new(&overlay_node),
            mouse::Cursor::Available(position),
            &renderer,
            &mut iced::advanced::clipboard::Null,
            &mut Shell::new(&mut messages),
        );
    }
    assert_eq!(messages.len(), 1);
    assert!(matches!(messages[0], Message::AgentRecommendedPromptSelected(1)));
    drop(overlay);
    drop(element);
    let message = messages.remove(0);
    app.update(message);
    assert_eq!(app.runtime.input_editor.text(), "点击后只填入草稿");
    assert!(!app.popovers.recommended_prompt);
    assert!(!app.runtime.request_lifecycle.is_requesting);

    app.popovers.recommended_prompt = true;
    let mut element =
        domain::recommendations_menu(app.input(), &domain::recommended_indices(app.input()));
    let mut tree = Tree::new(element.as_widget());
    let node = element
        .as_widget_mut()
        .layout(&mut tree, &renderer, &layout::Limits::new(Size::ZERO, bounds))
        .move_to(Point::new(0.0, 360.0));
    let mut overlay = element
        .as_widget_mut()
        .overlay(&mut tree, Layout::new(&node), &renderer, &viewport, Vector::ZERO)
        .unwrap();
    let overlay_node = overlay.as_overlay_mut().layout(&renderer, bounds);
    let mut messages = Vec::new();
    overlay.as_overlay_mut().update(
        &Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
        Layout::new(&overlay_node),
        mouse::Cursor::Available(Point::new(255.0, 395.0)),
        &renderer,
        &mut iced::advanced::clipboard::Null,
        &mut Shell::new(&mut messages),
    );
    assert!(matches!(messages.as_slice(), [Message::RecommendedPromptMenuChanged(false)]));
}
