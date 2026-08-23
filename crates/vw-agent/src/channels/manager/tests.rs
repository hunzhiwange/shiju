#[test]
fn test_module_is_linked() {
    let module = module_path!();
    assert!(module.contains("tests"));
}
// 本文件确认渠道管理器测试模块已正确挂载。
