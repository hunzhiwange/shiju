use super::*;
use crossterm::event::KeyCode;
#[test]
// 数据驱动的会话项必须保留通用列表的 Esc 取消语义。
fn resume_picker_is_data_driven_and_cancelable() {
    let mut picker = ResumePicker::new(vec![V3SelectionItem::new("thread-1", "Fix parser")]);
    assert_eq!(picker.handle_key_event(KeyEvent::from(KeyCode::Esc)), V3Action::Cancel);
}
