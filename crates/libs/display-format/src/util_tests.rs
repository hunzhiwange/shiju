//! 通用展示格式化工具的边界测试。
//!
//! 覆盖时长、数量、复数模板及 Unicode 字符边界安全的首尾截断。

#[test]
fn format_duration_covers_boundaries() {
    assert_eq!(super::format_duration(-1), "");
    assert_eq!(super::format_duration(0), "");
    assert_eq!(super::format_duration(1), "1s");
    assert_eq!(super::format_duration(59), "59s");
    assert_eq!(super::format_duration(60), "1m");
    assert_eq!(super::format_duration(61), "1m 1s");
    assert_eq!(super::format_duration(3599), "59m 59s");
    assert_eq!(super::format_duration(3600), "1h");
    assert_eq!(super::format_duration(3660), "1h 1m");
    assert_eq!(super::format_duration(86399), "23h 59m");
    assert_eq!(super::format_duration(86400), "~1 day");
    assert_eq!(super::format_duration(172800), "~2 days");
    assert_eq!(super::format_duration(604799), "~6 days");
    assert_eq!(super::format_duration(604800), "~1 week");
    assert_eq!(super::format_duration(1209600), "~2 weeks");
}

#[test]
fn truncate_respects_character_boundaries() {
    assert_eq!(super::truncate("", 0), "");
    assert_eq!(super::truncate("abc", 3), "abc");
    assert_eq!(super::truncate("abc", 4), "abc");
    assert_eq!(super::truncate("workflow", 4), "wor…");
    assert_eq!(super::truncate("节点编辑", 3), "节点…");
    assert_eq!(super::truncate("节点", 2), "节点");
    assert_eq!(super::truncate("abc", 0), "");
    assert_eq!(super::truncate("abc", 1), "…");
}

#[test]
fn titlecase_handles_word_starts_and_special_input() {
    assert_eq!(super::titlecase(""), "");
    assert_eq!(super::titlecase("hello workflow-agent"), "Hello Workflow-Agent");
    assert_eq!(super::titlecase(" already\tspaced"), " Already\tSpaced");
    assert_eq!(super::titlecase("v2 api"), "V2 Api");
    assert_eq!(super::titlecase("ßeta"), "SSeta");
    assert_eq!(super::titlecase("中文-title"), "中文-Title");
}

#[test]
fn truncate_middle_preserves_edges() {
    assert_eq!(super::truncate_middle("", 0), "");
    assert_eq!(super::truncate_middle("short", 5), "short");
    assert_eq!(super::truncate_middle("short", 6), "short");
    assert_eq!(super::truncate_middle("abcdefghij", 5), "ab…ij");
    assert_eq!(super::truncate_middle("abcdefghi", 4), "ab…i");
    assert_eq!(super::truncate_middle("abcdefghi", 2), "a…");
    assert_eq!(super::truncate_middle("abcdefghi", 1), "…");
    assert_eq!(super::truncate_middle("abcdefghi", 0), "…");
    assert_eq!(super::truncate_middle("节点编辑器", 4), "节点…器");
}

#[test]
fn pluralize_uses_count_and_replaces_templates() {
    assert_eq!(super::pluralize(1, "{} item", "{} items"), "1 item");
    assert_eq!(super::pluralize(0, "{} item", "{} items"), "0 items");
    assert_eq!(super::pluralize(2, "{} item", "{} items"), "2 items");
    assert_eq!(super::pluralize(-1, "{} item", "{} items"), "-1 items");
    assert_eq!(super::pluralize(3, "one", "many"), "many");
    assert_eq!(super::pluralize(4, "{} of {}", "{} of {}"), "4 of 4");
}

#[test]
fn number_formats_plain_scaled_negative_and_special_values() {
    assert_eq!(super::number(0.0), "0");
    assert_eq!(super::number(999.0), "999");
    assert_eq!(super::number(999.5), "999.5");
    assert_eq!(super::number(1000.0), "1.0K");
    assert_eq!(super::number(999_999.0), "1000.0K");
    assert_eq!(super::number(1_000_000.0), "1.0M");
    assert_eq!(super::number(42.0), "42");
    assert_eq!(super::number(42.5), "42.5");
    assert_eq!(super::number(1500.0), "1.5K");
    assert_eq!(super::number(2_000_000.0), "2.0M");
    assert_eq!(super::number(-1500.0), "-1500");
    assert_eq!(super::number(-42.5), "-42.5");
    assert_eq!(super::number(f64::NAN), "NaN");
    assert_eq!(super::number(f64::INFINITY), "infM");
    assert_eq!(super::number(f64::NEG_INFINITY), "-inf");
}

#[test]
fn duration_covers_boundaries() {
    assert_eq!(super::duration(-1), "-1ms");
    assert_eq!(super::duration(0), "0ms");
    assert_eq!(super::duration(999), "999ms");
    assert_eq!(super::duration(1000), "1.0s");
    assert_eq!(super::duration(59_999), "60.0s");
    assert_eq!(super::duration(60_000), "1m 0s");
    assert_eq!(super::duration(65_000), "1m 5s");
    assert_eq!(super::duration(3_599_000), "59m 59s");
    assert_eq!(super::duration(3_600_000), "1h 0m");
    assert_eq!(super::duration(3_660_000), "1h 1m");
    assert_eq!(super::duration(86_399_000), "23h 59m");
    assert_eq!(super::duration(86_400_000), "1d 0h");
    assert_eq!(super::duration(90_000_000), "1d 1h");
}
