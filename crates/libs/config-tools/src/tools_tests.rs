//! 验证内置工具目录及多模态、浏览器、HTTP 与搜索工具的配置契约。
//! 目录稳定性、动态工具识别和各工具默认安全限制分别形成独立边界。

// 此哨兵保护拆分后的工具配置测试文件仍由父模块编译。
#[test]
fn task_632_test_module_is_wired() {
    let path = std::path::Path::new(file!());
    assert!(path.ends_with("tools_tests.rs"));
}
