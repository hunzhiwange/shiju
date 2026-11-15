//! 通道测试工具函数回归测试。
//!
//! 本模块验证通道日志等辅助逻辑在边界输入下保持安全行为。当前重点是
//! UTF-8 截断：日志裁剪不能把多字节字符切到非法边界，否则后续渲染或
//! 序列化可能出现 panic。

use super::truncate_with_ellipsis;

#[test]
fn channel_log_truncation_is_utf8_safe_for_multibyte_text() {
    let msg = "Hello from VibeWindow 🌍. Current status is healthy, and café-style UTF-8 text stays safe in logs.";

    // 使用 catch_unwind 固定“不 panic”这个行为契约；这里关心的是字符边界，
    // 不是具体截断出的展示文本。
    let result =
        std::panic::catch_unwind(|| truncate_with_ellipsis(msg, 80));
    assert!(result.is_ok(), "truncate_with_ellipsis should never panic on UTF-8");

    let truncated = result.unwrap();
    assert!(truncated.is_char_boundary(truncated.len()));
}
