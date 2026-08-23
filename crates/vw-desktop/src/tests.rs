//! 桌面库根测试模块的最小锚点，用于确认测试连线能够被正常编译。
//! 行为性覆盖由各业务模块的拆分测试承担。
#[test]
fn module_test_anchor() {
    let stable_value = 1 + 1;
    assert_eq!(stable_value, 2);
}
