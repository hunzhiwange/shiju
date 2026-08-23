//! 验证兼容供应商模块通过公共命名空间重新导出。

#[test]
// 通过引用兼容路径完成编译期检查，防止重构意外移除旧入口。
fn provider_compat_modules_are_reexported() {
    let _ = std::any::type_name::<super::auth::Method>();
}
