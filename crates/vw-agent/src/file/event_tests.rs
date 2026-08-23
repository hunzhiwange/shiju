// 覆盖文件编辑事件名称的稳定公共契约。

use super::EDITED;

#[test]
fn edited_event_type_is_stable() {
    assert_eq!(EDITED.r#type, "file.edited");
}
