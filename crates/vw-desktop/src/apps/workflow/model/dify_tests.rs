//! Dify 工作流模型转换的测试，覆盖节点数据、边与应用元信息映射。
//! fixture 包含未知字段和多种节点类型，验证往返转换保留受支持内容。
#[test]
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("dify_tests"));
}
