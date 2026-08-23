//! 验证系统设置公共格式化工具通过模块入口保持可用。
//! 该测试守住重导出契约，具体输入边界由 `utils_tests` 覆盖。

use super::{bool_support_label, format_context_limit, url_encode};

#[test]
fn common_exports_keep_formatting_helpers_available() {
    assert_eq!(bool_support_label(true), "支持");
    assert_eq!(format_context_limit(8192), "8K");
    assert_eq!(url_encode("a b"), "a%20b");
}
