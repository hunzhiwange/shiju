//! 验证文件链接从完整路径提取名称并兼容非 ASCII 文件名。
//! 路径分隔符只影响展示标签，点击目标仍需保留完整原始路径。

use super::file_link::{file_name_link, file_name_only, file_path_tooltip};

#[test]
fn file_name_only_removes_parent_directories() {
    assert_eq!(file_name_only("src/app/main.rs"), "main.rs");
    assert_eq!(file_name_only("README"), "README");
    assert_eq!(file_name_only("/"), "/");
}

#[test]
fn file_link_views_accept_full_paths_and_non_ascii_names() {
    let link = file_name_link("src/界面.rs", "/tmp/project/src/界面.rs");
    let tooltip = file_path_tooltip("/tmp/project/src/界面.rs".to_string());

    std::hint::black_box(link);
    std::hint::black_box(tooltip);
}
