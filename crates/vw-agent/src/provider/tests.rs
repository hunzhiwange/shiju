//! 验证兼容供应商模块通过公共命名空间重新导出。

#[test]
fn provider_compat_modules_are_reexported() {
    let _ = std::any::type_name::<super::auth::Method>();
}
