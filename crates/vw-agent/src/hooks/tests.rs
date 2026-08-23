// 验证公开钩子类型可从统一入口导出并满足跨线程传递约束。
use super::*;

#[test]
fn public_hook_types_are_exported() {
    fn assert_send<T: Send>() {}
    assert_send::<HookRunner>();
}
