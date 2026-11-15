use super::*;
struct Fixture { quick_messages: Vec<QuickMessageConfig> }
enum SettingsMessage { QuickMessages(QuickMessagesMessage) }
fn update(app: &mut Fixture, message: SettingsMessage) -> Effect {
    let SettingsMessage::QuickMessages(message) = message;
    super::update(&mut app.quick_messages, message)
}
fn quick_message(text: &str) -> QuickMessageConfig {
    QuickMessageConfig { text: text.into(), group: QuickMessageGroup::General, kind: QuickMessageKind::Message }
}
// 行为契约：使用代表性场景固定该分支的可观察结果。
// 断言聚焦输入、状态与返回任务，不复述内部控制流程。
#[test]
fn quick_message_settings_mutations_are_ordered_and_bounds_safe() {
    let mut app = Fixture { quick_messages: Vec::new() };
    app.quick_messages = vec![quick_message("第一条"), quick_message("第二条")];

    let _ = update(&mut app, SettingsMessage::QuickMessages(QuickMessagesMessage::Add));
    assert_eq!(app.quick_messages[2].text, "新快捷消息");
    assert_eq!(app.quick_messages[2].kind, QuickMessageKind::FillInput);

    let _ = update(
        &mut app,
        SettingsMessage::QuickMessages(QuickMessagesMessage::TextChanged(
            2,
            "创建会话".to_string(),
        )),
    );
    let _ = update(
        &mut app,
        SettingsMessage::QuickMessages(QuickMessagesMessage::GroupChanged(
            2,
            QuickMessageGroup::Git,
        )),
    );
    let _ = update(
        &mut app,
        SettingsMessage::QuickMessages(QuickMessagesMessage::KindChanged(
            2,
            QuickMessageKind::CreateNewSession,
        )),
    );
    let _ = update(&mut app, SettingsMessage::QuickMessages(QuickMessagesMessage::MoveUp(2)));
    assert_eq!(app.quick_messages[1].text, "创建会话");
    assert_eq!(app.quick_messages[1].group, QuickMessageGroup::Git);
    assert_eq!(app.quick_messages[1].kind, QuickMessageKind::CreateNewSession);

    let _ = update(&mut app, SettingsMessage::QuickMessages(QuickMessagesMessage::MoveDown(1)));
    let _ = update(&mut app, SettingsMessage::QuickMessages(QuickMessagesMessage::Remove(1)));
    assert_eq!(
        app.quick_messages.iter().map(|item| item.text.as_str()).collect::<Vec<_>>(),
        vec!["第一条", "创建会话"]
    );

    let snapshot = app.quick_messages.clone();
    for message in [
        QuickMessagesMessage::Remove(99),
        QuickMessagesMessage::TextChanged(99, "忽略".to_string()),
        QuickMessagesMessage::GroupChanged(99, QuickMessageGroup::Git),
        QuickMessagesMessage::KindChanged(99, QuickMessageKind::CreateNewSession),
        QuickMessagesMessage::MoveUp(99),
        QuickMessagesMessage::MoveDown(99),
        QuickMessagesMessage::MoveUp(0),
        QuickMessagesMessage::MoveDown(app.quick_messages.len() - 1),
    ] {
        let _ = update(&mut app, SettingsMessage::QuickMessages(message));
    }
    assert_eq!(app.quick_messages, snapshot);
}
