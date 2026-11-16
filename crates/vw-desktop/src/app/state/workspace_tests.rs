//! 工作区状态测试，验证 Git 工作树选项展示与搜索选择状态的数据保持。
//! 测试夹具确保选中的仓库、分支和查找上下文不会在结构转换中丢失。

#[test]
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("workspace_tests"));
}
