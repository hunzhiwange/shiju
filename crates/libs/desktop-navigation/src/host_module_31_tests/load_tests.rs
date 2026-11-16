//! 启动回填装配的测试注册哨兵；旧同步加载器已不再参与首屏初始化。

#[test]
fn load_tests_module_is_wired() {
    assert!(module_path!().ends_with("load_tests"));
}
