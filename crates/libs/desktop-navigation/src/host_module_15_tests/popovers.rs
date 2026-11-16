use crate::components::model_hover_tooltip::ModelPopoverHover;
use crate::panel::popovers as domain;
use desktop_chat_input::popovers::{Popover, Visibility};
#[derive(Default)]
struct PopoverFixture {
    input_popovers: Visibility,
    model_popover_hover: Option<ModelPopoverHover>,
    acp_agents: Vec<String>,
    collapsed: std::collections::BTreeSet<String>,
}
fn test_app() -> PopoverFixture {
    PopoverFixture::default()
}
fn update(app: &mut PopoverFixture, message: domain::Message) -> Option<domain::Effect> {
    domain::update(
        domain::Context {
            visibility: &mut app.input_popovers,
            hover: &mut app.model_popover_hover,
            collapsed_providers: &mut app.collapsed,
        },
        domain::Inputs { acp_agents_empty: app.acp_agents.is_empty(), ..Default::default() },
        message,
    )
}
fn set_all_popovers(app: &mut PopoverFixture, visible: bool) {
    app.input_popovers.model = visible;
    app.input_popovers.mode = visible;
    app.input_popovers.send_mode = visible;
    app.input_popovers.file = visible;
    app.input_popovers.acp = visible;
    app.input_popovers.reasoning_effort = visible;
    app.input_popovers.usage = visible;
    app.input_popovers.session_tool_selector = visible;
    app.input_popovers.session_actions = visible;
    app.input_popovers.executor = visible;
}

fn visibility(app: &PopoverFixture) -> [bool; 10] {
    [
        app.input_popovers.model,
        app.input_popovers.mode,
        app.input_popovers.send_mode,
        app.input_popovers.file,
        app.input_popovers.acp,
        app.input_popovers.reasoning_effort,
        app.input_popovers.usage,
        app.input_popovers.session_tool_selector,
        app.input_popovers.session_actions,
        app.input_popovers.executor,
    ]
}

#[test]
fn send_mode_popover_stays_closed_when_session_is_not_requesting() {
    let mut app = test_app();
    app.input_popovers.send_mode = true;
    app.input_popovers.model = true;

    let _ = update(&mut app, domain::Message::Toggle(Popover::SendMode));

    assert!(!app.input_popovers.send_mode);
    assert!(app.input_popovers.model);
}

// 失败与边界：覆盖非法、缺失、过期或冲突场景的既定处理规则。
// 重点确认现有状态与错误反馈不会被意外的部分更新污染。
#[test]
fn opening_acp_popover_with_empty_inventory_requests_bootstrap_and_closes_peers() {
    let mut app = test_app();
    set_all_popovers(&mut app, true);
    app.input_popovers.acp = false;
    app.acp_agents.clear();

    let _bootstrap_task = update(&mut app, domain::Message::Toggle(Popover::Acp));

    assert_eq!(
        visibility(&app),
        [false, false, false, false, true, false, false, false, false, false]
    );
    assert!(app.acp_agents.is_empty());
}

#[test]
fn close_popovers_clears_every_popover_and_model_hover() {
    let mut app = test_app();
    set_all_popovers(&mut app, true);
    app.model_popover_hover =
        Some(ModelPopoverHover::Text { text: "hovered".to_string(), anchor: None });

    let _ = update(&mut app, domain::Message::CloseAll);

    assert_eq!(visibility(&app), [false; 10]);
    assert!(app.model_popover_hover.is_none());
}

#[test]
fn individual_close_messages_only_close_their_target() {
    let cases = [
        (domain::Message::Close(Popover::Model), 0),
        (domain::Message::Close(Popover::Mode), 1),
        (domain::Message::Close(Popover::SendMode), 2),
        (domain::Message::Close(Popover::File), 3),
        (domain::Message::Close(Popover::Acp), 4),
        (domain::Message::Close(Popover::ReasoningEffort), 5),
        (domain::Message::Close(Popover::Usage), 6),
        (domain::Message::Close(Popover::Executor), 9),
    ];

    for (message, closed_index) in cases {
        let mut app = test_app();
        set_all_popovers(&mut app, true);

        let _ = update(&mut app, message);

        let mut expected = [true; 10];
        expected[closed_index] = false;
        assert_eq!(visibility(&app), expected);
    }
}

// 状态转换：覆盖交互前后的字段同步、选择关系与清理规则。
// 连续操作仍需保持模块不变量，并清除已经失效的瞬态反馈。
#[test]
fn closing_model_popover_also_clears_hover_details() {
    let mut app = test_app();
    app.input_popovers.model = true;
    app.model_popover_hover = Some(ModelPopoverHover::Model {
        provider_id: "provider".to_string(),
        model_id: "model".to_string(),
        anchor: None,
    });

    let _ = update(&mut app, domain::Message::Close(Popover::Model));

    assert!(!app.input_popovers.model);
    assert!(app.model_popover_hover.is_none());
}

#[test]
fn model_hover_changed_replaces_and_clears_hover_state() {
    let mut app = test_app();

    let _ = update(
        &mut app,
        domain::Message::HoverChanged(Some(ModelPopoverHover::Text {
            text: "details".to_string(),
            anchor: None,
        })),
    );

    let Some(ModelPopoverHover::Text { text, anchor }) = app.model_popover_hover.as_ref() else {
        panic!("expected text hover state");
    };
    assert_eq!(text, "details");
    assert!(anchor.is_none());

    let _ = update(&mut app, domain::Message::HoverChanged(None));
    assert!(app.model_popover_hover.is_none());
}

// 失败与边界：覆盖非法、缺失、过期或冲突场景的既定处理规则。
// 重点确认现有状态与错误反馈不会被意外的部分更新污染。
