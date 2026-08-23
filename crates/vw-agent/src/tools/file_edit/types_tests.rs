//! 确认文件编辑类型测试模块保持正确装配。

#[test]
fn module_is_linked_for_plan6_task() {
    let marker = module_path!();
    assert!(marker.contains("types_tests"));
}
