//! model_tests.rs 测试模块。
//!
//! 这些测试固定相邻解析器、视图辅助函数或状态计算的行为，防止后续 UI 重排时破坏边界契约。

use super::model::{FileTreeNode, build_file_tree_model, build_file_tree_subtree};

/// 构建 file tree model groups files by directory 对应的 Iced 界面片段或中间数据。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 无返回值时，函数通过发布消息或更新局部状态完成交互。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
#[test]
fn build_file_tree_model_groups_files_by_directory() {
    let files = vec![
        "/tmp/demo/src/main.rs".to_string(),
        "/tmp/demo/src/lib.rs".to_string(),
        "/tmp/demo/README.md".to_string(),
    ];

    let tree = build_file_tree_model("/tmp/demo", &files);

    assert!(tree.files.contains(&"README.md".to_string()));
    let src = tree.children.get("src").expect("src directory exists");
    assert!(src.files.contains(&"src/main.rs".to_string()));
    assert!(src.files.contains(&"src/lib.rs".to_string()));
}

/// 构建 file tree model accepts relative paths 对应的 Iced 界面片段或中间数据。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 无返回值时，函数通过发布消息或更新局部状态完成交互。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
#[test]
fn build_file_tree_model_accepts_relative_paths() {
    let files = vec!["src/main.rs".to_string(), "src/lib.rs".to_string(), "README.md".to_string()];

    let tree = build_file_tree_model("/tmp/demo", &files);

    assert!(tree.files.contains(&"README.md".to_string()));
    assert!(tree.children.contains_key("src"));
}

/// 构建 file tree model strips project name prefix 对应的 Iced 界面片段或中间数据。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 无返回值时，函数通过发布消息或更新局部状态完成交互。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
#[test]
fn build_file_tree_model_strips_project_name_prefix() {
    let files = vec![
        "demo/src/main.rs".to_string(),
        "demo/src/lib.rs".to_string(),
        "demo/README.md".to_string(),
    ];

    let tree = build_file_tree_model("/tmp/demo", &files);

    assert!(tree.files.contains(&"README.md".to_string()));
    assert!(tree.children.contains_key("src"));
}

/// 构建 file tree subtree loads only requested directory 对应的 Iced 界面片段或中间数据。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 无返回值时，函数通过发布消息或更新局部状态完成交互。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
#[test]
fn build_file_tree_subtree_loads_only_requested_directory() {
    let files = vec![
        "/tmp/demo/src/main.rs".to_string(),
        "/tmp/demo/src/lib.rs".to_string(),
        "/tmp/demo/src/nested/mod.rs".to_string(),
        "/tmp/demo/tests/basic.rs".to_string(),
    ];

    let tree = build_file_tree_subtree("/tmp/demo", &files, "src");

    assert!(tree.files.contains(&"src/main.rs".to_string()));
    assert!(tree.files.contains(&"src/lib.rs".to_string()));
    assert!(tree.children.contains_key("nested"));
    assert!(!tree.children.contains_key("tests"));
}

/// 验证 subtree returns nested directory without rebuilding paths 这一行为，确保对应解析或视图契约稳定。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 无返回值时，函数通过发布消息或更新局部状态完成交互。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
#[test]
fn subtree_returns_nested_directory_without_rebuilding_paths() {
    let files = vec![
        "/tmp/demo/src/nested/mod.rs".to_string(),
        "/tmp/demo/src/nested/deeper/lib.rs".to_string(),
    ];

    let nested = build_file_tree_subtree("/tmp/demo", &files, "src/nested");

    assert!(nested.files.contains(&"src/nested/mod.rs".to_string()));
    assert!(nested.children.contains_key("deeper"));
}

/// 验证 empty model reports no entries 这一行为，确保对应解析或视图契约稳定。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 无返回值时，函数通过发布消息或更新局部状态完成交互。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
#[test]
fn empty_model_reports_no_entries() {
    let tree = FileTreeNode::default();
    assert!(!tree.has_entries());
}

#[test]
fn build_file_tree_model_normalizes_dots_and_backslashes() {
    let files = vec![
        ".\\src\\main.rs".to_string(),
        "./src/lib.rs".to_string(),
        "/tmp/demo/./README.md".to_string(),
    ];

    let tree = build_file_tree_model("/tmp/demo", &files);

    assert!(tree.files.contains(&"README.md".to_string()));
    assert!(tree.children.contains_key("src"));
}

#[test]
fn build_file_tree_model_with_empty_root_normalizes_relative_paths() {
    let files = vec![" ./src/main.rs ".to_string(), "\\tests\\basic.rs".to_string()];

    let tree = build_file_tree_model("  ", &files);

    assert!(tree.children.contains_key("src"));
    assert!(tree.children.contains_key("tests"));
}

#[test]
fn build_file_tree_model_ignores_empty_and_dot_paths() {
    let files = vec![String::new(), "  ".to_string(), ".".to_string(), "/".to_string()];

    let tree = build_file_tree_model("/tmp/demo", &files);

    assert!(!tree.has_entries());
}

#[test]
fn build_file_tree_subtree_returns_root_for_empty_prefix() {
    let files = vec!["/tmp/demo/src/main.rs".to_string(), "/tmp/demo/README.md".to_string()];

    let tree = build_file_tree_subtree("/tmp/demo", &files, "");

    assert!(tree.has_entries());
    assert!(tree.files.contains(&"README.md".to_string()));
    assert!(tree.children.contains_key("src"));
}

#[test]
fn build_file_tree_subtree_returns_empty_for_missing_directory() {
    let files = vec!["/tmp/demo/src/main.rs".to_string()];

    let tree = build_file_tree_subtree("/tmp/demo", &files, "missing/nested");

    assert!(!tree.has_entries());
}
