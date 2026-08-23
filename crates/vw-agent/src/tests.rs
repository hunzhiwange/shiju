//! 验证 vw-agent 根模块的公共导出和基础运行契约。
//! 该冒烟断言确保公共配置类型仍从 crate 根导出。

#[test]
fn public_config_type_is_reexported() {
    let _ = std::any::type_name::<crate::Config>();
}
