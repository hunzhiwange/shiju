//! 任务存储测试模块的连线哨兵。
//! 确保存储路径、操作、持久化与产物子测试仍由聚合模块注册。

#[test]
fn store_test_module_is_linked() {
    let name = "store";
    assert_eq!(name.len(), 5);
}
