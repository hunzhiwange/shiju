use super::*;

// 验证 `local_history_recalls_unicode_drafts_without_duplicate_submissions` 场景的触发条件与预期结果。
#[test]
fn local_history_recalls_unicode_drafts_without_duplicate_submissions() {
    let mut history = ChatComposerHistory::new();
    let unicode = HistoryEntry::new("你好 👩‍💻\n组合 e\u{301}");
    history.record_local_submission(unicode.clone());
    history.record_local_submission(unicode.clone());

    assert_eq!(history.entries(), std::slice::from_ref(&unicode));
    assert_eq!(history.navigate_up("当前草稿"), Some(unicode));
    assert_eq!(history.navigate_down(), Some(HistoryEntry::new("当前草稿")));
}

// 验证 `reverse_search_walks_local_history_and_reports_boundaries` 场景的触发条件与预期结果。
#[test]
fn reverse_search_walks_local_history_and_reports_boundaries() {
    let mut history = ChatComposerHistory::new();
    history.record_local_submission(HistoryEntry::new("first"));
    history.record_local_submission(HistoryEntry::new("second unicode 你好"));
    history.record_local_submission(HistoryEntry::new("third"));

    assert_eq!(
        history.search("unicode", HistorySearchDirection::Older, true),
        HistorySearchResult::Found(HistoryEntry::new("second unicode 你好"))
    );
    assert_eq!(
        history.search("unicode", HistorySearchDirection::Older, false),
        HistorySearchResult::AtBoundary
    );
}
