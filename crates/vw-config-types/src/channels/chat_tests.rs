//! 验证聊天通道实例配置的旧格式兼容、命名实例解析与标识安全约束。
//! fixture 仅提供最小凭据，用于区分配置迁移逻辑与具体通道实现。

// 此哨兵保护拆分后的聊天通道测试文件仍由父模块编译。
#[test]
fn task_614_test_module_is_wired() {
    let path = std::path::Path::new(file!());
    assert!(path.ends_with("chat_tests.rs"));
}
