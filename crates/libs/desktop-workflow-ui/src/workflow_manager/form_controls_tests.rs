use super::{
    HandoffIdentity, HandoffKind, Message, StatusCategory, StepHandoff, TaskWorkflowRegistry,
    WorkflowManagerState, build_status_row, lifecycle_workflow_dto_from_owned_workflow, update,
};
use super::{
    WorkflowDraft, WorkflowDraftStatus, WorkflowManagerMessage, build_draft_editor,
    owned_workflow_from_draft,
};
use config_ui::DesktopLocale;
use desktop_i18n::I18n;
use desktop_task_model::StatusRole;
use iced::advanced::renderer::{Headless, Style};
use iced::advanced::widget::operation::{Focusable, Operation};
use iced::advanced::{Layout, clipboard, layout, widget::Tree};
use iced::keyboard::{self, Key, key::Named};
use iced::{Element, Event, Font, Point, Rectangle, Renderer, Size, Theme, mouse};
use iced_runtime::user_interface::{Cache, UserInterface};

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

        assert_ne!(closed, open, "{name} menu should open on Enter");
    }
}

#[test]
fn step_handoffs_survive_save_and_reopen() {
    let mut draft = WorkflowDraft::empty();
    draft.id = "handoff".into();
    draft.label = "Handoff".into();
    draft.statuses = vec![
        WorkflowDraftStatus::new("triage", "Triage", StatusRole::Backlog),
        WorkflowDraftStatus::new("work", "Work", StatusRole::Custom),
        WorkflowDraftStatus::new("done", "Done", StatusRole::Done),
    ];
    draft.statuses[0].handoff = StepHandoff::Member { member_id: "alice".into() };
    draft.statuses[1].handoff = StepHandoff::Agent {
        agent_id: "worker".into(),
        instructions: "Review the change".into(),
        completion_status_key: Some("done".into()),
        return_status_key: Some("triage".into()),
    };
    let saved = owned_workflow_from_draft(&draft).expect("valid step workflow");
    let dto = lifecycle_workflow_dto_from_owned_workflow(saved.clone());
    dto.validate().expect("new contract");
    let wire = serde_json::to_string(&dto).unwrap();
    let decoded: super::TaskLifecycleWorkflowDto = serde_json::from_str(&wire).unwrap();
    assert_eq!(decoded, dto);
    let reopened = WorkflowDraft::from_workflow(saved.into());
    for (before, after) in draft.statuses.iter().zip(&reopened.statuses) {
        assert_eq!(before.handoff, after.handoff);
    }
    assert_eq!(reopened.initial_status_key, "triage");
    let mut reordered = dto;
    reordered.initial_status_key = "work".into();
    reordered.steps.swap(0, 1);
    let mut registry = TaskWorkflowRegistry::new();
    // Reopening follows steps, even when the library order differs.
    let mut owned = owned_workflow_from_draft(&draft).unwrap();
    owned.initial_status_key = reordered.initial_status_key;
    owned.steps = reordered.steps;
    registry.replace_custom_workflows(vec![owned]).unwrap();
    let reopened = WorkflowDraft::from_workflow(registry.workflow_by_id("handoff").unwrap());
    assert_eq!(reopened.statuses[0].key, "work");
    assert_eq!(reopened.initial_status_key, "work");
}

#[tokio::test]
async fn step_form_shows_handoff_fields_in_all_locales_and_themes() {
    {
        let mut fonts = iced::advanced::graphics::text::font_system().write().unwrap();
        for font in desktop_assets::fonts::load_all() {
            fonts.load_font(font);
        }
    }
    let mut renderer =
        Renderer::new(Font::with_name("Noto Sans CJK SC"), 14.0.into(), Some("tiny-skia"))
            .await
            .unwrap();
    for locale in [
        DesktopLocale::SimplifiedChinese,
        DesktopLocale::TraditionalChinese,
        DesktopLocale::English,
    ] {
        let i18n = I18n::new(locale);
        for theme in [Theme::Light, Theme::Dark] {
            for handoff in [
                StepHandoff::KeepAssignee,
                StepHandoff::Member { member_id: "alice".into() },
                StepHandoff::Agent {
                    agent_id: "worker".into(),
                    instructions: "Review".into(),
                    completion_status_key: None,
                    return_status_key: None,
                },
            ] {
                let mut draft = WorkflowDraft::empty();
                let mut status = WorkflowDraftStatus::new("work", "Work", StatusRole::Custom);
                status.handoff = handoff.clone();
                draft.statuses.push(status);
                let mut scene =
                    EditorScene::new(build_draft_editor(i18n, &draft), &mut renderer, 760);
                assert!(scene.text_bounds(&renderer, i18n.__("接手方式")).is_some());
                assert!(scene.text_bounds(&renderer, i18n.__("下一步")).is_some());
                assert!(scene.text_bounds(&renderer, i18n.__("状态行为")).is_none());
                assert!(scene.text_bounds(&renderer, i18n.__("添加自定义结果")).is_none());
                assert_eq!(
                    scene.text_bounds(&renderer, i18n.__("工作说明")).is_some(),
                    matches!(handoff, StepHandoff::Agent { .. })
                );
                assert!(!scene.capture(&mut renderer, &theme).is_empty());
            }
        }
    }
}

#[tokio::test]
async fn step_form_keyboard_edits_handoff_identity_targets_and_name() {
    let mut renderer = Renderer::new(Font::DEFAULT, 14.0.into(), Some("tiny-skia")).await.unwrap();
    let i18n = I18n::new(DesktopLocale::English);
    let mut draft = WorkflowDraft::empty();
    let mut status = WorkflowDraftStatus::new("work", "", StatusRole::Custom);
    status.handoff = HandoffKind::Agent.empty();
    draft.statuses = vec![status, WorkflowDraftStatus::new("done", "Done", StatusRole::Done)];
    draft.identities =
        vec![HandoffIdentity { id: "worker".into(), label: "Worker".into(), is_agent: true }];
    for width in [456, 760] {
        for theme in [Theme::Light, Theme::Dark] {
            let mut scene = EditorScene::new(
                build_status_row(i18n, 0, &draft.statuses[0], &draft),
                &mut renderer,
                width,
            );
            let mode = scene.bounds(&[0, 2, 1]);
            assert!(mode.x + mode.width <= width as f32);
            scene.open_menu(&mut renderer, &theme, mode, "handoff");
            assert!(scene.key(&mut renderer, Key::Named(Named::End), None).is_empty());
            assert!(matches!(
                scene.key(&mut renderer, Key::Named(Named::Enter), None).as_slice(),
                [Message::WorkflowManager(WorkflowManagerMessage::DraftStatusHandoffChanged {
                    index: 0,
                    handoff: StepHandoff::Member { .. }
                })]
            ));
            let identity = scene.bounds(&[0, 4, 1]);
            scene.open_menu(&mut renderer, &theme, identity, "agent");
            assert!(scene.key(&mut renderer, Key::Named(Named::Home), None).is_empty());
            assert!(matches!(scene.key(&mut renderer, Key::Named(Named::Enter), None).as_slice(),
                [Message::WorkflowManager(WorkflowManagerMessage::DraftStatusHandoffChanged { index: 0, handoff: StepHandoff::Agent { agent_id, .. } })] if agent_id == "worker"));
            let completion = scene.bounds(&[0, 6, 1]);
            scene.open_menu(&mut renderer, &theme, completion, "completion");
            assert!(scene.key(&mut renderer, Key::Named(Named::End), None).is_empty());
            assert!(matches!(scene.key(&mut renderer, Key::Named(Named::Enter), None).as_slice(),
                [Message::WorkflowManager(WorkflowManagerMessage::DraftStatusHandoffChanged { index: 0, handoff: StepHandoff::Agent { completion_status_key: Some(key), .. } })] if key == "done"));
            let label = scene.bounds(&[0, 0, 1]);
            scene.ui.operate(&renderer, &mut FocusControl(label));
            assert!(
                matches!(scene.key(&mut renderer, Key::Character("E".into()), Some("E")).as_slice(),
                [Message::WorkflowManager(WorkflowManagerMessage::DraftStatusLabelChanged { index: 0, label })] if label == "E")
            );
        }
    }
}

#[test]
fn step_updates_keep_references_and_terminal_handoffs_consistent() {
    let mut registry = TaskWorkflowRegistry::new();
    let mut state = WorkflowManagerState::from_registry(&registry);
    let mut draft = WorkflowDraft::empty();
    draft.initial_status_key = "triage".into();
    draft.statuses = vec![
        WorkflowDraftStatus::new("triage", "Triage", StatusRole::Backlog),
        WorkflowDraftStatus::new("work", "Work", StatusRole::Custom),
    ];
    draft.statuses[1].handoff = StepHandoff::Agent {
        agent_id: "worker".into(),
        instructions: "Review".into(),
        completion_status_key: Some("triage".into()),
        return_status_key: Some("triage".into()),
    };
    state.draft = Some(draft);
    update(
        &mut state,
        &mut registry,
        WorkflowManagerMessage::DraftStatusKeyChanged { index: 0, key: "ready".into() },
    );
    let draft = state.draft.as_ref().unwrap();
    assert_eq!(draft.initial_status_key, "ready");
    assert!(
        matches!(&draft.statuses[1].handoff, StepHandoff::Agent { completion_status_key: Some(done), return_status_key: Some(back), .. } if done == "ready" && back == "ready")
    );
    update(
        &mut state,
        &mut registry,
        WorkflowManagerMessage::DraftStatusCategoryChanged {
            index: 1,
            category: StatusCategory::Done,
        },
    );
    assert_eq!(state.draft.as_ref().unwrap().statuses[1].handoff, StepHandoff::KeepAssignee);
}

#[tokio::test]
async fn step_member_picker_uses_only_supplied_login_identity() {
    let mut renderer = Renderer::new(Font::DEFAULT, 14.0.into(), Some("tiny-skia")).await.unwrap();
    let i18n = I18n::new(DesktopLocale::English);
    let mut draft = WorkflowDraft::empty();
    let mut status = WorkflowDraftStatus::new("triage", "Triage", StatusRole::Backlog);
    status.handoff = StepHandoff::Member { member_id: "unavailable".into() };
    draft.statuses.push(status);
    draft.identities = vec![
        HandoffIdentity { id: "worker".into(), label: "Worker".into(), is_agent: true },
        HandoffIdentity { id: "signed_in".into(), label: "Signed in user".into(), is_agent: false },
    ];
    let mut scene =
        EditorScene::new(build_status_row(i18n, 0, &draft.statuses[0], &draft), &mut renderer, 456);
    let member = scene.bounds(&[0, 4, 1]);
    scene.open_menu(&mut renderer, &Theme::Dark, member, "member");
    assert!(scene.key(&mut renderer, Key::Named(Named::End), None).is_empty());
    assert!(matches!(scene.key(&mut renderer, Key::Named(Named::Enter), None).as_slice(),
        [Message::WorkflowManager(WorkflowManagerMessage::DraftStatusHandoffChanged { index: 0, handoff: StepHandoff::Member { member_id } })] if member_id == "signed_in"));
}

#[test]
fn step_library_keeps_four_categories_and_unused_statuses() {
    let mut draft = WorkflowDraft::empty();
    draft.id = "catalog".into();
    draft.label = "Catalog".into();
    draft.statuses = vec![
        WorkflowDraftStatus::new("new", "New", StatusRole::Backlog),
        WorkflowDraftStatus::new("active", "Active", StatusRole::Custom),
        WorkflowDraftStatus::new("done", "Done", StatusRole::Done),
        WorkflowDraftStatus::new("closed", "Closed", StatusRole::Archived),
    ];
    draft.statuses[3].in_workflow = false;
    let workflow = owned_workflow_from_draft(&draft).unwrap();
    assert_eq!(workflow.steps.len(), 3);
    let workflow = desktop_task_model::TaskWorkflow::from(workflow);
    let groups = super::status_catalog(&workflow);
    assert!(groups.iter().all(|(_, rows)| rows.len() == 1));
    let reopened = WorkflowDraft::from_workflow(workflow);
    assert_eq!(reopened.statuses.len(), 4);
    assert!(!reopened.statuses[3].in_workflow);
}
