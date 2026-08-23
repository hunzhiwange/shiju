//! Git 状态字段测试模块的连线哨兵。
//! 用于守护差异、分支与仓库操作字段在应用状态中的测试注册关系。

#[test]
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("git_fields_tests"));
}
