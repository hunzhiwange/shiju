//! 验证聊天工具模块按约定重新导出文本辅助函数。
//! 通过公共入口调用可防止实现仍存在但模块门面遗漏接线。

use super::{truncate_chars, truncate_lines_middle};

#[test]
fn utils_reexports_text_helpers() {
    assert_eq!(truncate_chars("hello", 10), "hello");
    assert_eq!(truncate_lines_middle("a\nb", 3, 10), "a\nb");
}
