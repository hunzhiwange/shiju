// 本文件确认渠道运行时命令测试模块已正确挂载。

#[test]
// 哨兵防止运行时命令子测试在模块拆分后被遗漏。
fn test_module_is_linked() {
    let module = module_path!();
    assert!(module.contains("tests"));
}
