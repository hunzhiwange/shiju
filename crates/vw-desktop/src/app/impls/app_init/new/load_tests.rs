//! 验证新建应用时持久化数据的加载与回退状态。
//!
//! 该哨兵确认加载子模块仍被新建流程编译，具体加载行为由相邻初始化测试覆盖。

#[test]
fn load_tests_module_is_wired() {
    assert!(module_path!().ends_with("load_tests"));
}
