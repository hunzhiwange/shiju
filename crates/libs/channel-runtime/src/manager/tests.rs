// 本文件确认渠道管理器测试模块已正确挂载。

#[test]
// 哨兵用例防止拆分测试文件后遗漏模块挂载。
fn test_module_is_linked() {
    let module = module_path!();
    assert!(module.contains("tests"));
}
