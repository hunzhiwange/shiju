//! 验证权限弹窗标题与参数摘要的安全提取及回退规则。
//!
//! 元数据夹具包含字符串、对象、空值与异常类型，确保标题摘要不会信任任意载荷。

use crate::modals::{permission_modal, question_modal};
use iced::{Element, Length};
type Message = ();
#[derive(Default)]
struct Requests {
    permission_modal_request: Option<gateway_interaction::PendingPermissionRequestDto>,
    permission_modal_requests: Vec<gateway_interaction::PendingPermissionRequestDto>,
    question_modal_request: Option<question_types::Request>,
}
fn with_permission_modal<'a>(input: &Requests, root: Element<'a, Message>) -> Element<'a, Message> {
    permission_modal::with_permission_modal(
        input.permission_modal_request.as_ref(),
        &input.permission_modal_requests,
        config_ui::DesktopLocale::SimplifiedChinese,
        None,
        |_| (),
        root,
    )
}
fn with_question_modal<'a>(input: &Requests, root: Element<'a, Message>) -> Element<'a, Message> {
    question_modal::with_question_modal(
        input.question_modal_request.as_ref(),
        &[],
        &[],
        config_ui::DesktopLocale::SimplifiedChinese,
        None,
        |_| (),
        root,
    )
}
use serde_json::{Map, json};

fn request(id: &str) -> gateway_interaction::PendingPermissionRequestDto {
    gateway_interaction::PendingPermissionRequestDto {
        id: id.to_string(),
        session_id: "session-1".to_string(),
        permission: "shell".to_string(),
        patterns: Vec::new(),
        metadata: Map::new(),
        always: Vec::new(),
        tool: None,
    }
}

#[tokio::test]
async fn permission_and_question_modals_preserve_scroll_position() {
    use iced::advanced::renderer::Headless;
    use iced::advanced::widget::{Operation, operation};
    use iced::widget::{Id, Space, scrollable};
    use iced::{Font, Pixels, Rectangle, Size, Vector};
    use iced_runtime::user_interface::{Cache, UserInterface};

    struct ScrollPosition<'a> {
        id: &'a Id,
        y: Option<f32>,
    }

    impl Operation for ScrollPosition<'_> {
        fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
            operate(self);
        }

        fn scrollable(
            &mut self,
            id: Option<&Id>,
            _: Rectangle,
            _: Rectangle,
            translation: Vector,
            _: &mut dyn operation::Scrollable,
        ) {
            if id == Some(self.id) {
                self.y = Some(translation.y);
            }
        }
    }

    let mut app = Requests::default();
    let id = Id::unique();
    let view = |app: &Requests| {
        let base = scrollable(Space::new().width(Length::Fill).height(1600)).id(id.clone()).into();
        with_permission_modal(app, with_question_modal(app, base))
    };
    let bounds = Size::new(1000.0, 700.0);
    let mut renderer =
        <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
            .await
            .expect("headless renderer");
    let mut ui = UserInterface::build(view(&app), bounds, Cache::new(), &mut renderer);
    ui.operate(
        &mut renderer,
        &mut operation::scrollable::scroll_to(
            id.clone(),
            operation::scrollable::AbsoluteOffset { x: None, y: Some(200.0) },
        ),
    );
    let mut position = ScrollPosition { id: &id, y: None };
    ui.operate(&mut renderer, &mut position);
    assert_eq!(position.y, Some(200.0));

    let question = question_types::Request {
        id: "question-1".into(),
        session_id: "session-1".into(),
        questions: vec![question_types::Info {
            header: "Confirm choice".into(),
            question: "Which option should I use?".into(),
            options: Vec::new(),
            multiple: None,
            custom: Some(true),
        }],
        tool: None,
        mcp_form: None,
    };
    for (permission, question, expected_y) in [
        (Some(request("perm-1")), None, None),
        (None, None, Some(200.0)),
        (None, Some(question), None),
        (None, None, Some(200.0)),
    ] {
        app.permission_modal_request = permission;
        app.question_modal_request = question;
        ui = UserInterface::build(view(&app), bounds, ui.into_cache(), &mut renderer);
        position.y = None;
        ui.operate(&mut renderer, &mut position);
        assert_eq!(position.y, expected_y);
    }
}

#[test]
fn with_permission_modal_builds_single_request_dialog_when_queue_is_empty() {
    let mut app = Requests::default();
    app.permission_modal_request = Some(request("perm-1"));

    let element: iced::Element<'_, Message> = with_permission_modal(&app, root());

    std::hint::black_box(element);
}

#[test]
fn with_permission_modal_builds_single_and_multi_request_dialogs() {
    let mut app = Requests::default();
    let mut first = request("perm-1");
    first.patterns = (0..10).map(|idx| format!("pattern-{idx}")).collect();
    first.always = vec!["pattern-0".to_string(), "pattern-1".to_string()];
    first.metadata.insert("reason".to_string(), json!("Need command access"));
    first.metadata.insert("arguments".to_string(), json!({"cmd": "pwd"}));
    first.metadata.insert("requested_by".to_string(), json!("agent"));
    first.metadata.insert("requested_channel".to_string(), json!("chat"));
    first.metadata.insert("expires_at".to_string(), json!("2030-01-01T00:00:00Z"));
    first.tool = Some(gateway_interaction::PendingPermissionToolDto {
        message_id: "msg-1".to_string(),
        call_id: "call-1".to_string(),
    });
    let second = request("perm-2");
    app.permission_modal_request = Some(first.clone());
    app.permission_modal_requests = vec![first, second];

    let _element: iced::Element<'_, Message> = with_permission_modal(&app, root());
}

#[test]
fn with_permission_modal_builds_navigation_for_middle_and_last_requests() {
    let requests = vec![request("perm-1"), request("perm-2"), request("perm-3")];
    let mut app = Requests::default();
    app.permission_modal_requests = requests.clone();

    app.permission_modal_request = Some(requests[1].clone());
    let middle: iced::Element<'_, Message> = with_permission_modal(&app, root());
    std::hint::black_box(middle);

    app.permission_modal_request = Some(requests[2].clone());
    let last: iced::Element<'_, Message> = with_permission_modal(&app, root());
    std::hint::black_box(last);
}

#[test]
fn with_permission_modal_falls_back_to_first_position_for_request_outside_queue() {
    let mut app = Requests::default();
    app.permission_modal_request = Some(request("perm-current"));
    app.permission_modal_requests = vec![request("perm-1"), request("perm-2")];

    let element: iced::Element<'_, Message> = with_permission_modal(&app, root());

    std::hint::black_box(element);
}

#[test]
fn with_permission_modal_limits_long_pending_request_list() {
    let requests = (0..7).map(|idx| request(&format!("perm-{idx}"))).collect::<Vec<_>>();
    let mut app = Requests::default();
    app.permission_modal_request = Some(requests[0].clone());
    app.permission_modal_requests = requests;

    let element: iced::Element<'_, Message> = with_permission_modal(&app, root());

    std::hint::black_box(element);
}

mod feedback_dialogs;

mod session_dialogs;

mod todo_status;

#[path = "host_module_38_tests/question_modal_tests.rs"]
mod question_modal_tests;
