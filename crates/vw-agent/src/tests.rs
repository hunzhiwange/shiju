//! 验证 vw-agent 根模块的公共导出和基础运行契约。

#[test]
fn public_config_type_is_reexported() {
    let _ = std::any::type_name::<crate::Config>();
}
