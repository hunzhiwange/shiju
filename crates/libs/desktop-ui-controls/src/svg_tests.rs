//! 验证 SVG 组件测试模块的装配入口。
//! 具体路径解析由专用测试覆盖，此处只保证组件模块参与测试构建。

#[test]
fn svg_test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("svg_tests"));
}
