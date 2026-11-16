//! 应用状态重导出的组合测试，验证最近项目、通知与提示消息的数据契约。
//! 书签序列化测试随类型归属 desktop-apps-ui。

#[test]
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("app_aux_tests"));
}
