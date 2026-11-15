#[test]
fn history_labels_truncate_without_splitting_unicode() {
    assert_eq!(super::repository::ellipsize("short", 10), "short");
    assert_eq!(super::repository::ellipsize("中文提交标题很长", 9), "中文提交…");
}
