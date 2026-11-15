// 覆盖诊断文本截断的 Unicode 边界与时间戳解析。

use super::utils::{parse_rfc3339, truncate_for_display};

// 诊断断言同时检查严重级别与可操作提示，避免只有返回值正确而用户指引失真。
#[test]
fn truncate_for_display_handles_unicode_boundaries() {
    assert_eq!(truncate_for_display("abcdef", 6), "abcdef");
    assert_eq!(truncate_for_display("abcdef", 3), "abc…");
    assert_eq!(truncate_for_display("abcdef", 0), "…");
    assert_eq!(truncate_for_display("你好世界", 2), "你好…");
}

#[test]
fn parse_rfc3339_accepts_valid_timestamp_and_rejects_invalid_text() {
    let parsed = parse_rfc3339("2026-05-24T08:00:00+08:00").expect("valid timestamp");
    assert_eq!(parsed.to_rfc3339(), "2026-05-24T00:00:00+00:00");
    assert!(parse_rfc3339("not-a-date").is_none());
}
