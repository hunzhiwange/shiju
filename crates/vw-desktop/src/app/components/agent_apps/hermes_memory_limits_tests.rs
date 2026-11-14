use super::hermes_memory_limits::budget_text;

#[test]
fn hermes_budget_matches_utf16_count() {
    assert_eq!(budget_text("中文😀", 5), "剩余 1 / 5（UTF-16 单元）");
    assert_eq!(budget_text("中文😀", 4), "剩余 0 / 4（UTF-16 单元）");
    assert_eq!(budget_text("中文😀", 3), "超出预算 1 / 3（UTF-16 单元）；原文保留");
}

#[test]
fn hermes_budget_matches_utf16_count_zero_and_combining_marks() {
    assert_eq!(budget_text("", 0), "剩余 0 / 0（UTF-16 单元）");
    assert_eq!(budget_text("e\u{301}", 0), "超出预算 2 / 0（UTF-16 单元）；原文保留");
    let content = "😀".repeat(2201);
    assert_eq!(budget_text(&content, 2200), "超出预算 2202 / 2200（UTF-16 单元）；原文保留");
    assert_eq!(content.encode_utf16().count(), 4402);
}
