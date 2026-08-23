// 覆盖文件编辑事件名称的稳定公共契约。

use super::EDITED;

// 临时目录夹具同时覆盖正常路径和逃逸路径，确保安全判断不依赖当前工作区。
#[test]
fn edited_event_type_is_stable() {
    assert_eq!(EDITED.r#type, "file.edited");
}
