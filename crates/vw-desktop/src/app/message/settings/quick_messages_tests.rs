use super::*;
use crate::app::state::ToastKind;

fn app() -> App {
    App::new().0
}

fn quick_message(text: &str) -> QuickMessageConfig {
    QuickMessageConfig {
        text: text.to_string(),
        group: QuickMessageGroup::General,
        kind: QuickMessageKind::Message,
    }
}

#[test]
fn quick_message_settings_mutations_are_ordered_and_bounds_safe() {
    let mut app = app();
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

#[test]
fn quick_message_settings_save_validates_content_and_reports_result() {
    let mut app = app();
    app.quick_messages = vec![quick_message("  ")];

    let _ = update(&mut app, SettingsMessage::QuickMessages(QuickMessagesMessage::Save));
    let warning = app.active_toast.as_ref().expect("blank content should show a toast");
    assert_eq!(warning.kind, ToastKind::Warning);
    assert!(warning.message.contains("不能为空"));

    let _ = update(&mut app, SettingsMessage::QuickMessages(QuickMessagesMessage::Saved(Ok(()))));
    assert_eq!(app.active_toast.as_ref().map(|toast| toast.kind), Some(ToastKind::Success));

    let _ = update(
        &mut app,
        SettingsMessage::QuickMessages(QuickMessagesMessage::Saved(Err(
            "gateway unavailable".to_string()
        ))),
    );
    let error = app.active_toast.as_ref().expect("save error should show a toast");
    assert_eq!(error.kind, ToastKind::Error);
    assert!(error.message.contains("gateway unavailable"));
}
