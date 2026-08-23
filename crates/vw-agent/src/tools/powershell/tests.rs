//! 验证 PowerShell 测试模块已正确接入模块树。
//! 此烟雾测试只约束编译期连线，不宣称支持尚未覆盖的执行行为。

#[test]
fn module_is_linked_for_plan6_task() {
    let marker = module_path!();
    assert!(marker.contains("powershell::tests"));
}
