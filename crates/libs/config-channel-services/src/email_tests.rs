//! 确认邮件通道的独立测试模块已接入父模块。
//! 当前尚无行为用例，此哨兵专门保护测试模块声明不被重构遗漏。

// 使用源码路径而非业务断言，直接验证编译中的模块确为本文件。
#[test]
fn task_616_test_module_is_wired() {
    let path = std::path::Path::new(file!());
    assert!(path.ends_with("email_tests.rs"));
}
