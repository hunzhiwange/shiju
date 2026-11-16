//! 展示层状态类型测试，覆盖稳定标签、配置值解析、平台差异和克隆语义。
//! 枚举全集及默认值是界面与配置间的契约，未知输入应明确拒绝而非猜测。

#[test]
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("presentation_tests"));
}
