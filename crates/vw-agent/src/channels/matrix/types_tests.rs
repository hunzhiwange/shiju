// 本文件确认 Matrix 类型测试模块已正确挂载。

#[test]
// 哨兵确认条件编译下 Matrix 类型测试仍在测试树中。
fn test_module_is_linked() {
    let module = module_path!();
    assert!(module.contains("tests"));
}
