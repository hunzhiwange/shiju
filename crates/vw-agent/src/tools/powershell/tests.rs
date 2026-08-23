//! 验证 PowerShell 测试模块已正确接入模块树。

#[test]
fn module_is_linked_for_plan6_task() {
    let marker = module_path!();
    assert!(marker.contains("powershell::tests"));
}
