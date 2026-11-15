//! 从桌面主视图迁移的主题、估算及虚拟窗口断言。
use super::*;
use super::utils::FileStatus;
#[derive(Default)]
struct Fixture {
    git_focused_file: Option<String>, expanded_files_set: HashSet<String>,
    git_diff_contents_loading: HashSet<String>, git_diff_contents: HashMap<String, (String, String)>,
    git_diff_scroll_viewport_h: f32, git_diff_scroll_offset_y: f32,
}
impl Fixture {
    fn input(&self) -> FileListInput<'_> { FileListInput {
        git_focused_file: &self.git_focused_file, expanded_files_set: &self.expanded_files_set,
        git_diff_contents_loading: &self.git_diff_contents_loading, git_diff_contents: &self.git_diff_contents,
        git_diff_scroll_viewport_h: self.git_diff_scroll_viewport_h, git_diff_scroll_offset_y: self.git_diff_scroll_offset_y,
    } }
}
fn app() -> Fixture { Fixture::default() }
#[cfg(not(target_arch = "wasm32"))]
fn meta(
    path: &str,
    status: FileStatus,
    insertions: usize,
    deletions: usize,
) -> super::DiffFileMeta {
    super::DiffFileMeta {
        path: path.to_string(),
        status,
        old_oid: None,
        old_size: 0,
        new_size: 0,
        new_exists: true,
        insertions,
        deletions,
    }
}

#[cfg(target_arch = "wasm32")]
fn meta(
    path: &str,
    status: FileStatus,
    insertions: usize,
    deletions: usize,
) -> super::DiffFileMeta {
    super::DiffFileMeta {
        path: path.to_string(),
        status,
        old_size: 0,
        new_size: 0,
        new_exists: true,
        insertions,
        deletions,
    }
}









#[test]
fn diff_colors_follow_requested_theme() {
    let github = super::get_diff_colors(DiffTheme::GitHub);
    let monokai = super::get_diff_colors(DiffTheme::Monokai);

    assert_ne!(github.1, monokai.1);
    assert_ne!(github.2, monokai.2);
    assert_ne!(github.4, monokai.4);
}

#[test]
fn block_height_sums_items_and_gaps() {
    assert_eq!(super::sum_git_file_block_height(&[]), 0.0);
    assert_eq!(super::sum_git_file_block_height(&[10.0]), 10.0);
    assert_eq!(super::sum_git_file_block_height(&[10.0, 20.0, 30.0]), 80.0);
}

#[test]
fn estimate_file_card_height_covers_collapsed_loading_and_status_variants() {
    let mut app = app();
    let modified = meta("src/lib.rs", FileStatus::Modified, 2, 1);

    let collapsed = super::estimate_git_file_card_height(&app.input(), &modified);
    assert!(collapsed >= 32.0);

    app.expanded_files_set.insert("src/lib.rs".to_string());
    app.git_diff_contents_loading.insert("src/lib.rs".to_string());
    let loading = super::estimate_git_file_card_height(&app.input(), &modified);
    assert!(loading > collapsed);

    app.git_diff_contents_loading.clear();
    app.git_focused_file = Some("src/lib.rs".to_string());
    app.git_diff_contents.insert(
        "src/lib.rs".to_string(),
        ("old\nold2\nold3\n".to_string(), "new\nnew2\nnew3\nnew4\n".to_string()),
    );
    let focused_modified = super::estimate_git_file_card_height(&app.input(), &modified);
    assert!(focused_modified > loading);

    for status in [
        FileStatus::Added,
        FileStatus::Untracked,
        FileStatus::Deleted,
        FileStatus::Renamed,
        FileStatus::Unknown,
    ] {
        let file = format!("{status:?}.txt");
        app.expanded_files_set.insert(file.clone());
        app.git_diff_contents
            .insert(file.clone(), ("one\ntwo\n".to_string(), "one\ntwo\nthree\n".to_string()));
        let height = super::estimate_git_file_card_height(&app.input(), &meta(&file, status, 1, 1));
        assert!(height > collapsed, "{status:?}");
    }
}

#[test]
fn virtual_window_handles_empty_zero_viewport_clamping_and_buffer() {
    let mut app = app();

    assert_eq!(super::compute_git_file_virtual_window(&app.input(), &[]), (0, 0, 0.0, 0.0));

    let heights = vec![20.0; 60];
    assert_eq!(super::compute_git_file_virtual_window(&app.input(), &heights), (0, 60, 0.0, 0.0));

    app.git_diff_scroll_viewport_h = 50.0;
    app.git_diff_scroll_offset_y = -1.0;
    let (start, end, top, bottom) = super::compute_git_file_virtual_window(&app.input(), &heights);
    assert_eq!(start, 0);
    assert!(end > start);
    assert_eq!(top, 0.0);
    assert!(bottom > 0.0);

    app.git_diff_scroll_offset_y = 2.0;
    let (start, end, top, bottom) = super::compute_git_file_virtual_window(&app.input(), &heights);
    assert!(start < end);
    assert_eq!(end, heights.len());
    assert!(top > 0.0);
    assert_eq!(bottom, 0.0);
}

#[test]
fn theme_brightness_selects_dark_and_light_diff_palettes() {
    assert!(super::is_dark_theme(&iced::Theme::Dark));
    assert!(!super::is_dark_theme(&iced::Theme::Light));

    let dark = super::get_diff_colors(crate::DiffTheme::Monokai);
    let light = super::get_diff_colors(crate::DiffTheme::GitHub);
    assert_ne!(dark.1, light.1);
    assert_ne!(dark.2, light.2);
}

#[test]
fn file_card_height_accounts_for_focus_loading_and_missing_content() {
    let mut app = app();
    let file = meta("src/lib.rs", FileStatus::Modified, 3, 2);

    assert_eq!(
        super::estimate_git_file_card_height(&app.input(), &file),
        super::GIT_FILE_HEADER_ESTIMATED_HEIGHT
    );

    app.git_focused_file = Some(file.path.clone());
    assert_eq!(
        super::estimate_git_file_card_height(&app.input(), &file),
        super::GIT_FILE_HEADER_ESTIMATED_HEIGHT + super::GIT_FILE_FOCUSED_EXTRA_HEIGHT
    );

    app.expanded_files_set.insert(file.path.clone());
    app.git_diff_contents_loading.insert(file.path.clone());
    assert_eq!(
        super::estimate_git_file_card_height(&app.input(), &file),
        super::GIT_FILE_HEADER_ESTIMATED_HEIGHT
            + super::GIT_FILE_FOCUSED_EXTRA_HEIGHT
            + super::GIT_FILE_LOADING_PLACEHOLDER_HEIGHT
    );

    app.git_focused_file = None;
    app.git_diff_contents_loading.clear();
    assert_eq!(
        super::estimate_git_file_card_height(&app.input(), &file),
        super::GIT_FILE_HEADER_ESTIMATED_HEIGHT + super::GIT_FILE_LOADING_PLACEHOLDER_HEIGHT
    );
}

#[test]
fn file_card_height_uses_status_specific_line_estimates() {
    let cases = [
        (FileStatus::Added, "old", "one\ntwo", 5, 0, 5),
        (FileStatus::Untracked, "", "one\ntwo\nthree", 0, 0, 3),
        (FileStatus::Deleted, "one\ntwo", "", 0, 4, 4),
        (FileStatus::Modified, "one\ntwo", "three\nfour", 2, 1, 4),
        (FileStatus::Renamed, "one\ntwo", "three\nfour", 1, 1, 4),
        (FileStatus::Unknown, "one", "two", 0, 0, 12),
    ];

    for (index, (status, old, new, insertions, deletions, expected_lines)) in
        cases.into_iter().enumerate()
    {
        let mut app = app();
        let file = meta(&format!("src/file_{index}.rs"), status, insertions, deletions);
        app.expanded_files_set.insert(file.path.clone());
        app.git_diff_contents.insert(file.path.clone(), (old.to_string(), new.to_string()));

        assert_eq!(
            super::estimate_git_file_card_height(&app.input(), &file),
            super::GIT_FILE_HEADER_ESTIMATED_HEIGHT
                + expected_lines as f32 * super::GIT_FILE_LINE_ESTIMATED_HEIGHT
                + 12.0,
            "unexpected estimate for {status:?}"
        );
    }
}

#[test]
fn modified_empty_contents_still_reserve_one_diff_line() {
    let mut app = app();
    let file = meta("empty.txt", FileStatus::Modified, 0, 0);
    app.expanded_files_set.insert(file.path.clone());
    app.git_diff_contents.insert(file.path.clone(), (String::new(), String::new()));

    assert_eq!(
        super::estimate_git_file_card_height(&app.input(), &file),
        super::GIT_FILE_HEADER_ESTIMATED_HEIGHT + super::GIT_FILE_LINE_ESTIMATED_HEIGHT + 12.0
    );
}

#[test]
fn virtual_window_has_exact_buffered_bounds_and_spacers() {
    let mut app = app();
    let heights = vec![10.0; 40];
    app.git_diff_scroll_viewport_h = 10.0;

    app.git_diff_scroll_offset_y = 0.0;
    assert_eq!(super::compute_git_file_virtual_window(&app.input(), &heights), (0, 13, 0.0, 530.0));

    app.git_diff_scroll_offset_y = 0.5;
    assert_eq!(super::compute_git_file_virtual_window(&app.input(), &heights), (7, 33, 130.0, 130.0));

    app.git_diff_scroll_offset_y = 1.0;
    assert_eq!(super::compute_git_file_virtual_window(&app.input(), &heights), (27, 40, 530.0, 0.0));
}

#[test]
fn virtual_window_disables_clipping_for_non_positive_or_oversized_viewports() {
    let mut app = app();
    let heights = vec![10.0, 20.0, 30.0];

    app.git_diff_scroll_viewport_h = -1.0;
    assert_eq!(super::compute_git_file_virtual_window(&app.input(), &heights), (0, 3, 0.0, 0.0));

    app.git_diff_scroll_viewport_h = 1_000.0;
    app.git_diff_scroll_offset_y = 1.0;
    assert_eq!(super::compute_git_file_virtual_window(&app.input(), &heights), (0, 3, 0.0, 0.0));
}
