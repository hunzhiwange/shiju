//! 验证 worktree 顶层模块稳定地重导出公共类型。
//! 烟雾用例通过实际构造类型确认调用者无需依赖内部文件布局。

#[test]
fn top_level_reexports_worktree_types() {
    let info = super::Info {
        name: "demo".to_string(),
        branch: "vibewindow/demo".to_string(),
        directory: "/tmp/demo".to_string(),
    };

    assert_eq!(info.name, "demo");
}
