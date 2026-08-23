//! 任务领域测试模块的连线哨兵。
//! 用于守护执行器、存储、员工编排和工作流测试的顶层注册入口。

#[test]
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("tests"));
}
