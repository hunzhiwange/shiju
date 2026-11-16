//! 问题弹窗行为已迁入 desktop-chat-view；保留原测试入口契约。

#[test]
fn question_modal_tests_module_is_wired() {
    assert!(module_path!().ends_with("question_modal_tests"));
}
