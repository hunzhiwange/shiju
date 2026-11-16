#[test]
fn diff_view_layout_constants_are_stable() {
    assert_eq!(crate::git_panel::diff_view::DIFF_MARKER_WIDTH, 8.0);
    assert_eq!(crate::git_panel::diff_view::DIFF_LINE_NUMBER_WIDTH, 46.0);
    assert_eq!(crate::git_panel::diff_view::DIFF_SPLIT_DIVIDER_WIDTH, 8.0);
}
