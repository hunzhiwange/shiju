use super::*;
use crossterm::event::KeyCode;
#[test]
// Enter 只返回稳定主题 ID，不在选择器内部写入配置。
fn theme_picker_confirms_without_persisting_configuration() {
    let mut picker = ThemePicker::new(vec![V3SelectionItem::new("dark", "Dark")]);
    assert_eq!(
        picker.handle_key_event(KeyEvent::from(KeyCode::Enter)),
        V3Action::Select { surface: "theme".into(), item_id: "dark".into() }
    );
}
