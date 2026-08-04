use super::*;
use iced::Theme;
use iced::advanced::widget::Tree;
use vw_config_types::ui::QuickMessageConfig;

fn app() -> App {
    App::new().0
}

fn config(text: &str, group: QuickMessageGroup, kind: QuickMessageKind) -> QuickMessageConfig {
    QuickMessageConfig { text: text.to_string(), group, kind }
}

fn view_tree(app: &App) -> Tree {
    let element = view(app);
    Tree::new(element.as_widget())
}

#[test]
fn quick_message_settings_view_renders_all_item_types_and_controls() {
    let mut app = app();
    app.quick_messages = vec![
        config("继续推进", QuickMessageGroup::General, QuickMessageKind::Message),
        config("创建新会话", QuickMessageGroup::Git, QuickMessageKind::CreateNewSession),
    ];

    let tree = view_tree(&app);
    assert_eq!(tree.children.len(), 2);
    let panel = &tree.children[1];
    assert_eq!(panel.children.len(), 2);
    assert_eq!(panel.children[0].children.len(), 2);
    assert!(!panel.children[1].children.is_empty());
}

#[test]
fn quick_message_settings_view_supports_empty_and_default_lists() {
    let mut app = app();
    app.quick_messages.clear();
    let empty = view_tree(&app);
    assert_eq!(empty.children[1].children[0].children.len(), 1);

    app.quick_messages = vw_config_types::ui::AppSystemSettingsConfig::default().quick_messages;
    let defaults = view_tree(&app);
    assert_eq!(defaults.children[1].children[0].children.len(), 33);
}

#[test]
fn quick_message_settings_view_messages_preserve_indices_and_variants() {
    let Message::Settings(SettingsMessage::QuickMessages(QuickMessagesMessage::Add)) =
        add_message()
    else {
        panic!("unexpected add message");
    };
    let Message::Settings(SettingsMessage::QuickMessages(QuickMessagesMessage::Save)) =
        save_message()
    else {
        panic!("unexpected save message");
    };
    let Message::Settings(SettingsMessage::QuickMessages(QuickMessagesMessage::Remove(3))) =
        remove_message(3)
    else {
        panic!("unexpected remove message");
    };
    let Message::Settings(SettingsMessage::QuickMessages(QuickMessagesMessage::MoveUp(4))) =
        move_up_message(4)
    else {
        panic!("unexpected move-up message");
    };
    let Message::Settings(SettingsMessage::QuickMessages(QuickMessagesMessage::MoveDown(5))) =
        move_down_message(5)
    else {
        panic!("unexpected move-down message");
    };
}

#[test]
fn quick_message_settings_view_item_style_adapts_to_dark_theme() {
    let light = item_style(&Theme::Light);
    let dark = item_style(&Theme::Dark);

    assert_ne!(light.background, dark.background);
    assert_ne!(light.border.color, dark.border.color);
    assert_eq!(dark.border.width, 1.0);
    assert_eq!(dark.border.radius.top_left, 12.0);
}
